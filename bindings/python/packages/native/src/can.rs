//! Generated Python bindings for CAN bus framing.
//!
//! These mirror the `pamoja-can` Rust API: classic CAN 2.0 and CAN-FD frames, the
//! length encoding CAN-FD uses above eight bytes, and the J1939 identifier that
//! trucks, tractors, and gensets ride on top of it.
//!
//! A frame is a small value rather than a resource, so it crosses as a read-only
//! object; a decoded J1939 identifier does the same, with a `destination` of
//! `None` for a broadcast rather than a flag to check first.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_can::{
    dlc_to_len, len_to_dlc, priority, CanError, CanId, Frame, J1939Id, Signals, BROADCAST_ADDRESS,
    NOT_AVAILABLE,
};

use crate::PamojaError;

/// A CAN frame: an identifier, its flags, and its payload.
#[gen_stub_pyclass]
#[pyclass]
pub struct CanFrame {
    /// The arbitration identifier, already masked to 11 or 29 bits.
    #[pyo3(get)]
    id: u32,
    /// Whether the identifier is a 29-bit extended one.
    #[pyo3(get)]
    extended: bool,
    /// Whether this is a CAN-FD frame rather than classic CAN 2.0.
    #[pyo3(get)]
    fd: bool,
    /// Whether this is a remote transmission request, which carries no payload.
    #[pyo3(get)]
    remote: bool,
    /// The data length: the payload length, or the length a remote frame requests.
    #[pyo3(get)]
    len: usize,
    /// The data length code as it appears on the wire.
    #[pyo3(get)]
    dlc: u8,
    /// The payload, empty for a remote frame.
    data: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl CanFrame {
    /// The payload, empty for a remote frame.
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data)
    }
}

/// The fields J1939 packs into an extended CAN identifier.
#[gen_stub_pyclass]
#[pyclass]
pub struct J1939Message {
    /// The parameter group number, which names what the message carries.
    #[pyo3(get)]
    pgn: u32,
    /// The message priority, 0 (highest) to 7.
    #[pyo3(get)]
    priority: u8,
    /// The source address: the node that sent the message.
    #[pyo3(get)]
    source: u8,
    /// The PDU format byte of the parameter group.
    #[pyo3(get)]
    pdu_format: u8,
    /// The destination address for an addressed (PDU1) message, or `None` for a
    /// broadcast (PDU2) one.
    #[pyo3(get)]
    destination: Option<u8>,
    /// Whether the message is a broadcast.
    #[pyo3(get)]
    broadcast: bool,
}

/// Builds a classic CAN 2.0 frame, which carries up to eight bytes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn can_frame(id: u32, extended: bool, data: Vec<u8>) -> PyResult<CanFrame> {
    Frame::new(identifier(id, extended), &data)
        .map(describe)
        .map_err(to_py)
}

/// Builds a CAN-FD frame, which carries up to 64 bytes at the discrete CAN-FD lengths.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn can_fd_frame(id: u32, extended: bool, data: Vec<u8>) -> PyResult<CanFrame> {
    Frame::fd(identifier(id, extended), &data)
        .map(describe)
        .map_err(to_py)
}

/// Builds a remote transmission request, which asks another node to send.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn can_remote_frame(id: u32, extended: bool, len: usize) -> CanFrame {
    describe(Frame::remote(identifier(id, extended), len))
}

/// Returns the data length code that encodes a payload length.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn can_len_to_dlc(len: usize) -> u8 {
    len_to_dlc(len)
}

/// Returns the payload length a data length code encodes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn can_dlc_to_len(dlc: u8) -> usize {
    dlc_to_len(dlc)
}

/// Decodes the J1939 fields out of an extended CAN identifier.
///
/// Returns `None` for a standard 11-bit identifier, which J1939 does not use.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn j1939_decode(id: u32, extended: bool) -> Option<J1939Message> {
    J1939Id::from_id(identifier(id, extended)).map(|message| J1939Message {
        pgn: message.pgn(),
        priority: message.priority(),
        source: message.source(),
        pdu_format: message.pdu_format(),
        destination: message.destination(),
        broadcast: message.is_broadcast(),
    })
}

