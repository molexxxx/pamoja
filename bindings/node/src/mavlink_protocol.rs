//! The MAVLink service protocols: mission transfer, commands, and offboard setpoints.
//!
//! A frame carries one message; a real exchange with an autopilot is a sequence of them with
//! rules about order, matching, and retransmission. The machines here hold those rules and
//! nothing else: no IO, no timers. A caller feeds each one the frames off its link and sends
//! back what the machine hands it, applying its own timing policy for timeouts.
//!
//! Every machine takes a whole frame and answers with a whole frame, so the payload decoding,
//! message-id dispatch, and reply encoding happen once, in the engine. A frame a machine does
//! not handle comes back as `null` rather than as an error, so one link's traffic can be
//! routed through several machines in turn.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use pamoja_mavlink::dialect::{
    encode_message, Message, MissionItemInt, SetPositionTargetGlobalInt, SetPositionTargetLocalNed,
};
use pamoja_mavlink::protocol::{
    AckOutcome as CoreAckOutcome, CommandProtocol as CoreCommand, MissionReceiver as CoreReceiver,
    MissionSender as CoreSender, ReceiverAction, SenderStep as CoreSenderStep, TypeMask,
    MAX_RETRIES,
};
use pamoja_mavlink::{Frame as CoreFrame, Header};

use crate::mavlink::{error_of, MavlinkFrame, MavlinkHeader};
use crate::mavlink_schema::MavlinkMessage;

/// The number of times a request is retransmitted before a transfer is abandoned, as the
/// mission protocol recommends.
#[napi]
pub const MAVLINK_MAX_RETRIES: u8 = MAX_RETRIES;

/// Use the position fields of a setpoint.
#[napi]
pub const MAVLINK_TYPEMASK_POSITION: u32 = 1 << 0;
/// Use the velocity fields of a setpoint.
#[napi]
pub const MAVLINK_TYPEMASK_VELOCITY: u32 = 1 << 1;
/// Use the acceleration fields of a setpoint.
#[napi]
pub const MAVLINK_TYPEMASK_ACCELERATION: u32 = 1 << 2;
/// Use the yaw field of a setpoint.
#[napi]
pub const MAVLINK_TYPEMASK_YAW: u32 = 1 << 3;
/// Use the yaw rate field of a setpoint.
#[napi]
pub const MAVLINK_TYPEMASK_YAW_RATE: u32 = 1 << 4;
/// Treat the acceleration fields as a force.
#[napi]
pub const MAVLINK_TYPEMASK_FORCE: u32 = 1 << 5;

/// What one incoming frame produced for a mission receiver.
#[napi]
pub struct ReceiverStep {
    kind: &'static str,
    accepted: Option<Vec<u8>>,
    reply: CoreFrame,
}

#[napi]
impl ReceiverStep {
    /// What the receiver answered with: `request` for the next item, or `ack` to end the
    /// transfer.
    #[napi(getter)]
    pub fn kind(&self) -> String {
        self.kind.to_owned()
    }

    /// The `MISSION_ITEM_INT` the frame carried, if it was the one expected next, as a
    /// message read by field name.
    #[napi(getter)]
    pub fn accepted(&self) -> Option<MavlinkMessage> {
        self.accepted
            .as_ref()
            .map(|payload| MavlinkMessage::from_typed(MissionItemInt::DESCRIPTOR, payload.clone()))
    }

    /// The frame to send back.
    #[napi(getter)]
    pub fn reply(&self) -> MavlinkFrame {
        MavlinkFrame::from_frame(self.reply)
    }
}

/// Requests a plan's items in order and collects them, ending with an acknowledgement.
#[napi]
pub struct MissionReceiver {
    inner: CoreReceiver,
}

#[napi]
impl MissionReceiver {
    /// Creates a receiver for a plan from a target vehicle.
    ///
    /// @param targetSystem - The sending system's id.
    /// @param targetComponent - The sending component's id.
    /// @param missionType - The `MAV_MISSION_TYPE` to transfer; defaults to the main
    ///   mission.
    #[napi(constructor)]
    pub fn new(target_system: u8, target_component: u8, mission_type: Option<u8>) -> Self {
        Self {
            inner: CoreReceiver::new(target_system, target_component, mission_type.unwrap_or(0)),
        }
    }

    /// Builds the frame that starts a download.
    ///
    /// @param header - The addressing fields to stamp on the frame.
    /// @returns The `MISSION_REQUEST_LIST` frame.
    #[napi]
    pub fn request_list(&self, header: MavlinkHeader) -> Result<MavlinkFrame> {
        self.inner
            .request_list_frame(Header::from(header))
            .map(MavlinkFrame::from_frame)
            .map_err(error_of)
    }

