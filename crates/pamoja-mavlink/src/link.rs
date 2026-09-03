//! The byte-stream link seam and an in-process autopilot to exercise it with no hardware.
//!
//! MAVLink runs over anything that moves bytes: a serial line to a flight controller, a
//! UDP socket to a ground station, a radio. This module abstracts that as a single
//! [`ByteLink`] trait, so the same logic drives all of them, and a real serial or UDP
//! backend plugs into it later without touching the protocol above. A [`Connection`]
//! pairs a link with a [`Parser`](crate::Parser) and, optionally, signing, so sending and receiving whole
//! messages is one call each.
//!
//! To make the whole path testable with nothing plugged in, [`MemoryLink`] connects two
//! connections through an in-memory pipe, and [`SitlAutopilot`] is a software-in-the-loop
//! stand-in that heartbeats and answers commands the way a real autopilot would. This is
//! the drone equivalent of the loopback transport and device simulators the rest of the
//! SDK uses to run with zero hardware.
//!
//! This layer is available with the default `std` feature; the protocol core below it is
//! `no_std`.

use crate::dialect::{
    self, CommandAck, CommandLong, Heartbeat, Message, MissionCount, MissionItemInt,
    MissionRequest, MissionRequestInt, MissionRequestList,
};
use crate::error::{MavlinkError, Result};
use crate::frame::{Frame, Header};
use crate::protocol::mission::{MissionReceiver, MissionSender, ReceiverAction};
use crate::signing::{Signer, Verifier};

// Resolves a message id to its CRC_EXTRA through the common-dialect registry, so the
// parser can validate frames off the link.
fn crc_extra_for(msgid: u32) -> Option<u8> {
    dialect::crc_extra(msgid)
}

/// A bidirectional byte stream: the seam a MAVLink [`Connection`] moves frames over.
///
/// Implemented here by [`MemoryLink`] for hardware-free testing; a serial port or UDP
/// socket implements the same two methods to carry MAVLink over real links.
pub trait ByteLink {
    /// Reads available bytes into `buf`, returning how many were read.
    ///
    /// # Arguments
    ///
    /// * `buf` - the destination for the bytes read.
    ///
    /// # Returns
    ///
    /// The number of bytes read; `0` means the link has no more input.
    ///
    /// # Errors
    ///
    /// Returns a [`MavlinkError`] if the underlying link fails.
    fn read(&mut self, buf: &mut [u8]) -> impl core::future::Future<Output = Result<usize>>;

    /// Writes all of `data` to the link.
    ///
    /// # Arguments
    ///
    /// * `data` - the bytes to write.
    ///
    /// # Returns
    ///
    /// `Ok(())` once every byte has been handed to the link.
    ///
    /// # Errors
    ///
    /// Returns a [`MavlinkError`] if the underlying link fails.
    fn write_all(&mut self, data: &[u8]) -> impl core::future::Future<Output = Result<()>>;
}

// The size of a read from the link into the connection's staging buffer.
const READ_CHUNK: usize = 512;

/// A MAVLink endpoint over a [`ByteLink`]: sends and receives whole messages, and signs
/// and verifies them when configured to.
///
/// A connection owns its sending identity and sequence counter, a streaming [`Parser`](crate::Parser) for
/// the bytes it reads, and optional signing. Attach a [`Signer`] to sign every outgoing
/// frame and a [`Verifier`] to check every signed incoming one.
pub struct Connection<L> {
    link: L,
    parser: crate::parser::Parser,
    header: Header,
    signer: Option<Signer>,
    verifier: Option<Verifier>,
    require_signed: bool,
    staging: [u8; READ_CHUNK],
    staged_len: usize,
    staged_pos: usize,
}

impl<L: ByteLink> Connection<L> {
    /// Creates a connection that sends as the given system and component.
    ///
    /// # Arguments
    ///
    /// * `link` - the byte stream to carry frames over.
    /// * `system_id` - this endpoint's system id.
    /// * `component_id` - this endpoint's component id.
    ///
    /// # Returns
    ///
    /// The connection, with signing off.
    pub fn new(link: L, system_id: u8, component_id: u8) -> Self {
        Connection {
            link,
            parser: crate::parser::Parser::new(),
            header: Header::new(system_id, component_id, 0),
            signer: None,
            verifier: None,
            require_signed: false,
            staging: [0u8; READ_CHUNK],
            staged_len: 0,
            staged_pos: 0,
        }
    }

