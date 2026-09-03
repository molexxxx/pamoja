//! The service protocols driven by frames rather than decoded messages.
//!
//! Each machine in this module speaks in typed messages, which is the right level for the
//! logic. A caller holding a link has frames, though, and turning one into the right call
//! means decoding the payload, matching the message id, calling the machine, and encoding
//! whatever it answers. That glue is the same every time, so it lives here once: a machine
//! takes an incoming [`Frame`], and gives back the frame to send in reply and what happened.
//!
//! Like the machines themselves this has no IO, no timers, and no allocation; a frame is a
//! fixed buffer of bytes.

use crate::dialect::{
    encode_message, mav_mission_result, CommandAck, Message, MissionAck, MissionCount,
    MissionItemInt, MissionRequest, MissionRequestInt, MissionRequestList,
};
use crate::error::Result;
use crate::frame::{Frame, Header};

use super::{AckOutcome, CommandProtocol, MissionReceiver, MissionSender, ReceiverAction};

/// What one incoming frame produced for a [`MissionReceiver`].
#[derive(Clone, Copy, Debug)]
pub struct ReceiverStep {
    /// The item the frame carried, if it was the one expected next.
    pub accepted: Option<MissionItemInt>,
    /// The next thing to send, as the machine sees it.
    pub action: ReceiverAction,
    /// That same thing, ready for the link.
    pub reply: Frame,
}

impl MissionReceiver {
    /// Builds the frame that starts a download.
    ///
    /// # Arguments
    ///
    /// * `header` - the addressing fields to stamp on the frame.
    ///
    /// # Returns
    ///
    /// The `MISSION_REQUEST_LIST` frame.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::PayloadTooLong`](crate::MavlinkError::PayloadTooLong) if the
    /// message does not fit a frame, which a request list never fails.
    pub fn request_list_frame(&self, header: Header) -> Result<Frame> {
        encode_message(header, &self.request_list())
    }

    /// Handles an incoming frame, if it is one this transfer is waiting for.
    ///
    /// A `MISSION_COUNT` opens the transfer and a `MISSION_ITEM_INT` advances it; any other
    /// message is not this machine's to handle and is reported as such rather than refused,
    /// so a caller can route one link's traffic through several machines.
    ///
    /// # Arguments
    ///
    /// * `frame` - the frame off the link.
    /// * `header` - the addressing fields to stamp on the reply.
    ///
    /// # Returns
    ///
    /// The step taken, or [`None`] if the frame carries a message this transfer does not
    /// handle.
    ///
    /// # Errors
    ///
    /// Returns an error only if the reply does not fit a frame, which no mission message can
    /// cause; a short or long payload decodes the way the message layer defines.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_mavlink::dialect::{encode_message, MissionCount, MissionItemInt};
    /// use pamoja_mavlink::protocol::{MissionReceiver, ReceiverAction};
    /// use pamoja_mavlink::Header;
    ///
    /// let vehicle = Header::new(1, 1, 0);
    /// let station = Header::new(255, 190, 0);
    /// let mut download = MissionReceiver::new(1, 1, 0);
    ///
    /// // The vehicle announces one item, and the receiver asks for it.
    /// let count = MissionCount { count: 1, target_system: 255, target_component: 190, mission_type: 0, opaque_id: 0 };
    /// let step = download
    ///     .on_frame(&encode_message(vehicle, &count)?, station)?
    ///     .expect("a count is handled");
    /// assert!(matches!(step.action, ReceiverAction::Request(request) if request.seq == 0));
    /// assert_eq!(step.reply.message_id(), 51); // MISSION_REQUEST_INT
    ///
    /// // A frame for some other machine is passed over rather than refused.
    /// let heartbeat = pamoja_mavlink::dialect::Heartbeat { custom_mode: 0, type_: 2, autopilot: 3, base_mode: 0, system_status: 4, mavlink_version: 3 };
    /// assert!(download.on_frame(&encode_message(vehicle, &heartbeat)?, station)?.is_none());
    /// # Ok::<(), pamoja_mavlink::MavlinkError>(())
    /// ```
    pub fn on_frame(&mut self, frame: &Frame, header: Header) -> Result<Option<ReceiverStep>> {
        let id = frame.message_id();
        if id == MissionCount::ID {
            let count = MissionCount::decode(frame.payload())?;
            let action = self.on_count(count.count);
            Ok(Some(ReceiverStep {
                accepted: None,
                reply: reply_of(&action, header)?,
                action,
            }))
        } else if id == MissionItemInt::ID {
            let item = MissionItemInt::decode(frame.payload())?;
            let (accepted, action) = self.on_item(&item);
            Ok(Some(ReceiverStep {
                accepted,
                reply: reply_of(&action, header)?,
                action,
            }))
        } else {
            Ok(None)
        }
    }
}

fn reply_of(action: &ReceiverAction, header: Header) -> Result<Frame> {
    match action {
        ReceiverAction::Request(request) => encode_message(header, request),
        ReceiverAction::Ack(ack) => encode_message(header, ack),
    }
}