    /// Handles an incoming frame, if it is one this transfer is waiting for.
    ///
    /// A `MISSION_COUNT` opens the transfer and a `MISSION_ITEM_INT` advances it.
    ///
    /// @param frame - The frame off the link.
    /// @param header - The addressing fields to stamp on the reply.
    /// @returns The step taken, or `null` if the frame carries a message this transfer does
    ///   not handle.
    #[napi]
    pub fn on_frame(
        &mut self,
        frame: &MavlinkFrame,
        header: MavlinkHeader,
    ) -> Result<Option<ReceiverStep>> {
        let step = self
            .inner
            .on_frame(frame.frame(), Header::from(header))
            .map_err(error_of)?;
        Ok(step.map(|step| ReceiverStep {
            kind: match step.action {
                ReceiverAction::Request(_) => "request",
                ReceiverAction::Ack(_) => "ack",
            },
            accepted: step.accepted.map(|item| {
                let mut payload = [0u8; pamoja_mavlink::MAX_PAYLOAD];
                let len = item.encode(&mut payload);
                payload[..len].to_vec()
            }),
            reply: step.reply,
        }))
    }

    /// Whether every item has been received and the acknowledgement produced.
    #[napi(getter)]
    pub fn complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// The next sequence number the receiver expects.
    #[napi(getter)]
    pub fn expected(&self) -> u16 {
        self.inner.expected()
    }
}

/// What one incoming frame produced for a mission sender.
#[napi]
pub struct SenderStep {
    kind: &'static str,
    reply: Option<CoreFrame>,
    result: Option<u8>,
}

#[napi]
impl SenderStep {
    /// What happened: `reply` when there is a frame to send, or `finished` when the receiver
    /// acknowledged the transfer.
    #[napi(getter)]
    pub fn kind(&self) -> String {
        self.kind.to_owned()
    }

    /// The frame to send back, when there is one.
    #[napi(getter)]
    pub fn reply(&self) -> Option<MavlinkFrame> {
        self.reply.map(MavlinkFrame::from_frame)
    }

    /// The receiver's `MAV_MISSION_RESULT`, when the transfer finished.
    #[napi(getter)]
    pub fn result(&self) -> Option<u8> {
        self.result
    }
}

/// Holds a plan and answers a receiver's requests for its items.
#[napi]
pub struct MissionSender {
    items: Vec<MissionItemInt>,
    target_system: u8,
    target_component: u8,
    mission_type: u8,
}

impl MissionSender {
    fn with<R>(&self, query: impl FnOnce(&CoreSender<'_>) -> R) -> R {
        query(&CoreSender::new(
            &self.items,
            self.target_system,
            self.target_component,
            self.mission_type,
        ))
    }
}

#[napi]
impl MissionSender {
    /// Creates a sender for a plan bound for a target vehicle, with no items yet.
    ///
    /// @param targetSystem - The receiving system's id.
    /// @param targetComponent - The receiving component's id.
    /// @param missionType - The `MAV_MISSION_TYPE` of the plan; defaults to the main
    ///   mission.
    #[napi(constructor)]
    pub fn new(target_system: u8, target_component: u8, mission_type: Option<u8>) -> Self {
        Self {
            items: Vec::new(),
            target_system,
            target_component,
            mission_type: mission_type.unwrap_or(0),
        }
    }

    /// Appends an item to the plan.
    ///
    /// The sender stamps the sequence number, target ids, and mission type onto each item as
    /// it is handed out, so the item need only carry its content: command, frame, position,
    /// and parameters. Build one by field name with the `MISSION_ITEM_INT` schema.
    ///
    /// @param item - The item, as a `MISSION_ITEM_INT` message or its payload.
    #[napi]
    pub fn add_item(&mut self, item: Buffer) -> Result<()> {
        let decoded = MissionItemInt::decode(&item).map_err(error_of)?;
        self.items.push(decoded);
        Ok(())
    }

    /// The number of items in the plan.
    #[napi(getter)]
    pub fn length(&self) -> u16 {
        self.with(|plan| plan.len())
    }

    /// Builds the frame that opens an upload.
    ///
    /// @param header - The addressing fields to stamp on the frame.
    /// @returns The `MISSION_COUNT` frame.
    #[napi]
    pub fn count(&self, header: MavlinkHeader) -> Result<MavlinkFrame> {
        self.with(|plan| plan.count_frame(Header::from(header)))
            .map(MavlinkFrame::from_frame)
            .map_err(error_of)
    }

