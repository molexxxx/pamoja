//! The C ABI for the MAVLink service protocols: mission transfer, commands, and offboard
//! setpoints.
//!
//! A frame carries one message; a real exchange with an autopilot is a sequence of them with
//! rules about order, matching, and retransmission. The machines here hold those rules and
//! nothing else: no IO, no timers, no allocation beyond the handle. A caller feeds each one
//! the frames off its link and sends back what the machine hands it, applying its own timing
//! policy for timeouts and retransmission.
//!
//! Every machine takes a whole frame and answers with a whole frame, so the payload decoding,
//! message-id dispatch, and reply encoding happen once, here, rather than in each caller. A
//! frame a machine does not handle is reported as ignored rather than as an error, so one
//! link's traffic can be routed through several machines in turn.

use pamoja_mavlink::dialect::{
    Message, MissionItemInt, SetPositionTargetGlobalInt, SetPositionTargetLocalNed,
};
use pamoja_mavlink::protocol::{
    AckOutcome, CommandProtocol, MissionReceiver, MissionSender, ReceiverAction, ReceiverStep,
    SenderStep, TypeMask, MAX_RETRIES,
};
use pamoja_mavlink::{dialect, Frame, Header};

use crate::mavlink::{status_of, PamojaMavlinkFrame, PamojaMavlinkHeader};
use crate::mavlink_schema::PamojaMavlinkMessage;
use crate::{read_bytes, set_last_error, PamojaStatus};

/// The number of times a request is retransmitted before a transfer is abandoned, as the
/// mission protocol recommends.
pub const PAMOJA_MAVLINK_MAX_RETRIES: u8 = MAX_RETRIES;

/// The frame was not one this machine handles; nothing was produced.
pub const PAMOJA_MAVLINK_STEP_IGNORED: u32 = 0;

/// A mission receiver answered with a request for the next item.
pub const PAMOJA_MAVLINK_RECEIVER_REQUEST: u32 = 1;
/// A mission receiver answered with the acknowledgement that ends the transfer.
pub const PAMOJA_MAVLINK_RECEIVER_ACK: u32 = 2;

/// A mission sender answered with a frame to send.
pub const PAMOJA_MAVLINK_SENDER_REPLY: u32 = 1;
/// A mission sender saw the receiver's acknowledgement; the transfer is over.
pub const PAMOJA_MAVLINK_SENDER_FINISHED: u32 = 2;

/// An acknowledgement was for a different command; keep waiting.
pub const PAMOJA_MAVLINK_ACK_UNRELATED: u32 = 1;
/// The command is still running; the value is the reported progress percent, or 255 when
/// the autopilot does not report one.
pub const PAMOJA_MAVLINK_ACK_IN_PROGRESS: u32 = 2;
/// The command finished; the value is its `MAV_RESULT`.
pub const PAMOJA_MAVLINK_ACK_FINAL: u32 = 3;

/// Use the position fields of a setpoint.
pub const PAMOJA_MAVLINK_TYPEMASK_POSITION: u32 = 1 << 0;
/// Use the velocity fields of a setpoint.
pub const PAMOJA_MAVLINK_TYPEMASK_VELOCITY: u32 = 1 << 1;
/// Use the acceleration fields of a setpoint.
pub const PAMOJA_MAVLINK_TYPEMASK_ACCELERATION: u32 = 1 << 2;
/// Use the yaw field of a setpoint.
pub const PAMOJA_MAVLINK_TYPEMASK_YAW: u32 = 1 << 3;
/// Use the yaw rate field of a setpoint.
pub const PAMOJA_MAVLINK_TYPEMASK_YAW_RATE: u32 = 1 << 4;
/// Treat the acceleration fields as a force.
pub const PAMOJA_MAVLINK_TYPEMASK_FORCE: u32 = 1 << 5;

/// Writes a frame through an out-pointer, reporting a null pointer as an error.
unsafe fn emit(frame: Frame, out_frame: *mut *mut PamojaMavlinkFrame) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_frame = PamojaMavlinkFrame::into_handle(frame);
    PamojaStatus::Ok
}

/// Requests a plan's items in order and collects them, ending with an acknowledgement.
pub struct PamojaMavlinkMissionReceiver {
    inner: MissionReceiver,
}