/// What one incoming frame produced for a [`MissionSender`].
///
/// A reply carries a whole frame buffer, which dwarfs the other variant; boxing it would
/// need an allocator, and this layer has none.
#[derive(Clone, Copy, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum SenderStep {
    /// Send this frame: the count that opens the transfer, a requested item, or an error.
    Reply(Frame),
    /// The receiver has acknowledged the transfer with this
    /// [`MAV_MISSION_RESULT`](crate::dialect::mav_mission_result); nothing more to send.
    Finished(u8),
}

impl MissionSender<'_> {
    /// Builds the frame that opens an upload.
    ///
    /// # Arguments
    ///
    /// * `header` - the addressing fields to stamp on the frame.
    ///
    /// # Returns
    ///
    /// The `MISSION_COUNT` frame.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::PayloadTooLong`](crate::MavlinkError::PayloadTooLong) if the
    /// message does not fit a frame, which a count never fails.
    pub fn count_frame(&self, header: Header) -> Result<Frame> {
        encode_message(header, &self.count())
    }

    /// Handles an incoming frame, if it is one this transfer answers.
    ///
    /// A `MISSION_REQUEST_LIST` is answered with the count, a `MISSION_REQUEST_INT` (or the
    /// older `MISSION_REQUEST`) with the item asked for, and a request past the end of the
    /// plan with a `MISSION_ACK` reporting an invalid sequence. A `MISSION_ACK` from the
    /// receiver ends the transfer. Any other message is reported as not handled.
    ///
    /// # Arguments
    ///
    /// * `frame` - the frame off the link.
    /// * `header` - the addressing fields to stamp on the reply.
    ///
    /// # Returns
    ///
    /// The step taken, or [`None`] if the frame carries a message this transfer does not
    /// handle.
    ///
    /// # Errors
    ///
    /// Returns an error only if the reply does not fit a frame, which no mission message can
    /// cause; a short or long payload decodes the way the message layer defines.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_mavlink::dialect::{encode_message, Message, MissionItemInt, MissionRequestInt};
    /// use pamoja_mavlink::protocol::{MissionSender, SenderStep};
    /// use pamoja_mavlink::Header;
    ///
    /// let waypoint = MissionItemInt { command: 16, x: -338_567_800, y: 1_512_153_000, z: 50.0, ..MissionItemInt::zeroed() };
    /// let plan = [waypoint];
    /// let upload = MissionSender::new(&plan, 1, 1, 0);
    ///
    /// // The vehicle asks for item 0 and gets it back, stamped with its sequence number.
    /// let request = MissionRequestInt { seq: 0, target_system: 255, target_component: 190, mission_type: 0 };
    /// let step = upload
    ///     .on_frame(&encode_message(Header::new(1, 1, 0), &request)?, Header::new(255, 190, 0))?
    ///     .expect("a request is handled");
    /// let SenderStep::Reply(reply) = step else { panic!("an item is sent") };
    /// assert_eq!(MissionItemInt::decode(reply.payload())?.seq, 0);
    /// # Ok::<(), pamoja_mavlink::MavlinkError>(())
    /// ```
    pub fn on_frame(&self, frame: &Frame, header: Header) -> Result<Option<SenderStep>> {
        let id = frame.message_id();
        if id == MissionRequestList::ID {
            MissionRequestList::decode(frame.payload())?;
            encode_message(header, &self.count()).map(|frame| Some(SenderStep::Reply(frame)))
        } else if id == MissionRequestInt::ID || id == MissionRequest::ID {
            let seq = if id == MissionRequestInt::ID {
                MissionRequestInt::decode(frame.payload())?.seq
            } else {
                MissionRequest::decode(frame.payload())?.seq
            };
            let reply = match self.item(seq) {
                Some(item) => encode_message(header, &item)?,
                None => encode_message(header, &self.refuse(mav_mission_result::INVALID_SEQUENCE))?,
            };
            Ok(Some(SenderStep::Reply(reply)))
        } else if id == MissionAck::ID {
            let ack = MissionAck::decode(frame.payload())?;
            Ok(Some(SenderStep::Finished(ack.type_)))
        } else {
            Ok(None)
        }
    }

    fn refuse(&self, result: u8) -> MissionAck {
        let count = self.count();
        MissionAck {
            target_system: count.target_system,
            target_component: count.target_component,
            type_: result,
            mission_type: count.mission_type,
            ..MissionAck::zeroed()
        }
    }
}