    /// Handles an incoming frame, if it is one this transfer answers.
    ///
    /// A `MISSION_REQUEST_LIST` is answered with the count, a `MISSION_REQUEST_INT` (or the
    /// older `MISSION_REQUEST`) with the item asked for, and a request past the end of the
    /// plan with a `MISSION_ACK` reporting an invalid sequence. A `MISSION_ACK` from the
    /// receiver ends the transfer.
    ///
    /// @param frame - The frame off the link.
    /// @param header - The addressing fields to stamp on the reply.
    /// @returns The step taken, or `null` if the frame carries a message this transfer does
    ///   not handle.
    #[napi]
    pub fn on_frame(
        &self,
        frame: &MavlinkFrame,
        header: MavlinkHeader,
    ) -> Result<Option<SenderStep>> {
        let step = self
            .with(|plan| plan.on_frame(frame.frame(), Header::from(header)))
            .map_err(error_of)?;
        Ok(step.map(|step| match step {
            CoreSenderStep::Reply(reply) => SenderStep {
                kind: "reply",
                reply: Some(reply),
                result: None,
            },
            CoreSenderStep::Finished(result) => SenderStep {
                kind: "finished",
                reply: None,
                result: Some(result),
            },
        }))
    }
}

/// What an incoming acknowledgement means for the command in flight.
#[napi(object)]
pub struct AckOutcome {
    /// `unrelated` if the ack was for another command, `inProgress` if the command is still
    /// running, or `final` if it finished.
    pub kind: String,
    /// The progress percent when in progress (255 when the autopilot does not report one),
    /// or the `MAV_RESULT` when final.
    pub value: Option<u8>,
}

/// Tracks one command awaiting its acknowledgement.
#[napi]
pub struct CommandProtocol {
    inner: CoreCommand,
}

#[napi]
impl CommandProtocol {
    /// Starts tracking a command.
    ///
    /// @param command - The `MAV_CMD` id being sent.
    /// @param maxRetries - How many times the command may be resent after a timeout before
    ///   the caller gives up; defaults to `MAVLINK_MAX_RETRIES`.
    #[napi(constructor)]
    pub fn new(command: u16, max_retries: Option<u8>) -> Self {
        Self {
            inner: CoreCommand::new(command, max_retries.unwrap_or(MAX_RETRIES)),
        }
    }

    /// The command id being tracked.
    #[napi(getter)]
    pub fn command(&self) -> u16 {
        self.inner.command()
    }

    /// The `confirmation` count to stamp on the command being sent: zero for the first
    /// transmission, incremented on each retransmission.
    #[napi(getter)]
    pub fn confirmation(&self) -> u8 {
        self.inner.confirmation()
    }

    /// Classifies an incoming frame against the command in flight.
    ///
    /// @param frame - The frame off the link.
    /// @returns The outcome, or `null` if the frame is not a `COMMAND_ACK`.
    #[napi]
    pub fn on_frame(&self, frame: &MavlinkFrame) -> Result<Option<AckOutcome>> {
        let outcome = self.inner.on_frame(frame.frame()).map_err(error_of)?;
        Ok(outcome.map(|outcome| match outcome {
            CoreAckOutcome::Unrelated => AckOutcome {
                kind: "unrelated".to_owned(),
                value: None,
            },
            CoreAckOutcome::InProgress(progress) => AckOutcome {
                kind: "inProgress".to_owned(),
                value: Some(progress),
            },
            CoreAckOutcome::Final(result) => AckOutcome {
                kind: "final".to_owned(),
                value: Some(result),
            },
        }))
    }