/// Creates a receiver for a plan from a target vehicle.
///
/// # Arguments
///
/// * `target_system` - the sending system's id.
/// * `target_component` - the sending component's id.
/// * `mission_type` - the `MAV_MISSION_TYPE` to transfer.
///
/// # Returns
///
/// A receiver the caller releases with [`pamoja_mavlink_mission_receiver_free`].
#[no_mangle]
pub extern "C" fn pamoja_mavlink_mission_receiver_new(
    target_system: u8,
    target_component: u8,
    mission_type: u8,
) -> *mut PamojaMavlinkMissionReceiver {
    Box::into_raw(Box::new(PamojaMavlinkMissionReceiver {
        inner: MissionReceiver::new(target_system, target_component, mission_type),
    }))
}

/// Builds the frame that starts a download.
///
/// # Arguments
///
/// * `receiver` - the transfer.
/// * `header` - the addressing fields to stamp on the frame.
/// * `out_frame` - set to the `MISSION_REQUEST_LIST` frame, which the caller releases with
///   `pamoja_mavlink_frame_free`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `receiver` must be a live receiver handle, and `out_frame` must point at writable storage
/// for one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_mission_receiver_request_list(
    receiver: *const PamojaMavlinkMissionReceiver,
    header: PamojaMavlinkHeader,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    let Some(receiver) = receiver.as_ref() else {
        set_last_error("receiver must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match receiver.inner.request_list_frame(Header::from(header)) {
        Ok(frame) => emit(frame, out_frame),
        Err(error) => status_of(error),
    }
}

/// Handles an incoming frame, if it is one this transfer is waiting for.
///
/// A `MISSION_COUNT` opens the transfer and a `MISSION_ITEM_INT` advances it. Any other
/// message sets `out_kind` to [`PAMOJA_MAVLINK_STEP_IGNORED`] and produces nothing.
///
/// # Arguments
///
/// * `receiver` - the transfer.
/// * `frame` - the frame off the link.
/// * `header` - the addressing fields to stamp on the reply.
/// * `out_kind` - set to [`PAMOJA_MAVLINK_RECEIVER_REQUEST`],
///   [`PAMOJA_MAVLINK_RECEIVER_ACK`], or [`PAMOJA_MAVLINK_STEP_IGNORED`].
/// * `out_accepted` - set to the `MISSION_ITEM_INT` the frame carried if it was the one
///   expected next, as a message the caller reads by field name and releases with
///   `pamoja_mavlink_message_free`, or to null.
/// * `out_reply` - set to the frame to send back, which the caller releases with
///   `pamoja_mavlink_frame_free`, or to null if the frame was ignored.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, including when the frame was ignored.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if any pointer is null.
///
/// # Safety
///
/// `receiver` must be a live receiver handle, `frame` a live frame handle, and each out
/// pointer must point at writable storage for its value.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_mission_receiver_on_frame(
    receiver: *mut PamojaMavlinkMissionReceiver,
    frame: *const PamojaMavlinkFrame,
    header: PamojaMavlinkHeader,
    out_kind: *mut u32,
    out_accepted: *mut *mut PamojaMavlinkMessage,
    out_reply: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    let Some(receiver) = receiver.as_mut() else {
        set_last_error("receiver must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(frame) = frame.as_ref() else {
        set_last_error("frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if out_kind.is_null() || out_accepted.is_null() || out_reply.is_null() {
        set_last_error("the output pointers must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_kind = PAMOJA_MAVLINK_STEP_IGNORED;
    *out_accepted = std::ptr::null_mut();
    *out_reply = std::ptr::null_mut();

    let step = match receiver.inner.on_frame(frame.frame(), Header::from(header)) {
        Ok(Some(step)) => step,
        Ok(None) => return PamojaStatus::Ok,
        Err(error) => return status_of(error),
    };
    let ReceiverStep {
        accepted,
        action,
        reply,
    } = step;
    *out_kind = match action {
        ReceiverAction::Request(_) => PAMOJA_MAVLINK_RECEIVER_REQUEST,
        ReceiverAction::Ack(_) => PAMOJA_MAVLINK_RECEIVER_ACK,
    };
    if let Some(item) = accepted {
        let mut payload = [0u8; pamoja_mavlink::MAX_PAYLOAD];
        let len = item.encode(&mut payload);
        *out_accepted =
            PamojaMavlinkMessage::from_typed(MissionItemInt::DESCRIPTOR, payload[..len].to_vec());
    }
    *out_reply = PamojaMavlinkFrame::into_handle(reply);
    PamojaStatus::Ok
}

/// Reports whether the transfer has finished.
///
/// # Arguments
///
/// * `receiver` - the transfer.
///
/// # Returns
///
/// `1` once every item has been received and the acknowledgement produced, `0` otherwise or
/// if `receiver` is null.
///
/// # Safety
///
/// `receiver` must be a live receiver handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_mission_receiver_is_complete(
    receiver: *const PamojaMavlinkMissionReceiver,
) -> u8 {
    receiver
        .as_ref()
        .map_or(0, |receiver| u8::from(receiver.inner.is_complete()))
}

/// Returns the next sequence number the receiver expects.
///
/// # Arguments
///
/// * `receiver` - the transfer.
///
/// # Returns
///
/// The expected sequence number, or `0` if `receiver` is null.
///
/// # Safety
///
/// `receiver` must be a live receiver handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_mission_receiver_expected(
    receiver: *const PamojaMavlinkMissionReceiver,
) -> u16 {
    receiver
        .as_ref()
        .map_or(0, |receiver| receiver.inner.expected())
}

/// Releases a receiver.
///
/// # Arguments
///
/// * `receiver` - the handle to release; null is ignored.
///
/// # Safety
///
/// `receiver` must have come from [`pamoja_mavlink_mission_receiver_new`] and must not be
/// used afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_mission_receiver_free(
    receiver: *mut PamojaMavlinkMissionReceiver,
) {
    if !receiver.is_null() {
        drop(Box::from_raw(receiver));
    }
}