    /// Signs every outgoing frame with `signer`.
    ///
    /// # Arguments
    ///
    /// * `signer` - the signer to stamp outgoing frames with.
    ///
    /// # Returns
    ///
    /// The connection, for chaining.
    pub fn with_signer(mut self, signer: Signer) -> Self {
        self.signer = Some(signer);
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
    /// The connection, for chaining.
    pub fn with_verifier(mut self, verifier: Verifier) -> Self {
        self.verifier = Some(verifier);
        self.require_signed = true;
        self
    }

    /// Sends a typed message, signing it if a signer is attached.
    ///
    /// # Arguments
    ///
    /// * `message` - the message to send.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the frame has been written to the link.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::PayloadTooLong`] if the message does not fit a frame, or a
    /// link error from the underlying [`ByteLink`].
    pub async fn send<M: Message>(&mut self, message: &M) -> Result<()> {
        let mut payload = [0u8; crate::frame::MAX_PAYLOAD];
        let len = message.encode(&mut payload);
        let frame = match self.signer.as_mut() {
            Some(signer) => signer.sign(self.header, M::ID, &payload[..len], M::CRC_EXTRA)?,
            None => Frame::encode_v2(self.header, M::ID, &payload[..len], M::CRC_EXTRA)?,
        };
        self.link.write_all(frame.as_bytes()).await?;
        self.header.sequence = self.header.sequence.wrapping_add(1);
        Ok(())
    }

    /// Receives the next whole frame from the link, verifying its signature if required.
    ///
    /// # Returns
    ///
    /// The next valid frame.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::Closed`] if the link ends before a frame arrives,
    /// [`MavlinkError::Unsigned`] if a signature is required but the frame is unsigned,
    /// [`MavlinkError::BadSignature`] or [`MavlinkError::ReplayedTimestamp`] if a signed
    /// frame does not verify, or a link error from the underlying [`ByteLink`].
    pub async fn recv(&mut self) -> Result<Frame> {
        loop {
            while self.staged_pos < self.staged_len {
                let byte = self.staging[self.staged_pos];
                self.staged_pos += 1;
                if let Some(frame) = self.parser.push_byte(byte, &crc_extra_for) {
                    if let Some(verifier) = self.verifier.as_mut() {
                        if frame.is_signed() {
                            verifier.verify(&frame)?;
                        } else if self.require_signed {
                            return Err(MavlinkError::Unsigned);
                        }
                    }
                    return Ok(frame);
                }
            }
            let n = self.link.read(&mut self.staging).await?;
            if n == 0 {
                return Err(MavlinkError::Closed);
            }
            self.staged_len = n;
            self.staged_pos = 0;
        }
    }

    /// Returns a shared reference to the underlying link.
    ///
    /// # Returns
    ///
    /// The link.
    pub fn link(&self) -> &L {
        &self.link
    }
}

/// An in-process byte link: one end of a bidirectional pipe between two connections.
///
/// [`pair`](MemoryLink::pair) makes two ends whose writes appear as the other's reads, so two
/// [`Connection`]s (or a [`Vehicle`](crate::vehicle::Vehicle) and a [`SitlAutopilot`]) exchange
/// frames with no socket and no hardware. A read awaits until bytes are available and reports
/// end of input once the other end is dropped, so the two ends can run as concurrent tasks the
/// way a real link's peers do.
pub struct MemoryLink {
    stream: tokio::io::DuplexStream,
}

impl MemoryLink {
    /// Creates a connected pair of links.
    ///
    /// # Returns
    ///
    /// Two ends; bytes written to one are read from the other.
    pub fn pair() -> (MemoryLink, MemoryLink) {
        // A generous buffer so a burst of frames never blocks the writer in a test.
        let (a, b) = tokio::io::duplex(64 * 1024);
        (MemoryLink { stream: a }, MemoryLink { stream: b })
    }
}

impl ByteLink for MemoryLink {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        use tokio::io::AsyncReadExt;
        self.stream
            .read(buf)
            .await
            .map_err(|_| MavlinkError::Closed)
    }

    async fn write_all(&mut self, data: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        self.stream
            .write_all(data)
            .await
            .map_err(|_| MavlinkError::Closed)
    }
}

/// A hardware-free autopilot stand-in for software-in-the-loop testing.
///
/// It behaves like the parts of an autopilot a ground station first talks to: it emits a
/// [`Heartbeat`] on demand, answers a [`CommandLong`] with a [`CommandAck`], and speaks both
/// sides of the mission protocol, receiving an uploaded plan and serving it back on download.
/// Wire it to one end of a [`MemoryLink::pair`] and drive a ground-station [`Connection`] or a
/// [`Vehicle`](crate::vehicle::Vehicle) on the other to exercise the full connect, command,
/// mission, and telemetry path in a test.
pub struct SitlAutopilot {
    connection: Connection<MemoryLink>,
    mission: Vec<MissionItemInt>,
    receiving: Option<(MissionReceiver, Vec<MissionItemInt>)>,
}