    /// Records a timeout and reports whether the command may be resent.
    ///
    /// @returns The new confirmation count to stamp on the resend, or `null` once the retry
    ///   budget is exhausted.
    #[napi]
    pub fn on_timeout(&mut self) -> Option<u8> {
        self.inner.on_timeout()
    }
}

/// Builds a setpoint `type_mask` from the fields to use.
///
/// @param flags - A bitwise-or of the `MAVLINK_TYPEMASK_*` flags; fields left out are ignored.
/// @returns The mask, as the `type_mask` field of a setpoint carries it.
#[napi]
pub fn mavlink_offboard_type_mask(flags: u32) -> u16 {
    let mut mask = TypeMask::ignore_all();
    if flags & MAVLINK_TYPEMASK_POSITION != 0 {
        mask = mask.use_position();
    }
    if flags & MAVLINK_TYPEMASK_VELOCITY != 0 {
        mask = mask.use_velocity();
    }
    if flags & MAVLINK_TYPEMASK_ACCELERATION != 0 {
        mask = mask.use_acceleration();
    }
    if flags & MAVLINK_TYPEMASK_YAW != 0 {
        mask = mask.use_yaw();
    }
    if flags & MAVLINK_TYPEMASK_YAW_RATE != 0 {
        mask = mask.use_yaw_rate();
    }
    if flags & MAVLINK_TYPEMASK_FORCE != 0 {
        mask = mask.force();
    }
    mask.bits()
}

/// Builds a local-frame position setpoint frame.
///
/// @param header - The addressing fields to stamp on the frame.
/// @param timeBootMs - The sender's boot timestamp, in milliseconds.
/// @param coordinateFrame - The `MAV_FRAME` of the setpoint.
/// @param targetSystem - The target system id.
/// @param targetComponent - The target component id.
/// @param x - The position along x, in metres in the chosen frame.
/// @param y - The position along y.
/// @param z - The position along z.
/// @returns The `SET_POSITION_TARGET_LOCAL_NED` frame.
#[napi]
#[allow(clippy::too_many_arguments)]
pub fn mavlink_offboard_local_position(
    header: MavlinkHeader,
    time_boot_ms: u32,
    coordinate_frame: u8,
    target_system: u8,
    target_component: u8,
    x: f64,
    y: f64,
    z: f64,
) -> Result<MavlinkFrame> {
    let setpoint = SetPositionTargetLocalNed::position(
        time_boot_ms,
        coordinate_frame,
        target_system,
        target_component,
        x as f32,
        y as f32,
        z as f32,
    );
    encode_message(Header::from(header), &setpoint)
        .map(MavlinkFrame::from_frame)
        .map_err(error_of)
}

/// Builds a local-frame velocity setpoint frame.
///
/// @param header - The addressing fields to stamp on the frame.
/// @param timeBootMs - The sender's boot timestamp, in milliseconds.
/// @param coordinateFrame - The `MAV_FRAME` of the setpoint.
/// @param targetSystem - The target system id.
/// @param targetComponent - The target component id.
/// @param vx - The velocity along x, in metres per second in the chosen frame.
/// @param vy - The velocity along y.
/// @param vz - The velocity along z.
/// @returns The `SET_POSITION_TARGET_LOCAL_NED` frame.
#[napi]
#[allow(clippy::too_many_arguments)]
pub fn mavlink_offboard_local_velocity(
    header: MavlinkHeader,
    time_boot_ms: u32,
    coordinate_frame: u8,
    target_system: u8,
    target_component: u8,
    vx: f64,
    vy: f64,
    vz: f64,
) -> Result<MavlinkFrame> {
    let setpoint = SetPositionTargetLocalNed::velocity(
        time_boot_ms,
        coordinate_frame,
        target_system,
        target_component,
        vx as f32,
        vy as f32,
        vz as f32,
    );
    encode_message(Header::from(header), &setpoint)
        .map(MavlinkFrame::from_frame)
        .map_err(error_of)
}

/// Builds a global-frame position setpoint frame.
///
/// @param header - The addressing fields to stamp on the frame.
/// @param timeBootMs - The sender's boot timestamp, in milliseconds.
/// @param coordinateFrame - The `MAV_FRAME` of the setpoint.
/// @param targetSystem - The target system id.
/// @param targetComponent - The target component id.
/// @param latInt - The latitude, in degrees times ten million.
/// @param lonInt - The longitude, in degrees times ten million.
/// @param alt - The altitude, in metres.
/// @returns The `SET_POSITION_TARGET_GLOBAL_INT` frame.
#[napi]
#[allow(clippy::too_many_arguments)]
pub fn mavlink_offboard_global_position(
    header: MavlinkHeader,
    time_boot_ms: u32,
    coordinate_frame: u8,
    target_system: u8,
    target_component: u8,
    lat_int: i32,
    lon_int: i32,
    alt: f64,
) -> Result<MavlinkFrame> {
    let setpoint = SetPositionTargetGlobalInt::position(
        time_boot_ms,
        coordinate_frame,
        target_system,
        target_component,
        lat_int,
        lon_int,
        alt as f32,
    );
    encode_message(Header::from(header), &setpoint)
        .map(MavlinkFrame::from_frame)
        .map_err(error_of)
}
