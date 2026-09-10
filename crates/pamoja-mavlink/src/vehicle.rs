//! A MAVLink vehicle modeled as a pamoja [`Device`].
//!
//! [`Vehicle`] wraps a [`Connection`] over any [`ByteLink`] and presents an autopilot through
//! the core device traits, so a PX4 or ArduPilot vehicle drives like any other pamoja device
//! from any language binding. It maps the three surfaces a ground station needs onto the
//! device model:
//!
//! - [`Device`] - [`connect`](Device::connect) waits for the vehicle's first heartbeat (and
//!   learns its system and component id), and the vehicle's stable [`id`](Device::id) is its
//!   MAVLink address.
//! - [`Telemetry`] - [`next_frame`](Telemetry::next_frame) yields the next decoded telemetry
//!   [`Report`].
//! - [`Actuator`] - [`apply`](Actuator::apply) streams an offboard [`Setpoint`].
//!
//! On top of those it offers the mission and command surfaces as async methods that drive the
//! sans-IO [`protocol`](crate::protocol) machines over the link, applying the mission
//! protocol's timeout-and-retransmit rules: [`upload_mission`](Vehicle::upload_mission) and
//! [`download_mission`](Vehicle::download_mission) run the plan transfer, and
//! [`send_command`](Vehicle::send_command) (with [`arm`](Vehicle::arm),
//! [`set_mode`](Vehicle::set_mode), [`takeoff`](Vehicle::takeoff), and friends) run the command
//! protocol.
//!
//! This layer is available with the default `std` feature; the wire core and the protocol
//! machines below it are `no_std`.

use std::time::Duration;

use pamoja_core::{Actuator, Device, Error as CoreError, Result as CoreResult, Telemetry};
use tokio::time::timeout;

use crate::dialect::{
    mav_autopilot, mav_cmd, mav_mission_result, mav_mission_type, mav_state, mav_type, Attitude,
    BatteryStatus, CommandAck, CommandLong, GlobalPositionInt, GpsRawInt, Heartbeat, Message,
    MissionAck, MissionCount, MissionItemInt, MissionRequest, MissionRequestInt,
    SetPositionTargetGlobalInt, SetPositionTargetLocalNed, Statustext, SysStatus, VfrHud,
};
use crate::frame::Frame;
use crate::link::{ByteLink, Connection};
use crate::protocol::command::{AckOutcome, CommandProtocol};
use crate::protocol::mission::{MissionReceiver, MissionSender, ReceiverAction};
use crate::protocol::MAX_RETRIES;
use crate::signing::{Signer, Verifier};
use crate::MavlinkError;

/// How long to wait for a response before retransmitting, as the mission and command protocols
/// recommend for their request/response messages.
const RESPONSE_TIMEOUT: Duration = Duration::from_millis(1500);

/// How long [`connect`](Vehicle::connect) waits for the vehicle's first heartbeat.
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(5);

/// The conventional ground-station component id.
pub const GCS_COMPONENT: u8 = 190;

/// A decoded telemetry report from a [`Vehicle`].
///
/// The common messages a ground station displays are decoded into typed variants; anything
/// else is carried as a [`Report::Other`] raw frame so no traffic is lost.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Report {
    /// The periodic heartbeat announcing the vehicle's type, autopilot, and status.
    Heartbeat(Heartbeat),
    /// Onboard sensor health, load, and battery state.
    SysStatus(SysStatus),
    /// The raw GPS fix.
    GpsRawInt(GpsRawInt),
    /// Orientation and angular rates.
    Attitude(Attitude),
    /// The fused global position, altitude, and velocity.
    GlobalPositionInt(GlobalPositionInt),
    /// The heads-up flight summary.
    VfrHud(VfrHud),
    /// Battery charge, current, and per-cell voltages.
    BatteryStatus(BatteryStatus),
    /// A human-readable status message.
    Statustext(Statustext),
    /// Any other message, carried as the raw frame. Boxed because a [`Frame`] is far larger
    /// than a decoded message, so the common typed reports stay small to move around.
    Other(Box<Frame>),
}

