//! Generated Python bindings for the MAVLink service protocols: mission transfer,
//! commands, and offboard setpoints.
//!
//! A frame carries one message; a real exchange with an autopilot is a sequence
//! of them with rules about order, matching, and retransmission. The machines
//! here hold those rules and nothing else: no IO, no timers. A caller feeds each
//! one the frames off its link and sends back what the machine hands it,
//! applying its own timing policy for timeouts.
//!
//! Every machine takes a whole frame and answers with a whole frame, so the
//! payload decoding, message-id dispatch, and reply encoding happen once, in the
//! engine. A frame a machine does not handle comes back as `None` rather than as
//! an error, so one link's traffic can be routed through several machines.

use std::sync::Mutex;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_mavlink::dialect::{
    encode_message, Message, MissionItemInt, SetPositionTargetGlobalInt, SetPositionTargetLocalNed,
};
use pamoja_mavlink::protocol::{
    AckOutcome as CoreAckOutcome, CommandProtocol as CoreCommand, MissionReceiver as CoreReceiver,
    MissionSender as CoreSender, ReceiverAction, SenderStep as CoreSenderStep, TypeMask,
    MAX_RETRIES,
};
use pamoja_mavlink::{Frame as CoreFrame, Header, MavlinkError};

use crate::mavlink::{MavlinkFrame, MavlinkHeader};
use crate::mavlink_schema::MavlinkMessage;

/// Turns a MAVLink error into the exception a caller sees.
fn error_of(error: MavlinkError) -> PyErr {
    PyValueError::new_err(error.to_string())
}

/// What one incoming frame produced for a mission receiver.
#[gen_stub_pyclass]
#[pyclass]
pub struct ReceiverStep {
    kind: &'static str,
    accepted: Option<Vec<u8>>,
    reply: CoreFrame,
}

#[gen_stub_pymethods]
#[pymethods]
impl ReceiverStep {
    /// What the receiver answered with: ``"request"`` for the next item, or
    /// ``"ack"`` to end the transfer.
    #[getter]
    fn kind(&self) -> String {
        self.kind.to_owned()
    }

    /// The ``MISSION_ITEM_INT`` the frame carried, if it was the one expected
    /// next, as a message read by field name.
    #[getter]
    fn accepted(&self) -> Option<MavlinkMessage> {
        self.accepted
            .as_ref()
            .map(|payload| MavlinkMessage::from_typed(MissionItemInt::DESCRIPTOR, payload.clone()))
    }

    /// The frame to send back.
    #[getter]
    fn reply(&self) -> MavlinkFrame {
        MavlinkFrame::from_frame(self.reply)
    }

    /// Returns a readable form for logs and the interpreter.
    fn __repr__(&self) -> String {
        format!(
            "ReceiverStep(kind={:?}, accepted={})",
            self.kind,
            self.accepted.is_some()
        )
    }
}