impl CommandProtocol {
    /// Classifies an incoming frame against the command in flight, if it is an acknowledgement.
    ///
    /// # Arguments
    ///
    /// * `frame` - the frame off the link.
    ///
    /// # Returns
    ///
    /// The outcome, or [`None`] if the frame is not a `COMMAND_ACK`.
    ///
    /// # Errors
    ///
    /// Never fails in practice; the `Result` mirrors the message layer, which zero-extends a
    /// short payload and ignores the tail of a long one rather than refusing either.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_mavlink::dialect::{encode_message, mav_cmd, mav_result, CommandAck};
    /// use pamoja_mavlink::protocol::{AckOutcome, CommandProtocol};
    /// use pamoja_mavlink::Header;
    ///
    /// let arm = CommandProtocol::new(mav_cmd::COMPONENT_ARM_DISARM, 3);
    /// let ack = CommandAck { command: mav_cmd::COMPONENT_ARM_DISARM, result: mav_result::ACCEPTED, ..CommandAck::zeroed() };
    /// let outcome = arm.on_frame(&encode_message(Header::new(1, 1, 0), &ack)?)?;
    /// assert_eq!(outcome, Some(AckOutcome::Final(mav_result::ACCEPTED)));
    /// # Ok::<(), pamoja_mavlink::MavlinkError>(())
    /// ```
    pub fn on_frame(&self, frame: &Frame) -> Result<Option<AckOutcome>> {
        if frame.message_id() == CommandAck::ID {
            Ok(Some(self.on_ack(&CommandAck::decode(frame.payload())?)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::{mav_cmd, mav_result};

    const VEHICLE: Header = Header::new(1, 1, 0);
    const STATION: Header = Header::new(255, 190, 0);

    fn plan() -> [MissionItemInt; 2] {
        [
            MissionItemInt {
                command: mav_cmd::NAV_TAKEOFF,
                z: 20.0,
                ..MissionItemInt::zeroed()
            },
            MissionItemInt {
                command: mav_cmd::NAV_WAYPOINT,
                x: -338_567_800,
                y: 1_512_153_000,
                z: 50.0,
                ..MissionItemInt::zeroed()
            },
        ]
    }

    #[test]
    fn a_whole_upload_runs_frame_to_frame() -> Result<()> {
        let items = plan();
        let upload = MissionSender::new(&items, 1, 1, 0);
        let mut download = MissionReceiver::new(255, 190, 0);

        // The station opens with a request list, and the vehicle answers with the count.
        let opened = download.request_list_frame(STATION)?;
        let Some(SenderStep::Reply(mut from_vehicle)) = upload.on_frame(&opened, VEHICLE)? else {
            panic!("a request list is answered");
        };
        assert_eq!(from_vehicle.message_id(), MissionCount::ID);

        // Each side answers the other until the receiver acknowledges.
        let mut accepted = 0;
        loop {
            let step = download
                .on_frame(&from_vehicle, STATION)?
                .expect("the vehicle only sends what the receiver handles");
            if step.accepted.is_some() {
                accepted += 1;
            }
            match upload.on_frame(&step.reply, VEHICLE)? {
                Some(SenderStep::Reply(next)) => from_vehicle = next,
                Some(SenderStep::Finished(result)) => {
                    assert_eq!(result, mav_mission_result::ACCEPTED);
                    break;
                }
                None => panic!("the receiver only sends what the sender handles"),
            }
        }
        assert_eq!(accepted, 2);
        assert!(download.is_complete());
        Ok(())
    }

    #[test]
    fn a_request_past_the_plan_is_refused_with_the_published_result() -> Result<()> {
        let items = plan();
        let upload = MissionSender::new(&items, 1, 1, 0);
        let request = MissionRequestInt {
            seq: 7,
            target_system: 255,
            target_component: 190,
            mission_type: 0,
        };
        let Some(SenderStep::Reply(reply)) =
            upload.on_frame(&encode_message(VEHICLE, &request)?, STATION)?
        else {
            panic!("a request is answered");
        };
        assert_eq!(reply.message_id(), MissionAck::ID);
        assert_eq!(
            MissionAck::decode(reply.payload())?.type_,
            mav_mission_result::INVALID_SEQUENCE
        );
        Ok(())
    }

    #[test]
    fn the_older_request_message_is_answered_too() -> Result<()> {
        let items = plan();
        let upload = MissionSender::new(&items, 1, 1, 0);
        let request = MissionRequest {
            seq: 1,
            target_system: 255,
            target_component: 190,
            mission_type: 0,
        };
        let Some(SenderStep::Reply(reply)) =
            upload.on_frame(&encode_message(VEHICLE, &request)?, STATION)?
        else {
            panic!("a request is answered");
        };
        assert_eq!(MissionItemInt::decode(reply.payload())?.seq, 1);
        Ok(())
    }

    #[test]
    fn a_frame_for_another_machine_is_passed_over() -> Result<()> {
        let items = plan();
        let upload = MissionSender::new(&items, 1, 1, 0);
        let mut download = MissionReceiver::new(255, 190, 0);
        let arm = CommandProtocol::new(mav_cmd::COMPONENT_ARM_DISARM, 3);

        let ack = CommandAck {
            command: mav_cmd::COMPONENT_ARM_DISARM,
            result: mav_result::IN_PROGRESS,
            progress: 40,
            ..CommandAck::zeroed()
        };
        let frame = encode_message(VEHICLE, &ack)?;

        assert!(upload.on_frame(&frame, STATION)?.is_none());
        assert!(download.on_frame(&frame, STATION)?.is_none());
        assert_eq!(arm.on_frame(&frame)?, Some(AckOutcome::InProgress(40)));
        Ok(())
    }
}