impl Report {
    /// Decodes a frame into a typed report, falling back to [`Report::Other`].
    ///
    /// # Arguments
    ///
    /// * `frame` - the received frame.
    ///
    /// # Returns
    ///
    /// The decoded report.
    fn from_frame(frame: &Frame) -> Report {
        // A telemetry decode of a well-formed frame does not fail (a short payload is
        // zero-extended), so a decode error falls back to the raw frame rather than dropping it.
        // `unwrap_or_else` keeps the success path from boxing the frame it does not need.
        let raw = || Report::Other(Box::new(*frame));
        match frame.message_id() {
            Heartbeat::ID => Heartbeat::decode(frame.payload())
                .map(Report::Heartbeat)
                .unwrap_or_else(|_| raw()),
            SysStatus::ID => SysStatus::decode(frame.payload())
                .map(Report::SysStatus)
                .unwrap_or_else(|_| raw()),
            GpsRawInt::ID => GpsRawInt::decode(frame.payload())
                .map(Report::GpsRawInt)
                .unwrap_or_else(|_| raw()),
            Attitude::ID => Attitude::decode(frame.payload())
                .map(Report::Attitude)
                .unwrap_or_else(|_| raw()),
            GlobalPositionInt::ID => GlobalPositionInt::decode(frame.payload())
                .map(Report::GlobalPositionInt)
                .unwrap_or_else(|_| raw()),
            VfrHud::ID => VfrHud::decode(frame.payload())
                .map(Report::VfrHud)
                .unwrap_or_else(|_| raw()),
            BatteryStatus::ID => BatteryStatus::decode(frame.payload())
                .map(Report::BatteryStatus)
                .unwrap_or_else(|_| raw()),
            Statustext::ID => Statustext::decode(frame.payload())
                .map(Report::Statustext)
                .unwrap_or_else(|_| raw()),
            _ => raw(),
        }
    }
}

/// An offboard control setpoint, in the local or global frame.
///
/// Build one with the constructors on [`SetPositionTargetLocalNed`] and
/// [`SetPositionTargetGlobalInt`] (in [`protocol::offboard`](crate::protocol::offboard)), then
/// stream it with [`Actuator::apply`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Setpoint {
    /// A setpoint in the local NED frame.
    Local(SetPositionTargetLocalNed),
    /// A setpoint in the global frame.
    Global(SetPositionTargetGlobalInt),
}

/// A MAVLink vehicle over a [`ByteLink`], exposed through the pamoja device model.
///
/// A vehicle sends as a ground station (its own system and component id) and addresses a target
/// vehicle. The target is learned from the first heartbeat unless it is pinned with
/// [`with_target`](Vehicle::with_target). Attach signing with
/// [`with_signer`](Vehicle::with_signer) and [`with_verifier`](Vehicle::with_verifier).
pub struct Vehicle<L> {
    connection: Connection<L>,
    id: String,
    target_system: u8,
    target_component: u8,
    autodetect_target: bool,
}

impl<L: ByteLink> Vehicle<L> {
    /// Creates a vehicle client sending as the given ground-station identity.
    ///
    /// The target vehicle defaults to system 1, component 1, and is updated from the first
    /// heartbeat seen during [`connect`](Device::connect).
    ///
    /// # Arguments
    ///
    /// * `link` - the byte link to the vehicle.
    /// * `system_id` - this ground station's system id.
    /// * `component_id` - this ground station's component id.
    ///
    /// # Returns
    ///
    /// The vehicle client, with signing off.
    pub fn new(link: L, system_id: u8, component_id: u8) -> Self {
        Vehicle {
            connection: Connection::new(link, system_id, component_id),
            id: Self::format_id(1, 1),
            target_system: 1,
            target_component: 1,
            autodetect_target: true,
        }
    }