/// Requests a plan's items in order and collects them, ending with an
/// acknowledgement.
#[gen_stub_pyclass]
#[pyclass]
pub struct MissionReceiver {
    inner: Mutex<CoreReceiver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MissionReceiver {
    /// Creates a receiver for a plan from a target vehicle, identified by system
    /// and component id, for one ``MAV_MISSION_TYPE``.
    #[new]
    #[pyo3(signature = (target_system, target_component, mission_type = 0))]
    fn new(target_system: u8, target_component: u8, mission_type: u8) -> Self {
        Self {
            inner: Mutex::new(CoreReceiver::new(
                target_system,
                target_component,
                mission_type,
            )),
        }
    }

    /// Builds the ``MISSION_REQUEST_LIST`` frame that starts a download.
    fn request_list(&self, header: MavlinkHeader) -> PyResult<MavlinkFrame> {
        let inner = self
            .inner
            .lock()
            .expect("the receiver lock is never poisoned");
        inner
            .request_list_frame(Header::from(header))
            .map(MavlinkFrame::from_frame)
            .map_err(error_of)
    }

    /// Handles an incoming frame, if it is one this transfer is waiting for.
    ///
    /// A ``MISSION_COUNT`` opens the transfer and a ``MISSION_ITEM_INT``
    /// advances it. Returns the step taken, or ``None`` if the frame carries a
    /// message this transfer does not handle.
    fn on_frame(
        &self,
        frame: &MavlinkFrame,
        header: MavlinkHeader,
    ) -> PyResult<Option<ReceiverStep>> {
        let mut inner = self
            .inner
            .lock()
            .expect("the receiver lock is never poisoned");
        let step = inner
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
    #[getter]
    fn complete(&self) -> bool {
        self.inner
            .lock()
            .expect("the receiver lock is never poisoned")
            .is_complete()
    }

    /// The next sequence number the receiver expects.
    #[getter]
    fn expected(&self) -> u16 {
        self.inner
            .lock()
            .expect("the receiver lock is never poisoned")
            .expected()
    }
}

/// What one incoming frame produced for a mission sender.
#[gen_stub_pyclass]
#[pyclass]
pub struct SenderStep {
    kind: &'static str,
    reply: Option<CoreFrame>,
    result: Option<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl SenderStep {
    /// What happened: ``"reply"`` when there is a frame to send, or
    /// ``"finished"`` when the receiver acknowledged the transfer.
    #[getter]
    fn kind(&self) -> String {
        self.kind.to_owned()
    }

    /// The frame to send back, when there is one.
    #[getter]
    fn reply(&self) -> Option<MavlinkFrame> {
        self.reply.map(MavlinkFrame::from_frame)
    }

    /// The receiver's ``MAV_MISSION_RESULT``, when the transfer finished.
    #[getter]
    fn result(&self) -> Option<u8> {
        self.result
    }

    /// Returns a readable form for logs and the interpreter.
    fn __repr__(&self) -> String {
        format!("SenderStep(kind={:?}, result={:?})", self.kind, self.result)
    }
}

/// Holds a plan and answers a receiver's requests for its items.
#[gen_stub_pyclass]
#[pyclass]
pub struct MissionSender {
    items: Mutex<Vec<MissionItemInt>>,
    target_system: u8,
    target_component: u8,
    mission_type: u8,
}

impl MissionSender {
    fn with<R>(&self, query: impl FnOnce(&CoreSender<'_>) -> R) -> R {
        let items = self.items.lock().expect("the plan lock is never poisoned");
        query(&CoreSender::new(
            &items,
            self.target_system,
            self.target_component,
            self.mission_type,
        ))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl MissionSender {
    /// Creates a sender for a plan bound for a target vehicle, with no items
    /// yet.
    #[new]
    #[pyo3(signature = (target_system, target_component, mission_type = 0))]
    fn new(target_system: u8, target_component: u8, mission_type: u8) -> Self {
        Self {
            items: Mutex::new(Vec::new()),
            target_system,
            target_component,
            mission_type,
        }
    }

    /// Appends an item to the plan, from a ``MISSION_ITEM_INT`` payload.
    ///
    /// The sender stamps the sequence number, target ids, and mission type onto
    /// each item as it is handed out, so the item need only carry its content:
    /// command, frame, position, and parameters. Build one by field name with the
    /// ``MISSION_ITEM_INT`` schema and pass its payload.
    fn add_item(&self, item: Vec<u8>) -> PyResult<()> {
        let decoded = MissionItemInt::decode(&item).map_err(error_of)?;
        self.items
            .lock()
            .expect("the plan lock is never poisoned")
            .push(decoded);
        Ok(())
    }

    /// Returns the number of items in the plan.
    fn __len__(&self) -> usize {
        self.with(|plan| plan.len()) as usize
    }

    /// Builds the ``MISSION_COUNT`` frame that opens an upload.
    fn count(&self, header: MavlinkHeader) -> PyResult<MavlinkFrame> {
        self.with(|plan| plan.count_frame(Header::from(header)))
            .map(MavlinkFrame::from_frame)
            .map_err(error_of)
    }

    /// Handles an incoming frame, if it is one this transfer answers.
    ///
    /// A ``MISSION_REQUEST_LIST`` is answered with the count, a
    /// ``MISSION_REQUEST_INT`` (or the older ``MISSION_REQUEST``) with the item
    /// asked for, and a request past the end of the plan with a ``MISSION_ACK``
    /// reporting an invalid sequence. A ``MISSION_ACK`` from the receiver ends
    /// the transfer. Returns ``None`` for a message this transfer does not
    /// handle.
    fn on_frame(
        &self,
        frame: &MavlinkFrame,
        header: MavlinkHeader,
    ) -> PyResult<Option<SenderStep>> {
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
#[gen_stub_pyclass]
#[pyclass]
pub struct AckOutcome {
    /// ``"unrelated"`` if the ack was for another command, ``"in_progress"`` if
    /// the command is still running, or ``"final"`` if it finished.
    #[pyo3(get)]
    kind: String,
    /// The progress percent when in progress (255 when the autopilot does not
    /// report one), or the ``MAV_RESULT`` when final.
    #[pyo3(get)]
    value: Option<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl AckOutcome {
    /// Returns a readable form for logs and the interpreter.
    fn __repr__(&self) -> String {
        format!("AckOutcome(kind={:?}, value={:?})", self.kind, self.value)
    }
}

/// Tracks one command awaiting its acknowledgement.
#[gen_stub_pyclass]
#[pyclass]
pub struct CommandProtocol {
    inner: Mutex<CoreCommand>,
}

#[gen_stub_pymethods]
#[pymethods]
impl CommandProtocol {
    /// Starts tracking a ``MAV_CMD``, allowing ``max_retries`` retransmissions
    /// after a timeout before the caller gives up.
    #[new]
    #[pyo3(signature = (command, max_retries = MAX_RETRIES))]
    fn new(command: u16, max_retries: u8) -> Self {
        Self {
            inner: Mutex::new(CoreCommand::new(command, max_retries)),
        }
    }

    /// The command id being tracked.
    #[getter]
    fn command(&self) -> u16 {
        self.inner
            .lock()
            .expect("the command lock is never poisoned")
            .command()
    }

    /// The ``confirmation`` count to stamp on the command being sent: zero for
    /// the first transmission, incremented on each retransmission.
    #[getter]
    fn confirmation(&self) -> u8 {
        self.inner
            .lock()
            .expect("the command lock is never poisoned")
            .confirmation()
    }

    /// Classifies an incoming frame against the command in flight.
    ///
    /// Returns the outcome, or ``None`` if the frame is not a ``COMMAND_ACK``.
    fn on_frame(&self, frame: &MavlinkFrame) -> PyResult<Option<AckOutcome>> {
        let inner = self
            .inner
            .lock()
            .expect("the command lock is never poisoned");
        let outcome = inner.on_frame(frame.frame()).map_err(error_of)?;
        Ok(outcome.map(|outcome| match outcome {
            CoreAckOutcome::Unrelated => AckOutcome {
                kind: "unrelated".to_owned(),
                value: None,
            },
            CoreAckOutcome::InProgress(progress) => AckOutcome {
                kind: "in_progress".to_owned(),
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
    /// Returns the new confirmation count to stamp on the resend, or ``None``
    /// once the retry budget is exhausted.
    fn on_timeout(&self) -> Option<u8> {
        self.inner
            .lock()
            .expect("the command lock is never poisoned")
            .on_timeout()
    }
}

/// Builds a setpoint ``type_mask`` from the fields to use.
///
/// ``flags`` is a bitwise-or of the ``TypeMask`` flags; fields left out are
/// ignored. Returns the mask as the ``type_mask`` field of a setpoint carries it.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_offboard_type_mask(flags: u32) -> u16 {
    let mut mask = TypeMask::ignore_all();
    if flags & 1 != 0 {
        mask = mask.use_position();
    }
    if flags & 2 != 0 {
        mask = mask.use_velocity();
    }
    if flags & 4 != 0 {
        mask = mask.use_acceleration();
    }
    if flags & 8 != 0 {
        mask = mask.use_yaw();
    }
    if flags & 16 != 0 {
        mask = mask.use_yaw_rate();
    }
    if flags & 32 != 0 {
        mask = mask.force();
    }
    mask.bits()
}

/// Builds a local-frame position setpoint frame, in metres in the chosen
/// ``MAV_FRAME``.
#[gen_stub_pyfunction]
#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn mavlink_offboard_local_position(
    header: MavlinkHeader,
    time_boot_ms: u32,
    coordinate_frame: u8,
    target_system: u8,
    target_component: u8,
    x: f32,
    y: f32,
    z: f32,
) -> PyResult<MavlinkFrame> {
    let setpoint = SetPositionTargetLocalNed::position(
        time_boot_ms,
        coordinate_frame,
        target_system,
        target_component,
        x,
        y,
        z,
    );
    encode_message(Header::from(header), &setpoint)
        .map(MavlinkFrame::from_frame)
        .map_err(error_of)
}

/// Builds a local-frame velocity setpoint frame, in metres per second in the
/// chosen ``MAV_FRAME``.
#[gen_stub_pyfunction]
#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn mavlink_offboard_local_velocity(
    header: MavlinkHeader,
    time_boot_ms: u32,
    coordinate_frame: u8,
    target_system: u8,
    target_component: u8,
    vx: f32,
    vy: f32,
    vz: f32,
) -> PyResult<MavlinkFrame> {
    let setpoint = SetPositionTargetLocalNed::velocity(
        time_boot_ms,
        coordinate_frame,
        target_system,
        target_component,
        vx,
        vy,
        vz,
    );
    encode_message(Header::from(header), &setpoint)
        .map(MavlinkFrame::from_frame)
        .map_err(error_of)
}

/// Builds a global-frame position setpoint frame, with latitude and longitude
/// in degrees times ten million and altitude in metres.
#[gen_stub_pyfunction]
#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn mavlink_offboard_global_position(
    header: MavlinkHeader,
    time_boot_ms: u32,
    coordinate_frame: u8,
    target_system: u8,
    target_component: u8,
    lat_int: i32,
    lon_int: i32,
    alt: f32,
) -> PyResult<MavlinkFrame> {
    let setpoint = SetPositionTargetGlobalInt::position(
        time_boot_ms,
        coordinate_frame,
        target_system,
        target_component,
        lat_int,
        lon_int,
        alt,
    );
    encode_message(Header::from(header), &setpoint)
        .map(MavlinkFrame::from_frame)
        .map_err(error_of)
}