impl SitlAutopilot {
    /// Creates a SITL autopilot on a link, sending as the given system and component.
    ///
    /// # Arguments
    ///
    /// * `link` - the autopilot's end of a linked pair.
    /// * `system_id` - the vehicle's system id.
    /// * `component_id` - the autopilot component id.
    ///
    /// # Returns
    ///
    /// The autopilot, with signing off.
    pub fn new(link: MemoryLink, system_id: u8, component_id: u8) -> Self {
        SitlAutopilot {
            connection: Connection::new(link, system_id, component_id),
            mission: Vec::new(),
            receiving: None,
        }
    }

    /// Preloads the plan the autopilot serves on a mission download.
    ///
    /// # Arguments
    ///
    /// * `items` - the mission items to store.
    pub fn load_mission(&mut self, items: &[MissionItemInt]) {
        self.mission = items.to_vec();
    }

    /// Signs the autopilot's outgoing frames and verifies incoming ones with the same key.
    ///
    /// # Arguments
    ///
    /// * `signer` - the signer for outgoing frames.
    /// * `verifier` - the verifier for incoming signed frames.
    ///
    /// # Returns
    ///
    /// The autopilot, for chaining.
    pub fn secured(mut self, signer: Signer, verifier: Verifier) -> Self {
        self.connection = self.connection.with_signer(signer).with_verifier(verifier);
        self
    }

    /// Emits a heartbeat announcing the vehicle as an active quadrotor.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the heartbeat has been sent.
    ///
    /// # Errors
    ///
    /// Returns a link error if the heartbeat cannot be written.
    pub async fn emit_heartbeat(&mut self) -> Result<()> {
        let heartbeat = Heartbeat {
            custom_mode: 0,
            type_: dialect::mav_type::QUADROTOR,
            autopilot: dialect::mav_autopilot::ARDUPILOTMEGA,
            base_mode: dialect::mav_mode_flag::CUSTOM_MODE_ENABLED,
            system_status: dialect::mav_state::ACTIVE,
            mavlink_version: 3,
        };
        self.connection.send(&heartbeat).await
    }

    /// Reads one frame and answers it the way an autopilot would.
    ///
    /// A command is acknowledged as accepted; a mission upload is received and stored; a
    /// mission download is served from the stored plan. Any other frame is read and left
    /// unanswered. Call it in a loop to keep the autopilot responsive.
    ///
    /// # Returns
    ///
    /// The frame that was read.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Connection::recv`] and [`Connection::send`].
    pub async fn serve_once(&mut self) -> Result<Frame> {
        let frame = self.connection.recv().await?;
        let (sys, comp) = (frame.system_id(), frame.component_id());
        match frame.message_id() {
            CommandLong::ID => {
                let command = CommandLong::decode(frame.payload())?;
                let ack = CommandAck {
                    command: command.command,
                    result: dialect::mav_result::ACCEPTED,
                    progress: 0,
                    result_param2: 0,
                    target_system: sys,
                    target_component: comp,
                };
                self.connection.send(&ack).await?;
            }
            MissionCount::ID => {
                let count = MissionCount::decode(frame.payload())?.count;
                let mut receiver =
                    MissionReceiver::new(sys, comp, dialect::mav_mission_type::MISSION);
                let buffer = Vec::with_capacity(count as usize);
                self.step_receive(receiver.on_count(count), receiver, buffer)
                    .await?;
            }
            MissionItemInt::ID => {
                if let Some((mut receiver, mut buffer)) = self.receiving.take() {
                    let item = MissionItemInt::decode(frame.payload())?;
                    let (accepted, action) = receiver.on_item(&item);
                    if let Some(item) = accepted {
                        buffer.push(item);
                    }
                    self.step_receive(action, receiver, buffer).await?;
                }
            }
            MissionRequestList::ID => {
                let count = MissionSender::new(
                    &self.mission,
                    sys,
                    comp,
                    dialect::mav_mission_type::MISSION,
                )
                .count();
                self.connection.send(&count).await?;
            }
            MissionRequestInt::ID => {
                let seq = MissionRequestInt::decode(frame.payload())?.seq;
                self.serve_item(sys, comp, seq).await?;
            }
            MissionRequest::ID => {
                let seq = MissionRequest::decode(frame.payload())?.seq;
                self.serve_item(sys, comp, seq).await?;
            }
            _ => {}
        }
        Ok(frame)
    }