    /// Pins the target vehicle's system and component id instead of learning them.
    ///
    /// # Arguments
    ///
    /// * `system_id` - the target vehicle's system id.
    /// * `component_id` - the target vehicle's component id.
    ///
    /// # Returns
    ///
    /// The vehicle, for chaining.
    pub fn with_target(mut self, system_id: u8, component_id: u8) -> Self {
        self.target_system = system_id;
        self.target_component = component_id;
        self.autodetect_target = false;
        self.id = Self::format_id(system_id, component_id);
        self
    }

    /// Signs every outgoing frame with `signer`.
    ///
    /// # Arguments
    ///
    /// * `signer` - the signer to stamp outgoing frames with.
    ///
    /// # Returns
    ///
    /// The vehicle, for chaining.
    pub fn with_signer(mut self, signer: Signer) -> Self {
        self.connection = self.connection.with_signer(signer);
        self
    }

    /// Verifies signed incoming frames with `verifier`, and rejects unsigned ones.
    ///
    /// # Arguments
    ///
    /// * `verifier` - the verifier to check incoming signed frames with.
    ///
    /// # Returns
    ///
    /// The vehicle, for chaining.
    pub fn with_verifier(mut self, verifier: Verifier) -> Self {
        self.connection = self.connection.with_verifier(verifier);
        self
    }

    /// Returns the target vehicle's system id.
    ///
    /// # Returns
    ///
    /// The target system id.
    pub fn target_system(&self) -> u8 {
        self.target_system
    }

    /// Returns the target vehicle's component id.
    ///
    /// # Returns
    ///
    /// The target component id.
    pub fn target_component(&self) -> u8 {
        self.target_component
    }

    /// Sends a ground-station heartbeat, which some autopilots require before they accept
    /// commands.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the heartbeat has been sent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`](pamoja_core::Error::Transport) if the frame cannot be sent.
    pub async fn send_heartbeat(&mut self) -> CoreResult<()> {
        let heartbeat = Heartbeat {
            custom_mode: 0,
            type_: mav_type::GCS,
            autopilot: mav_autopilot::INVALID,
            base_mode: 0,
            system_status: mav_state::ACTIVE,
            mavlink_version: 3,
        };
        self.tx(&heartbeat).await
    }

    /// Reads the next telemetry report from the vehicle.
    ///
    /// Unlike [`Telemetry::next_frame`], this treats a closed link as an error rather than an
    /// end of stream.
    ///
    /// # Returns
    ///
    /// The next decoded [`Report`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`](pamoja_core::Error::Closed) if the link ends, or
    /// [`Error::Transport`](pamoja_core::Error::Transport) on a link fault.
    pub async fn recv(&mut self) -> CoreResult<Report> {
        let frame = self.rx().await?;
        Ok(Report::from_frame(&frame))
    }

    /// Sends a command to the vehicle and awaits its result, retransmitting on timeout.
    ///
    /// The command is sent as a `COMMAND_LONG` and matched to its `COMMAND_ACK`. An
    /// in-progress acknowledgment extends the wait; a missing acknowledgment resends the
    /// command with an incremented confirmation, up to the retry budget.
    ///
    /// # Arguments
    ///
    /// * `command` - the [`MAV_CMD`](crate::dialect::mav_cmd) id.
    /// * `params` - the seven command parameters.
    ///
    /// # Returns
    ///
    /// The [`MAV_RESULT`](crate::dialect::mav_result) the vehicle reported, including a
    /// rejection such as [`DENIED`](crate::dialect::mav_result::DENIED).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`](pamoja_core::Error::Transport) if the command is not
    /// acknowledged within the retry budget or the link faults.
    pub async fn send_command(&mut self, command: u16, params: [f32; 7]) -> CoreResult<u8> {
        let mut protocol = CommandProtocol::new(command, MAX_RETRIES);
        loop {
            let request = CommandLong {
                param1: params[0],
                param2: params[1],
                param3: params[2],
                param4: params[3],
                param5: params[4],
                param6: params[5],
                param7: params[6],
                command,
                target_system: self.target_system,
                target_component: self.target_component,
                confirmation: protocol.confirmation(),
            };
            self.tx(&request).await?;

            // Wait for the matching acknowledgment, ignoring unrelated traffic; on a timeout
            // fall out to resend, and on an exhausted budget give up.
            let resend = loop {
                match timeout(RESPONSE_TIMEOUT, self.rx()).await {
                    Err(_elapsed) => {
                        if protocol.on_timeout().is_none() {
                            return Err(CoreError::Transport(
                                "command was not acknowledged".into(),
                            ));
                        }
                        break true;
                    }
                    Ok(frame) => {
                        let frame = frame?;
                        if frame.message_id() == CommandAck::ID {
                            let ack = CommandAck::decode(frame.payload()).map_err(map_mav)?;
                            match protocol.on_ack(&ack) {
                                AckOutcome::Final(result) => return Ok(result),
                                AckOutcome::InProgress(_) | AckOutcome::Unrelated => continue,
                            }
                        }
                    }
                }
            };
            debug_assert!(resend);
        }
    }