/// Holds a plan and answers a receiver's requests for its items.
pub struct PamojaMavlinkMissionSender {
    items: Vec<MissionItemInt>,
    target_system: u8,
    target_component: u8,
    mission_type: u8,
}

impl PamojaMavlinkMissionSender {
    /// Runs a query against the borrowed sender the engine defines.
    fn with<R>(&self, query: impl FnOnce(&MissionSender<'_>) -> R) -> R {
        query(&MissionSender::new(
            &self.items,
            self.target_system,
            self.target_component,
            self.mission_type,
        ))
    }
}

/// Creates a sender for a plan bound for a target vehicle, with no items yet.
///
/// # Arguments
///
/// * `target_system` - the receiving system's id.
/// * `target_component` - the receiving component's id.
/// * `mission_type` - the `MAV_MISSION_TYPE` of the plan.
///
/// # Returns
///
/// A sender the caller releases with [`pamoja_mavlink_mission_sender_free`].
#[no_mangle]
pub extern "C" fn pamoja_mavlink_mission_sender_new(
    target_system: u8,
    target_component: u8,
    mission_type: u8,
) -> *mut PamojaMavlinkMissionSender {
    Box::into_raw(Box::new(PamojaMavlinkMissionSender {
        items: Vec::new(),
        target_system,
        target_component,
        mission_type,
    }))
}

/// Appends an item to the plan.
///
/// The sender stamps the sequence number, target ids, and mission type onto each item as it
/// is handed out, so the payload need only carry the item's content: its command, frame,
/// position, and parameters. Build one by field name with the message schema for
/// `MISSION_ITEM_INT` and pass its payload.
///
/// # Arguments
///
/// * `sender` - the plan to extend.
/// * `payload` - a `MISSION_ITEM_INT` payload.
/// * `payload_len` - the payload length in bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, and
/// [`PamojaStatus::Codec`] if the payload does not form an item.
///
/// # Safety
///
/// `sender` must be a live sender handle and `payload` must point at `payload_len` readable
/// bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_mission_sender_add_item(
    sender: *mut PamojaMavlinkMissionSender,
    payload: *const u8,
    payload_len: usize,
) -> PamojaStatus {
    let Some(sender) = sender.as_mut() else {
        set_last_error("sender must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    match MissionItemInt::decode(&payload) {
        Ok(item) => {
            sender.items.push(item);
            PamojaStatus::Ok
        }
        Err(error) => status_of(error),
    }
}

/// Returns the number of items in the plan.
///
/// # Arguments
///
/// * `sender` - the plan.
///
/// # Returns
///
/// The item count, or `0` if `sender` is null.
///
/// # Safety
///
/// `sender` must be a live sender handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_mission_sender_len(
    sender: *const PamojaMavlinkMissionSender,
) -> u16 {
    sender
        .as_ref()
        .map_or(0, |sender| sender.with(|plan| plan.len()))
}

