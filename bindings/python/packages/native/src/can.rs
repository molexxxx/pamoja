//! Generated Python bindings for CAN bus framing.
//!
//! These mirror the `pamoja-can` Rust API: classic CAN 2.0 and CAN-FD frames, the
//! length encoding CAN-FD uses above eight bytes, and the J1939 identifier that
//! trucks, tractors, and gensets ride on top of it.
//!
//! A frame is a small value rather than a resource, so it crosses as a read-only
//! object; a decoded J1939 identifier does the same, with a `destination` of
//! `None` for a broadcast rather than a flag to check first. A `CanBus` is a node on a
//! bus, simulated or a kernel interface.

use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_can::bus::{CanBus, CanBusKind, Filter};
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

/// A frame a node keeps: one whose identifier, masked, equals `id`, masked, and whose format is
/// the filter's.
#[gen_stub_pyclass]
#[pyclass(frozen, name = "CanFilter")]
pub struct CanFilter {
    /// The identifier to match.
    #[pyo3(get)]
    id: u32,
    /// The identifier bits that have to match.
    #[pyo3(get)]
    mask: u32,
    /// Whether the identifier is a 29-bit extended one.
    #[pyo3(get)]
    extended: bool,
}

impl CanFilter {
    fn of(filter: Filter) -> CanFilter {
        CanFilter {
            id: filter.id().raw(),
            mask: filter.mask(),
            extended: filter.id().is_extended(),
        }
    }

    fn filter(&self) -> Filter {
        Filter::new(identifier(self.id, self.extended), self.mask)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl CanFilter {
    /// A filter on the identifier bits a mask selects; bits outside the identifier's width are
    /// dropped.
    #[new]
    #[pyo3(signature = (id, mask, extended = false))]
    fn new(id: u32, mask: u32, extended: bool) -> CanFilter {
        CanFilter::of(Filter::new(identifier(id, extended), mask))
    }

    /// A filter that passes one identifier and nothing else.
    #[staticmethod]
    #[pyo3(signature = (id, extended = false))]
    fn exact(id: u32, extended: bool) -> CanFilter {
        CanFilter::of(Filter::exact(identifier(id, extended)))
    }

    /// A filter that passes one J1939 parameter group at any priority, from any source, and for
    /// an addressed group, to any destination.
    #[staticmethod]
    fn pgn(pgn: u32) -> CanFilter {
        CanFilter::of(Filter::pgn(pgn))
    }

    /// Whether a frame with an identifier passes the filter.
    #[pyo3(signature = (id, extended = false))]
    fn matches(&self, id: u32, extended: bool) -> bool {
        self.filter().matches(identifier(id, extended))
    }
}

fn frame_of(frame: &CanFrame) -> PyResult<Frame> {
    let id = identifier(frame.id, frame.extended);
    let built = if frame.remote {
        Ok(Frame::remote(id, frame.len))
    } else if frame.fd {
        Frame::fd(id, &frame.data)
    } else {
        Frame::new(id, &frame.data)
    };
    built.map_err(to_py)
}

fn failed(error: impl ToString) -> PyErr {
    PamojaError::new_err(error.to_string())
}

/// One node's place on a CAN bus: a kernel interface through SocketCAN on a Linux board, or a
/// bus inside the program.
///
/// A node hears every frame the others send and none of its own, and keeps only the frames its
/// filters pass. A send and a receive release the interpreter while the bus is busy; a receive
/// on a simulated bus with nothing waiting returns `None` at once and counts its timeout in
/// `waited_micros`.
#[gen_stub_pyclass]
#[pyclass(frozen, name = "CanBus")]
pub struct CanBusNode {
    inner: CanBus,
}

#[gen_stub_pymethods]
#[pymethods]
impl CanBusNode {
    /// Opens a kernel CAN interface, such as `can0`, through SocketCAN. Raises `PamojaError`
    /// anywhere but Linux, and when the interface does not exist or cannot be bound.
    #[staticmethod]
    fn open(interface: String) -> PyResult<CanBusNode> {
        CanBus::open(&interface)
            .map(|inner| CanBusNode { inner })
            .map_err(failed)
    }

    /// A new bus inside the program, with this node the first on it.
    #[staticmethod]
    fn simulated() -> CanBusNode {
        CanBusNode {
            inner: CanBus::simulated(),
        }
    }

    /// Puts another node on the same bus.
    fn join(&self) -> PyResult<CanBusNode> {
        self.inner
            .join()
            .map(|inner| CanBusNode { inner })
            .map_err(failed)
    }

    /// What the bus is: `"Device"` or `"Simulated"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner.kind() {
            CanBusKind::Device => "Device",
            CanBusKind::Simulated => "Simulated",
        }
    }

    /// The kernel interface the node is on, or `None` on a simulated bus.
    #[getter]
    fn interface(&self) -> Option<String> {
        self.inner.interface()
    }

    /// Sends a frame to every other node on the bus.
    fn send(&self, py: Python<'_>, frame: PyRef<'_, CanFrame>) -> PyResult<()> {
        let frame = frame_of(&frame)?;
        let bus = self.inner.clone();
        py.detach(|| bus.send(&frame)).map_err(failed)
    }

    /// Takes the next frame the node keeps, waiting up to `timeout_micros` for one, or returns
    /// `None` when the timeout passed with nothing.
    fn receive(&self, py: Python<'_>, timeout_micros: u64) -> PyResult<Option<CanFrame>> {
        let bus = self.inner.clone();
        let frame = py
            .detach(|| bus.receive(Duration::from_micros(timeout_micros)))
            .map_err(failed)?;
        Ok(frame.map(describe))
    }

    /// Keeps only the frames that pass at least one of the filters, from now on; an empty list
    /// keeps nothing.
    fn set_filters(&self, filters: Vec<PyRef<'_, CanFilter>>) -> PyResult<()> {
        let filters: Vec<Filter> = filters.iter().map(|filter| filter.filter()).collect();
        self.inner.set_filters(&filters).map_err(failed)
    }

    /// Keeps every frame again, as a node does when it joins.
    fn clear_filters(&self) -> PyResult<()> {
        self.inner.clear_filters().map_err(failed)
    }

    /// How many frames the node has sent.
    #[getter]
    fn sent(&self) -> usize {
        self.inner.sent()
    }

    /// How many frames the node has received.
    #[getter]
    fn received(&self) -> usize {
        self.inner.received()
    }

    /// How long receives on the node have waited without a frame, in microseconds, whether or
    /// not the process slept through it.
    #[getter]
    fn waited_micros(&self) -> u64 {
        self.inner.waited_micros()
    }
}