    /// Arms or disarms the vehicle.
    ///
    /// # Arguments
    ///
    /// * `arm` - `true` to arm, `false` to disarm.
    ///
    /// # Returns
    ///
    /// The [`MAV_RESULT`](crate::dialect::mav_result) of the arm command.
    ///
    /// # Errors
    ///
    /// As [`send_command`](Vehicle::send_command).
    pub async fn arm(&mut self, arm: bool) -> CoreResult<u8> {
        let flag = if arm { 1.0 } else { 0.0 };
        self.send_command(
            mav_cmd::COMPONENT_ARM_DISARM,
            [flag, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        )
        .await
    }

    /// Requests a mode change.
    ///
    /// # Arguments
    ///
    /// * `base_mode` - the [`MAV_MODE_FLAG`](crate::dialect::mav_mode_flag) base-mode bits.
    /// * `custom_mode` - the autopilot-specific custom mode.
    ///
    /// # Returns
    ///
    /// The [`MAV_RESULT`](crate::dialect::mav_result) of the mode command.
    ///
    /// # Errors
    ///
    /// As [`send_command`](Vehicle::send_command).
    pub async fn set_mode(&mut self, base_mode: u8, custom_mode: u32) -> CoreResult<u8> {
        self.send_command(
            mav_cmd::DO_SET_MODE,
            [
                base_mode as f32,
                custom_mode as f32,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
        )
        .await
    }

    /// Commands a takeoff to an altitude.
    ///
    /// # Arguments
    ///
    /// * `altitude` - the target altitude, in meters.
    ///
    /// # Returns
    ///
    /// The [`MAV_RESULT`](crate::dialect::mav_result) of the takeoff command.
    ///
    /// # Errors
    ///
    /// As [`send_command`](Vehicle::send_command).
    pub async fn takeoff(&mut self, altitude: f32) -> CoreResult<u8> {
        self.send_command(
            mav_cmd::NAV_TAKEOFF,
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, altitude],
        )
        .await
    }

    /// Asks the vehicle to emit one message, by id.
    ///
    /// # Arguments
    ///
    /// * `message_id` - the id of the message to request.
    ///
    /// # Returns
    ///
    /// The [`MAV_RESULT`](crate::dialect::mav_result) of the request.
    ///
    /// # Errors
    ///
    /// As [`send_command`](Vehicle::send_command).
    pub async fn request_message(&mut self, message_id: u32) -> CoreResult<u8> {
        self.send_command(
            mav_cmd::REQUEST_MESSAGE,
            [message_id as f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        )
        .await
    }

    /// Sets how often the vehicle streams a message.
    ///
    /// # Arguments
    ///
    /// * `message_id` - the id of the message.
    /// * `interval_us` - the send interval, in microseconds, or `-1` to disable.
    ///
    /// # Returns
    ///
    /// The [`MAV_RESULT`](crate::dialect::mav_result) of the request.
    ///
    /// # Errors
    ///
    /// As [`send_command`](Vehicle::send_command).
    pub async fn set_message_interval(
        &mut self,
        message_id: u32,
        interval_us: i32,
    ) -> CoreResult<u8> {
        self.send_command(
            mav_cmd::SET_MESSAGE_INTERVAL,
            [
                message_id as f32,
                interval_us as f32,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
        )
        .await
    }

    /// Uploads a mission plan to the vehicle.
    ///
    /// Runs the mission protocol's sender role: announces the count, answers each item request,
    /// and completes on the vehicle's acknowledgment, retransmitting on timeout.
    ///
    /// # Arguments
    ///
    /// * `items` - the mission items, in sequence order; the target ids, sequence numbers, and
    ///   mission type are stamped on for you.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the vehicle accepts the plan.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`](pamoja_core::Error::Transport) if the transfer times out or
    /// the vehicle rejects the plan.
    pub async fn upload_mission(&mut self, items: &[MissionItemInt]) -> CoreResult<()> {
        let sender = MissionSender::new(
            items,
            self.target_system,
            self.target_component,
            mav_mission_type::MISSION,
        );
        self.tx(&sender.count()).await?;

        // `None` means the opening count is still unanswered and is what a timeout resends;
        // `Some(seq)` is the last item sent, resent on a timeout.
        let mut last_seq: Option<u16> = None;
        let mut retries = MAX_RETRIES;
        loop {
            match timeout(RESPONSE_TIMEOUT, self.rx()).await {
                Err(_elapsed) => {
                    if retries == 0 {
                        return Err(CoreError::Transport("mission upload timed out".into()));
                    }
                    retries -= 1;
                    match last_seq {
                        None => self.tx(&sender.count()).await?,
                        Some(seq) => {
                            if let Some(item) = sender.item(seq) {
                                self.tx(&item).await?;
                            }
                        }
                    }
                }
                Ok(frame) => {
                    let frame = frame?;
                    match frame.message_id() {
                        MissionRequestInt::ID => {
                            let request =
                                MissionRequestInt::decode(frame.payload()).map_err(map_mav)?;
                            self.answer_item(&sender, request.seq, &mut last_seq, &mut retries)
                                .await?;
                        }
                        MissionRequest::ID => {
                            let request =
                                MissionRequest::decode(frame.payload()).map_err(map_mav)?;
                            self.answer_item(&sender, request.seq, &mut last_seq, &mut retries)
                                .await?;
                        }
                        MissionAck::ID => {
                            let ack = MissionAck::decode(frame.payload()).map_err(map_mav)?;
                            if ack.type_ == mav_mission_result::ACCEPTED {
                                return Ok(());
                            }
                            return Err(CoreError::Transport(format!(
                                "vehicle rejected the mission: result {}",
                                ack.type_
                            )));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Downloads the vehicle's mission plan.
    ///
    /// Runs the mission protocol's receiver role: requests the count, requests each item in
    /// order, re-requests an out-of-order item, and acknowledges completion, retransmitting on
    /// timeout.
    ///
    /// # Returns
    ///
    /// The mission items, in sequence order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`](pamoja_core::Error::Transport) if the transfer times out or
    /// the link faults.
    pub async fn download_mission(&mut self) -> CoreResult<Vec<MissionItemInt>> {
        let mut receiver = MissionReceiver::new(
            self.target_system,
            self.target_component,
            mav_mission_type::MISSION,
        );
        self.tx(&receiver.request_list()).await?;

        let mut items: Vec<MissionItemInt> = Vec::new();
        let mut last_request: Option<MissionRequestInt> = None;
        let mut got_count = false;
        let mut retries = MAX_RETRIES;
        loop {
            match timeout(RESPONSE_TIMEOUT, self.rx()).await {
                Err(_elapsed) => {
                    if retries == 0 {
                        return Err(CoreError::Transport("mission download timed out".into()));
                    }
                    retries -= 1;
                    match &last_request {
                        Some(request) => self.tx(request).await?,
                        None => self.tx(&receiver.request_list()).await?,
                    }
                }
                Ok(frame) => {
                    let frame = frame?;
                    match frame.message_id() {
                        MissionCount::ID if !got_count => {
                            let count = MissionCount::decode(frame.payload())
                                .map_err(map_mav)?
                                .count;
                            got_count = true;
                            items.reserve(count as usize);
                            match receiver.on_count(count) {
                                ReceiverAction::Request(request) => {
                                    self.tx(&request).await?;
                                    last_request = Some(request);
                                    retries = MAX_RETRIES;
                                }
                                ReceiverAction::Ack(ack) => {
                                    self.tx(&ack).await?;
                                    return Ok(items);
                                }
                            }
                        }
                        MissionItemInt::ID => {
                            let item = MissionItemInt::decode(frame.payload()).map_err(map_mav)?;
                            let (accepted, action) = receiver.on_item(&item);
                            if let Some(item) = accepted {
                                items.push(item);
                            }
                            match action {
                                ReceiverAction::Request(request) => {
                                    self.tx(&request).await?;
                                    last_request = Some(request);
                                    retries = MAX_RETRIES;
                                }
                                ReceiverAction::Ack(ack) => {
                                    self.tx(&ack).await?;
                                    return Ok(items);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Answers a mission item request during an upload, recording it as the item to resend on a
    // timeout and refilling the retry budget.
    async fn answer_item(
        &mut self,
        sender: &MissionSender<'_>,
        seq: u16,
        last_seq: &mut Option<u16>,
        retries: &mut u8,
    ) -> CoreResult<()> {
        if let Some(item) = sender.item(seq) {
            self.tx(&item).await?;
            *last_seq = Some(seq);
            *retries = MAX_RETRIES;
        }
        Ok(())
    }

    // Waits for the first heartbeat, learning the target ids when auto-detecting.
    async fn wait_for_heartbeat(&mut self) -> CoreResult<()> {
        loop {
            let frame = timeout(HEARTBEAT_TIMEOUT, self.rx())
                .await
                .map_err(|_| CoreError::Transport("no heartbeat from the vehicle".into()))??;
            if frame.message_id() == Heartbeat::ID {
                if self.autodetect_target {
                    self.target_system = frame.system_id();
                    self.target_component = frame.component_id();
                    self.id = Self::format_id(self.target_system, self.target_component);
                }
                return Ok(());
            }
        }
    }

    async fn tx<M: Message>(&mut self, message: &M) -> CoreResult<()> {
        self.connection.send(message).await.map_err(map_mav)
    }

    async fn rx(&mut self) -> CoreResult<Frame> {
        self.connection.recv().await.map_err(map_mav)
    }

    fn format_id(system_id: u8, component_id: u8) -> String {
        format!("mavlink:{system_id}.{component_id}")
    }
}

impl<L: ByteLink + Send> Device for Vehicle<L> {
    fn id(&self) -> &str {
        &self.id
    }

    async fn connect(&mut self) -> CoreResult<()> {
        self.wait_for_heartbeat().await
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        // The link is released when the vehicle is dropped; there is no teardown handshake.
        Ok(())
    }
}

impl<L: ByteLink + Send> Telemetry for Vehicle<L> {
    type Frame = Report;

    async fn next_frame(&mut self) -> CoreResult<Option<Report>> {
        match self.connection.recv().await {
            Ok(frame) => Ok(Some(Report::from_frame(&frame))),
            Err(MavlinkError::Closed) => Ok(None),
            Err(err) => Err(map_mav(err)),
        }
    }
}

impl<L: ByteLink + Send> Actuator for Vehicle<L> {
    type Command = Setpoint;

    async fn apply(&mut self, command: Setpoint) -> CoreResult<()> {
        match command {
            Setpoint::Local(setpoint) => self.tx(&setpoint).await,
            Setpoint::Global(setpoint) => self.tx(&setpoint).await,
        }
    }
}

// Maps a wire-layer fault onto the shared error model: a closed link is `Closed`, a bad payload
// is a codec fault, a signing failure is an auth fault, and the rest are transport faults.
fn map_mav(err: MavlinkError) -> CoreError {
    match err {
        MavlinkError::Closed => CoreError::Closed,
        MavlinkError::BadPayload => CoreError::Codec("malformed MAVLink payload".into()),
        MavlinkError::Unsigned | MavlinkError::BadSignature | MavlinkError::ReplayedTimestamp => {
            CoreError::Auth(err.to_string())
        }
        other => CoreError::Transport(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::{mav_frame, mav_result};
    use crate::link::{MemoryLink, SitlAutopilot};

    fn waypoint(seq: u16, lat: i32, lon: i32, alt: f32) -> MissionItemInt {
        MissionItemInt {
            param1: 0.0,
            param2: 0.0,
            param3: 0.0,
            param4: 0.0,
            x: lat,
            y: lon,
            z: alt,
            seq,
            command: mav_cmd::NAV_WAYPOINT,
            target_system: 0,
            target_component: 0,
            frame: mav_frame::GLOBAL_RELATIVE_ALT_INT,
            current: (seq == 0) as u8,
            autocontinue: 1,
            mission_type: mav_mission_type::MISSION,
        }
    }

    // Spawns a SITL autopilot that serves whatever the vehicle sends until the test drops it.
    fn spawn_autopilot(link: MemoryLink) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut autopilot = SitlAutopilot::new(link, 1, 1);
            let _ = autopilot.emit_heartbeat().await;
            loop {
                if autopilot.serve_once().await.is_err() {
                    break;
                }
            }
        })
    }

    #[tokio::test]
    async fn connect_learns_the_target_from_the_heartbeat() {
        let (gcs, vehicle) = MemoryLink::pair();
        let handle = spawn_autopilot(vehicle);
        let mut client = Vehicle::new(gcs, 255, GCS_COMPONENT);
        client.connect().await.unwrap();
        assert_eq!(client.target_system(), 1);
        assert_eq!(client.id(), "mavlink:1.1");
        handle.abort();
    }

    #[tokio::test]
    async fn a_command_is_acknowledged() {
        let (gcs, vehicle) = MemoryLink::pair();
        let handle = spawn_autopilot(vehicle);
        let mut client = Vehicle::new(gcs, 255, GCS_COMPONENT);
        client.connect().await.unwrap();
        let result = client.arm(true).await.unwrap();
        assert_eq!(result, mav_result::ACCEPTED);
        handle.abort();
    }

    #[tokio::test]
    async fn a_mission_uploads_and_downloads_unchanged() {
        let (gcs, vehicle) = MemoryLink::pair();
        let handle = spawn_autopilot(vehicle);
        let mut client = Vehicle::new(gcs, 255, GCS_COMPONENT);
        client.connect().await.unwrap();

        let plan = [
            waypoint(0, 473_977_418, 85_455_939, 10.0),
            waypoint(1, 473_977_500, 85_456_000, 20.0),
            waypoint(2, 473_977_600, 85_456_100, 15.0),
        ];
        client.upload_mission(&plan).await.unwrap();
        let downloaded = client.download_mission().await.unwrap();

        assert_eq!(downloaded.len(), 3);
        for (sent, got) in plan.iter().zip(downloaded.iter()) {
            assert_eq!(sent.x, got.x);
            assert_eq!(sent.y, got.y);
            assert_eq!(sent.z, got.z);
            assert_eq!(sent.command, got.command);
        }
        handle.abort();
    }

    #[tokio::test]
    async fn an_offboard_setpoint_is_accepted_by_the_actuator() {
        let (gcs, vehicle) = MemoryLink::pair();
        let handle = spawn_autopilot(vehicle);
        let mut client = Vehicle::new(gcs, 255, GCS_COMPONENT);
        client.connect().await.unwrap();
        let setpoint = Setpoint::Local(SetPositionTargetLocalNed::velocity(
            0,
            mav_frame::LOCAL_NED,
            client.target_system(),
            client.target_component(),
            0.5,
            0.0,
            -0.2,
        ));
        client.apply(setpoint).await.unwrap();
        handle.abort();
    }
}