/// Builds the frame that opens an upload.
///
/// # Arguments
///
/// * `sender` - the plan.
/// * `header` - the addressing fields to stamp on the frame.
/// * `out_frame` - set to the `MISSION_COUNT` frame, which the caller releases with
///   `pamoja_mavlink_frame_free`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `sender` must be a live sender handle, and `out_frame` must point at writable storage for
/// one pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_mission_sender_count(
    sender: *const PamojaMavlinkMissionSender,
    header: PamojaMavlinkHeader,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    let Some(sender) = sender.as_ref() else {
        set_last_error("sender must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match sender.with(|plan| plan.count_frame(Header::from(header))) {
        Ok(frame) => emit(frame, out_frame),
        Err(error) => status_of(error),
    }
}

/// Handles an incoming frame, if it is one this transfer answers.
///
/// A `MISSION_REQUEST_LIST` is answered with the count, a `MISSION_REQUEST_INT` (or the
/// older `MISSION_REQUEST`) with the item asked for, and a request past the end of the plan
/// with a `MISSION_ACK` reporting an invalid sequence. A `MISSION_ACK` from the receiver
/// ends the transfer. Any other message sets `out_kind` to [`PAMOJA_MAVLINK_STEP_IGNORED`].
///
/// # Arguments
///
/// * `sender` - the plan.
/// * `frame` - the frame off the link.
/// * `header` - the addressing fields to stamp on the reply.
/// * `out_kind` - set to [`PAMOJA_MAVLINK_SENDER_REPLY`],
///   [`PAMOJA_MAVLINK_SENDER_FINISHED`], or [`PAMOJA_MAVLINK_STEP_IGNORED`].
/// * `out_result` - set to the receiver's `MAV_MISSION_RESULT` when the transfer finished.
/// * `out_reply` - set to the frame to send back, which the caller releases with
///   `pamoja_mavlink_frame_free`, or to null if there is nothing to send.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, including when the frame was ignored.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if any pointer is null.
///
/// # Safety
///
/// `sender` must be a live sender handle, `frame` a live frame handle, and each out pointer
/// must point at writable storage for its value.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_mission_sender_on_frame(
    sender: *const PamojaMavlinkMissionSender,
    frame: *const PamojaMavlinkFrame,
    header: PamojaMavlinkHeader,
    out_kind: *mut u32,
    out_result: *mut u8,
    out_reply: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    let Some(sender) = sender.as_ref() else {
        set_last_error("sender must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(frame) = frame.as_ref() else {
        set_last_error("frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if out_kind.is_null() || out_result.is_null() || out_reply.is_null() {
        set_last_error("the output pointers must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_kind = PAMOJA_MAVLINK_STEP_IGNORED;
    *out_result = 0;
    *out_reply = std::ptr::null_mut();

    match sender.with(|plan| plan.on_frame(frame.frame(), Header::from(header))) {
        Ok(Some(SenderStep::Reply(reply))) => {
            *out_kind = PAMOJA_MAVLINK_SENDER_REPLY;
            *out_reply = PamojaMavlinkFrame::into_handle(reply);
            PamojaStatus::Ok
        }
        Ok(Some(SenderStep::Finished(result))) => {
            *out_kind = PAMOJA_MAVLINK_SENDER_FINISHED;
            *out_result = result;
            PamojaStatus::Ok
        }
        Ok(None) => PamojaStatus::Ok,
        Err(error) => status_of(error),
    }
}

/// Releases a sender.
///
/// # Arguments
///
/// * `sender` - the handle to release; null is ignored.
///
/// # Safety
///
/// `sender` must have come from [`pamoja_mavlink_mission_sender_new`] and must not be used
/// afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_mission_sender_free(
    sender: *mut PamojaMavlinkMissionSender,
) {
    if !sender.is_null() {
        drop(Box::from_raw(sender));
    }
}

/// Tracks one command awaiting its acknowledgement.
pub struct PamojaMavlinkCommand {
    inner: CommandProtocol,
}

/// Starts tracking a command.
///
/// # Arguments
///
/// * `command` - the `MAV_CMD` id being sent.
/// * `max_retries` - how many times the command may be resent after a timeout before the
///   caller gives up; [`PAMOJA_MAVLINK_MAX_RETRIES`] is the usual choice.
///
/// # Returns
///
/// A tracker the caller releases with [`pamoja_mavlink_command_free`].
#[no_mangle]
pub extern "C" fn pamoja_mavlink_command_new(
    command: u16,
    max_retries: u8,
) -> *mut PamojaMavlinkCommand {
    Box::into_raw(Box::new(PamojaMavlinkCommand {
        inner: CommandProtocol::new(command, max_retries),
    }))
}

/// Returns the command id being tracked.
///
/// # Arguments
///
/// * `command` - the tracker.
///
/// # Returns
///
/// The command id, or `0` if `command` is null.
///
/// # Safety
///
/// `command` must be a live tracker handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_command_id(command: *const PamojaMavlinkCommand) -> u16 {
    command
        .as_ref()
        .map_or(0, |command| command.inner.command())
}

/// Returns the `confirmation` count to stamp on the command being sent.
///
/// It is zero for the first transmission and increments on each retransmission, which is
/// how an autopilot distinguishes a resend from a new command.
///
/// # Arguments
///
/// * `command` - the tracker.
///
/// # Returns
///
/// The current confirmation count, or `0` if `command` is null.
///
/// # Safety
///
/// `command` must be a live tracker handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_command_confirmation(
    command: *const PamojaMavlinkCommand,
) -> u8 {
    command
        .as_ref()
        .map_or(0, |command| command.inner.confirmation())
}