    // Applies one mission-receiver step: send the next request and keep receiving, or store the
    // completed plan and send the acknowledgement.
    async fn step_receive(
        &mut self,
        action: ReceiverAction,
        receiver: MissionReceiver,
        buffer: Vec<MissionItemInt>,
    ) -> Result<()> {
        match action {
            ReceiverAction::Request(request) => {
                self.connection.send(&request).await?;
                self.receiving = Some((receiver, buffer));
            }
            ReceiverAction::Ack(ack) => {
                self.mission = buffer;
                self.connection.send(&ack).await?;
            }
        }
        Ok(())
    }

    // Answers a request for one stored mission item.
    async fn serve_item(&mut self, sys: u8, comp: u8, seq: u16) -> Result<()> {
        let item = MissionSender::new(&self.mission, sys, comp, dialect::mav_mission_type::MISSION)
            .item(seq);
        if let Some(item) = item {
            self.connection.send(&item).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::KEY_LEN;

    const KEY: [u8; KEY_LEN] = [0x24; KEY_LEN];

    fn arm_command() -> CommandLong {
        CommandLong {
            param1: 1.0,
            param2: 0.0,
            param3: 0.0,
            param4: 0.0,
            param5: 0.0,
            param6: 0.0,
            param7: 0.0,
            command: dialect::mav_cmd::COMPONENT_ARM_DISARM,
            target_system: 1,
            target_component: 1,
            confirmation: 0,
        }
    }

    #[tokio::test]
    async fn a_heartbeat_crosses_the_link() {
        let (gcs_end, vehicle_end) = MemoryLink::pair();
        let mut vehicle = SitlAutopilot::new(vehicle_end, 1, 1);
        let mut gcs = Connection::new(gcs_end, 255, 190);

        vehicle.emit_heartbeat().await.unwrap();
        let frame = gcs.recv().await.unwrap();
        assert_eq!(frame.message_id(), Heartbeat::ID);
        let heartbeat = Heartbeat::decode(frame.payload()).unwrap();
        assert_eq!(heartbeat.system_status, dialect::mav_state::ACTIVE);
    }

    #[tokio::test]
    async fn a_command_is_answered_with_an_ack() {
        let (gcs_end, vehicle_end) = MemoryLink::pair();
        let mut vehicle = SitlAutopilot::new(vehicle_end, 1, 1);
        let mut gcs = Connection::new(gcs_end, 255, 190);

        gcs.send(&arm_command()).await.unwrap();
        let served = vehicle.serve_once().await.unwrap();
        assert_eq!(served.message_id(), CommandLong::ID);

        let frame = gcs.recv().await.unwrap();
        assert_eq!(frame.message_id(), CommandAck::ID);
        let ack = CommandAck::decode(frame.payload()).unwrap();
        assert_eq!(ack.command, dialect::mav_cmd::COMPONENT_ARM_DISARM);
        assert_eq!(ack.result, dialect::mav_result::ACCEPTED);
    }

    #[tokio::test]
    async fn a_signed_command_round_trips_over_the_link() {
        let (gcs_end, vehicle_end) = MemoryLink::pair();
        let mut vehicle = SitlAutopilot::new(vehicle_end, 1, 1)
            .secured(Signer::new(KEY, 1, 10_000), Verifier::new(KEY));
        let mut gcs = Connection::new(gcs_end, 255, 190)
            .with_signer(Signer::new(KEY, 2, 20_000))
            .with_verifier(Verifier::new(KEY));

        gcs.send(&arm_command()).await.unwrap();
        // The vehicle verifies the signed command before acting on it.
        vehicle.serve_once().await.unwrap();
        // The ground station verifies the signed acknowledgement.
        let frame = gcs.recv().await.unwrap();
        assert!(frame.is_signed());
        assert_eq!(frame.message_id(), CommandAck::ID);
    }

    #[tokio::test]
    async fn an_unsigned_frame_is_refused_when_signing_is_required() {
        let (gcs_end, vehicle_end) = MemoryLink::pair();
        // The vehicle requires signed frames; the ground station sends unsigned ones.
        let mut vehicle = SitlAutopilot::new(vehicle_end, 1, 1)
            .secured(Signer::new(KEY, 1, 10_000), Verifier::new(KEY));
        let mut gcs = Connection::new(gcs_end, 255, 190);

        gcs.send(&arm_command()).await.unwrap();
        assert_eq!(vehicle.serve_once().await, Err(MavlinkError::Unsigned));
    }
}