/// Composes the extended CAN identifier a set of J1939 fields describes.
///
/// The destination is used only for an addressed (PDU1) parameter group and
/// ignored for a broadcast (PDU2) one.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn j1939_compose(priority: u8, pgn: u32, source: u8, destination: u8) -> u32 {
    J1939Id::from_parts(priority, pgn, source, destination)
        .to_id()
        .raw()
}

/// Composes the identifier of a J1939 broadcast, which every node on the bus reads.
///
/// Most parameter groups are broadcast, so this is the common case; it saves a
/// caller knowing that a broadcast is addressed to `0xFF`.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn j1939_broadcast(priority: u8, pgn: u32, source: u8) -> u32 {
    J1939Id::broadcast(priority, pgn, source).to_id().raw()
}

/// Returns the named values J1939 publishes.
///
/// The order is the not-available byte, the broadcast address, and the control,
/// default, and lowest priorities.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn j1939_limits() -> (u8, u8, u8, u8, u8) {
    (
        NOT_AVAILABLE,
        BROADCAST_ADDRESS,
        priority::CONTROL,
        priority::DEFAULT,
        priority::LOWEST,
    )
}

/// The eight data bytes of a J1939 frame, addressed by the signals inside them.
///
/// A parameter group places each signal at a fixed byte offset, little-endian. A
/// payload starts with every signal marked not available, so a controller writes
/// only the signals it actually reports.
#[gen_stub_pyclass]
#[pyclass(name = "Signals")]
pub struct CanSignals {
    inner: Signals,
}

#[gen_stub_pymethods]
#[pymethods]
impl CanSignals {
    /// Builds a payload with every signal marked not available.
    #[new]
    fn new() -> CanSignals {
        CanSignals {
            inner: Signals::new(),
        }
    }

    /// Reads the eight data bytes of a frame that arrived off the bus.
    #[staticmethod]
    fn from_bytes(bytes: Vec<u8>) -> PyResult<CanSignals> {
        let bytes: [u8; 8] = bytes.as_slice().try_into().map_err(|_| {
            PamojaError::new_err("a J1939 payload is exactly eight bytes".to_string())
        })?;
        Ok(CanSignals {
            inner: Signals::from_bytes(bytes),
        })
    }

    /// Writes a one-byte signal at the offset its parameter group defines.
    fn set_u8(&mut self, at: usize, value: u8) {
        self.inner.set_u8(at, value);
    }

    /// Writes a two-byte little-endian signal at the offset its group defines.
    fn set_u16(&mut self, at: usize, value: u16) {
        self.inner.set_u16(at, value);
    }

    /// Reads a one-byte signal, or `None` if the offset is past the payload.
    fn u8(&self, at: usize) -> Option<u8> {
        self.inner.u8(at)
    }

    /// Reads a two-byte little-endian signal, or `None` if it would run past the
    /// payload.
    fn u16(&self, at: usize) -> Option<u16> {
        self.inner.u16(at)
    }

    /// The eight data bytes, ready to put in a frame.
    #[getter]
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.as_bytes())
    }
}

/// Describes a built frame as the read-only object Python receives.
fn describe(frame: Frame) -> CanFrame {
    CanFrame {
        id: frame.id().raw(),
        extended: frame.id().is_extended(),
        fd: frame.is_fd(),
        remote: frame.is_remote(),
        len: frame.len(),
        dlc: frame.dlc(),
        data: frame.data().to_vec(),
    }
}

/// Builds an identifier of the requested width, masking the value to fit it.
fn identifier(id: u32, extended: bool) -> CanId {
    if extended {
        CanId::extended(id)
    } else {
        CanId::standard(id as u16)
    }
}

/// Maps a framing error onto the SDK's Python exception.
fn to_py(error: CanError) -> PyErr {
    PamojaError::new_err(error.to_string())
}