/// Classifies an incoming frame against the command in flight.
///
/// # Arguments
///
/// * `command` - the tracker.
/// * `frame` - the frame off the link.
/// * `out_kind` - set to [`PAMOJA_MAVLINK_ACK_UNRELATED`], [`PAMOJA_MAVLINK_ACK_IN_PROGRESS`],
///   [`PAMOJA_MAVLINK_ACK_FINAL`], or [`PAMOJA_MAVLINK_STEP_IGNORED`] if the frame is not a
///   `COMMAND_ACK`.
/// * `out_value` - set to the progress percent when in progress, or the `MAV_RESULT` when
///   final.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, including when the frame was ignored.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if any pointer is null.
///
/// # Safety
///
/// `command` must be a live tracker handle, `frame` a live frame handle, and each out
/// pointer must point at writable storage for its value.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_command_on_frame(
    command: *const PamojaMavlinkCommand,
    frame: *const PamojaMavlinkFrame,
    out_kind: *mut u32,
    out_value: *mut u8,
) -> PamojaStatus {
    let Some(command) = command.as_ref() else {
        set_last_error("command must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(frame) = frame.as_ref() else {
        set_last_error("frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if out_kind.is_null() || out_value.is_null() {
        set_last_error("the output pointers must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_kind = PAMOJA_MAVLINK_STEP_IGNORED;
    *out_value = 0;
    match command.inner.on_frame(frame.frame()) {
        Ok(Some(AckOutcome::Unrelated)) => *out_kind = PAMOJA_MAVLINK_ACK_UNRELATED,
        Ok(Some(AckOutcome::InProgress(progress))) => {
            *out_kind = PAMOJA_MAVLINK_ACK_IN_PROGRESS;
            *out_value = progress;
        }
        Ok(Some(AckOutcome::Final(result))) => {
            *out_kind = PAMOJA_MAVLINK_ACK_FINAL;
            *out_value = result;
        }
        Ok(None) => {}
        Err(error) => return status_of(error),
    }
    PamojaStatus::Ok
}

/// Records a timeout and reports whether the command may be resent.
///
/// On a resend the `confirmation` count is incremented, so the next call to
/// [`pamoja_mavlink_command_confirmation`] stamps the new value.
///
/// # Arguments
///
/// * `command` - the tracker.
/// * `out_confirmation` - set to the new confirmation count when a resend is allowed.
///
/// # Returns
///
/// `1` if a retry remains and the command should be resent, `0` once the retry budget is
/// exhausted or if a pointer is null.
///
/// # Safety
///
/// `command` must be a live tracker handle, and `out_confirmation` must point at writable
/// storage for one byte.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_command_on_timeout(
    command: *mut PamojaMavlinkCommand,
    out_confirmation: *mut u8,
) -> u8 {
    let Some(command) = command.as_mut() else {
        return 0;
    };
    if out_confirmation.is_null() {
        return 0;
    }
    match command.inner.on_timeout() {
        Some(confirmation) => {
            *out_confirmation = confirmation;
            1
        }
        None => 0,
    }
}

/// Releases a command tracker.
///
/// # Arguments
///
/// * `command` - the handle to release; null is ignored.
///
/// # Safety
///
/// `command` must have come from [`pamoja_mavlink_command_new`] and must not be used
/// afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_command_free(command: *mut PamojaMavlinkCommand) {
    if !command.is_null() {
        drop(Box::from_raw(command));
    }
}

/// Builds a setpoint `type_mask` from the fields to use.
///
/// A setpoint carries position, velocity, acceleration, yaw, and yaw rate together; the mask
/// says which of them the autopilot should act on. Fields left out of `flags` are ignored.
///
/// # Arguments
///
/// * `flags` - a bitwise-or of the `PAMOJA_MAVLINK_TYPEMASK_*` flags.
///
/// # Returns
///
/// The mask, as the `type_mask` field of a setpoint carries it.
#[no_mangle]
pub extern "C" fn pamoja_mavlink_offboard_type_mask(flags: u32) -> u16 {
    let mut mask = TypeMask::ignore_all();
    if flags & PAMOJA_MAVLINK_TYPEMASK_POSITION != 0 {
        mask = mask.use_position();
    }
    if flags & PAMOJA_MAVLINK_TYPEMASK_VELOCITY != 0 {
        mask = mask.use_velocity();
    }
    if flags & PAMOJA_MAVLINK_TYPEMASK_ACCELERATION != 0 {
        mask = mask.use_acceleration();
    }
    if flags & PAMOJA_MAVLINK_TYPEMASK_YAW != 0 {
        mask = mask.use_yaw();
    }
    if flags & PAMOJA_MAVLINK_TYPEMASK_YAW_RATE != 0 {
        mask = mask.use_yaw_rate();
    }
    if flags & PAMOJA_MAVLINK_TYPEMASK_FORCE != 0 {
        mask = mask.force();
    }
    mask.bits()
}

/// Builds a local-frame position setpoint frame.
///
/// # Arguments
///
/// * `header` - the addressing fields to stamp on the frame.
/// * `time_boot_ms` - the sender's boot timestamp, in milliseconds.
/// * `coordinate_frame` - the `MAV_FRAME` of the setpoint.
/// * `target_system` - the target system id.
/// * `target_component` - the target component id.
/// * `x`, `y`, `z` - the position, in metres in the chosen frame.
/// * `out_frame` - set to the `SET_POSITION_TARGET_LOCAL_NED` frame, which the caller
///   releases with `pamoja_mavlink_frame_free`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_frame` is null.
///
/// # Safety
///
/// `out_frame` must point at writable storage for one pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_mavlink_offboard_local_position(
    header: PamojaMavlinkHeader,
    time_boot_ms: u32,
    coordinate_frame: u8,
    target_system: u8,
    target_component: u8,
    x: f32,
    y: f32,
    z: f32,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    let setpoint = SetPositionTargetLocalNed::position(
        time_boot_ms,
        coordinate_frame,
        target_system,
        target_component,
        x,
        y,
        z,
    );
    match dialect::encode_message(Header::from(header), &setpoint) {
        Ok(frame) => emit(frame, out_frame),
        Err(error) => status_of(error),
    }
}

/// Builds a local-frame velocity setpoint frame.
///
/// # Arguments
///
/// * `header` - the addressing fields to stamp on the frame.
/// * `time_boot_ms` - the sender's boot timestamp, in milliseconds.
/// * `coordinate_frame` - the `MAV_FRAME` of the setpoint.
/// * `target_system` - the target system id.
/// * `target_component` - the target component id.
/// * `vx`, `vy`, `vz` - the velocity, in metres per second in the chosen frame.
/// * `out_frame` - set to the `SET_POSITION_TARGET_LOCAL_NED` frame, which the caller
///   releases with `pamoja_mavlink_frame_free`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_frame` is null.
///
/// # Safety
///
/// `out_frame` must point at writable storage for one pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_mavlink_offboard_local_velocity(
    header: PamojaMavlinkHeader,
    time_boot_ms: u32,
    coordinate_frame: u8,
    target_system: u8,
    target_component: u8,
    vx: f32,
    vy: f32,
    vz: f32,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    let setpoint = SetPositionTargetLocalNed::velocity(
        time_boot_ms,
        coordinate_frame,
        target_system,
        target_component,
        vx,
        vy,
        vz,
    );
    match dialect::encode_message(Header::from(header), &setpoint) {
        Ok(frame) => emit(frame, out_frame),
        Err(error) => status_of(error),
    }
}

/// Builds a global-frame position setpoint frame.
///
/// # Arguments
///
/// * `header` - the addressing fields to stamp on the frame.
/// * `time_boot_ms` - the sender's boot timestamp, in milliseconds.
/// * `coordinate_frame` - the `MAV_FRAME` of the setpoint.
/// * `target_system` - the target system id.
/// * `target_component` - the target component id.
/// * `lat_int`, `lon_int` - the latitude and longitude, in degrees times ten million.
/// * `alt` - the altitude, in metres.
/// * `out_frame` - set to the `SET_POSITION_TARGET_GLOBAL_INT` frame, which the caller
///   releases with `pamoja_mavlink_frame_free`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_frame` is null.
///
/// # Safety
///
/// `out_frame` must point at writable storage for one pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_mavlink_offboard_global_position(
    header: PamojaMavlinkHeader,
    time_boot_ms: u32,
    coordinate_frame: u8,
    target_system: u8,
    target_component: u8,
    lat_int: i32,
    lon_int: i32,
    alt: f32,
    out_frame: *mut *mut PamojaMavlinkFrame,
) -> PamojaStatus {
    let setpoint = SetPositionTargetGlobalInt::position(
        time_boot_ms,
        coordinate_frame,
        target_system,
        target_component,
        lat_int,
        lon_int,
        alt,
    );
    match dialect::encode_message(Header::from(header), &setpoint) {
        Ok(frame) => emit(frame, out_frame),
        Err(error) => status_of(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mavlink::{pamoja_mavlink_frame_free, pamoja_mavlink_frame_message_id};
    use crate::mavlink_schema::pamoja_mavlink_message_free;
    use pamoja_mavlink::dialect::{mav_cmd, mav_mission_result, mav_result, CommandAck};

    const VEHICLE: PamojaMavlinkHeader = PamojaMavlinkHeader {
        system_id: 1,
        component_id: 1,
        sequence: 0,
    };
    const STATION: PamojaMavlinkHeader = PamojaMavlinkHeader {
        system_id: 255,
        component_id: 190,
        sequence: 0,
    };

    fn item_payload(command: u16, z: f32) -> Vec<u8> {
        let item = MissionItemInt {
            command,
            z,
            ..MissionItemInt::zeroed()
        };
        let mut payload = [0u8; pamoja_mavlink::MAX_PAYLOAD];
        let len = item.encode(&mut payload);
        payload[..len].to_vec()
    }

    #[test]
    fn a_whole_upload_runs_across_the_boundary() {
        unsafe {
            let sender = pamoja_mavlink_mission_sender_new(1, 1, 0);
            for (command, z) in [(mav_cmd::NAV_TAKEOFF, 20.0), (mav_cmd::NAV_WAYPOINT, 50.0)] {
                let payload = item_payload(command, z);
                assert_eq!(
                    pamoja_mavlink_mission_sender_add_item(sender, payload.as_ptr(), payload.len()),
                    PamojaStatus::Ok
                );
            }
            assert_eq!(pamoja_mavlink_mission_sender_len(sender), 2);

            let receiver = pamoja_mavlink_mission_receiver_new(255, 190, 0);
            let mut opened = std::ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_mission_receiver_request_list(receiver, STATION, &mut opened),
                PamojaStatus::Ok
            );

            let mut kind = 0;
            let mut result = 0;
            let mut from_vehicle = std::ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_mission_sender_on_frame(
                    sender,
                    opened,
                    VEHICLE,
                    &mut kind,
                    &mut result,
                    &mut from_vehicle
                ),
                PamojaStatus::Ok
            );
            pamoja_mavlink_frame_free(opened);
            assert_eq!(kind, PAMOJA_MAVLINK_SENDER_REPLY);
            assert_eq!(pamoja_mavlink_frame_message_id(from_vehicle), 44);

            let mut accepted_items = 0;
            loop {
                let mut accepted = std::ptr::null_mut();
                let mut reply = std::ptr::null_mut();
                assert_eq!(
                    pamoja_mavlink_mission_receiver_on_frame(
                        receiver,
                        from_vehicle,
                        STATION,
                        &mut kind,
                        &mut accepted,
                        &mut reply
                    ),
                    PamojaStatus::Ok
                );
                pamoja_mavlink_frame_free(from_vehicle);
                assert_ne!(kind, PAMOJA_MAVLINK_STEP_IGNORED);
                if !accepted.is_null() {
                    accepted_items += 1;
                    pamoja_mavlink_message_free(accepted);
                }

                let mut next = std::ptr::null_mut();
                assert_eq!(
                    pamoja_mavlink_mission_sender_on_frame(
                        sender,
                        reply,
                        VEHICLE,
                        &mut kind,
                        &mut result,
                        &mut next
                    ),
                    PamojaStatus::Ok
                );
                pamoja_mavlink_frame_free(reply);
                if kind == PAMOJA_MAVLINK_SENDER_FINISHED {
                    assert_eq!(result, mav_mission_result::ACCEPTED);
                    break;
                }
                assert_eq!(kind, PAMOJA_MAVLINK_SENDER_REPLY);
                from_vehicle = next;
            }
            assert_eq!(accepted_items, 2);
            assert_eq!(pamoja_mavlink_mission_receiver_is_complete(receiver), 1);

            pamoja_mavlink_mission_receiver_free(receiver);
            pamoja_mavlink_mission_sender_free(sender);
        }
    }

    #[test]
    fn a_command_is_matched_to_its_acknowledgement_and_retried() {
        unsafe {
            let arm = pamoja_mavlink_command_new(mav_cmd::COMPONENT_ARM_DISARM, 2);
            assert_eq!(
                pamoja_mavlink_command_id(arm),
                mav_cmd::COMPONENT_ARM_DISARM
            );
            assert_eq!(pamoja_mavlink_command_confirmation(arm), 0);

            let ack = CommandAck {
                command: mav_cmd::COMPONENT_ARM_DISARM,
                result: mav_result::ACCEPTED,
                ..CommandAck::zeroed()
            };
            let frame = PamojaMavlinkFrame::into_handle(
                dialect::encode_message(Header::from(VEHICLE), &ack).unwrap(),
            );
            let mut kind = 0;
            let mut value = 0;
            assert_eq!(
                pamoja_mavlink_command_on_frame(arm, frame, &mut kind, &mut value),
                PamojaStatus::Ok
            );
            assert_eq!(kind, PAMOJA_MAVLINK_ACK_FINAL);
            assert_eq!(value, mav_result::ACCEPTED);
            pamoja_mavlink_frame_free(frame);

            // Two retries, then the budget is spent.
            let mut confirmation = 0;
            assert_eq!(pamoja_mavlink_command_on_timeout(arm, &mut confirmation), 1);
            assert_eq!(confirmation, 1);
            assert_eq!(pamoja_mavlink_command_on_timeout(arm, &mut confirmation), 1);
            assert_eq!(confirmation, 2);
            assert_eq!(pamoja_mavlink_command_on_timeout(arm, &mut confirmation), 0);
            pamoja_mavlink_command_free(arm);
        }
    }

    #[test]
    fn a_setpoint_goes_out_as_the_right_message() {
        unsafe {
            let mut frame = std::ptr::null_mut();
            assert_eq!(
                pamoja_mavlink_offboard_local_velocity(
                    STATION, 1_000, 1, 1, 1, 0.5, 0.0, 0.0, &mut frame
                ),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_mavlink_frame_message_id(frame), 84);
            pamoja_mavlink_frame_free(frame);

            assert_eq!(
                pamoja_mavlink_offboard_global_position(
                    STATION,
                    1_000,
                    6,
                    1,
                    1,
                    -338_567_800,
                    1_512_153_000,
                    50.0,
                    &mut frame
                ),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_mavlink_frame_message_id(frame), 86);
            pamoja_mavlink_frame_free(frame);

            let mask = pamoja_mavlink_offboard_type_mask(
                PAMOJA_MAVLINK_TYPEMASK_VELOCITY | PAMOJA_MAVLINK_TYPEMASK_YAW_RATE,
            );
            assert_eq!(
                mask,
                TypeMask::ignore_all().use_velocity().use_yaw_rate().bits()
            );
        }
    }
}
