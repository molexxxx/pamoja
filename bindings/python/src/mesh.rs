//! Generated Python bindings for mesh packet framing.
//!
//! These mirror the `pamoja-mesh` Rust API: an addressed packet that hops node to
//! node across a radio mesh, and the duplicate suppressor that stops a flood from
//! circulating forever.
//!
//! A frame is a small value rather than a resource, so it crosses as a read-only
//! object carrying both its fields and the bytes to transmit. The duplicate cache
//! holds state across calls, so it is a class, sized when it is built because the
//! Rust crate fixes its size with a const generic that cannot reach Python.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_mesh::{crc16, DynamicSeenCache, Frame, MeshError};

use crate::PamojaError;

/// A reasonable duplicate-cache size for a caller with no reason to choose one.
const SEEN_DEFAULT_CAPACITY: usize = 64;

/// A mesh packet: its addressing, its payload, and the bytes to transmit.
#[gen_stub_pyclass]
#[pyclass]
pub struct MeshFrame {
    /// The protocol version the frame declares.
    #[pyo3(get)]
    version: u8,
    /// The address of the node the frame came from.
    #[pyo3(get)]
    src: u32,
    /// The address the frame is addressed to.
    #[pyo3(get)]
    dst: u32,
    /// The sequence number identifying this packet from this source.
    #[pyo3(get)]
    id: u16,
    /// How many further relays the frame may take.
    #[pyo3(get)]
    hop_limit: u8,
    /// Whether the frame is addressed to every node.
    #[pyo3(get)]
    broadcast: bool,
    payload: Vec<u8>,
    bytes: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MeshFrame {
    /// The payload the frame carries.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.payload)
    }

    /// The whole frame as it goes on the air.
    #[getter]
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.bytes)
    }
}

/// A memory of recently seen packets, so a node relays each one only once.
#[gen_stub_pyclass]
#[pyclass]
pub struct SeenPackets {
    inner: DynamicSeenCache,
}

#[gen_stub_pymethods]
#[pymethods]
impl SeenPackets {
    /// Creates an empty cache remembering up to `capacity` packets.
    ///
    /// A capacity of zero remembers nothing, so every copy of a packet is relayed.
    #[new]
    #[pyo3(signature = (capacity = SEEN_DEFAULT_CAPACITY))]
    fn new(capacity: usize) -> Self {
        SeenPackets {
            inner: DynamicSeenCache::new(capacity),
        }
    }

    /// Reports whether a packet is currently remembered, without recording it.
    fn contains(&self, src: u32, id: u16) -> bool {
        self.inner.contains((src, id))
    }

    /// Records a packet and reports whether it was new.
    ///
    /// A true answer is when a node should act on the packet and relay it; a
    /// false one means another copy already arrived by a different path.
    fn record(&mut self, src: u32, id: u16) -> bool {
        self.inner.record((src, id))
    }

    /// How many packets this cache remembers.
    #[getter]
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }
}

/// Builds a mesh frame addressed to one node.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (src, dst, id, payload, hop_limit = None))]
pub fn mesh_frame(
    src: u32,
    dst: u32,
    id: u16,
    payload: Vec<u8>,
    hop_limit: Option<u8>,
) -> PyResult<MeshFrame> {
    Frame::new(src, dst, id, &payload)
        .map(|frame| describe(limited(frame, hop_limit)))
        .map_err(to_py)
}

/// Builds a mesh frame addressed to every node.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (src, id, payload, hop_limit = None))]
pub fn mesh_broadcast_frame(
    src: u32,
    id: u16,
    payload: Vec<u8>,
    hop_limit: Option<u8>,
) -> PyResult<MeshFrame> {
    Frame::broadcast(src, id, &payload)
        .map(|frame| describe(limited(frame, hop_limit)))
        .map_err(to_py)
}

/// Parses a frame received off a radio, rejecting anything the air mangled.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mesh_parse_frame(bytes: Vec<u8>) -> PyResult<MeshFrame> {
    Frame::parse(&bytes).map(describe).map_err(to_py)
}

/// Returns the same frame with one hop spent, or `None` once its hops run out.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mesh_relayed(bytes: Vec<u8>) -> PyResult<Option<MeshFrame>> {
    let frame = Frame::parse(&bytes).map_err(to_py)?;
    Ok(frame.relayed().map(describe))
}

/// Computes the CRC-16 a mesh frame carries.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mesh_crc16(data: Vec<u8>) -> u16 {
    crc16(&data)
}

/// Returns the frame, payload, and cache sizes a mesh node works within.
///
/// The order is the maximum frame length, the maximum payload, the broadcast
/// address, the default hop limit, and the duplicate-cache capacity.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mesh_limits() -> (usize, usize, u32, u8, usize) {
    (
        Frame::MAX_LEN,
        Frame::MAX_PAYLOAD,
        pamoja_mesh::BROADCAST,
        Frame::DEFAULT_HOP_LIMIT,
        SEEN_DEFAULT_CAPACITY,
    )
}

/// Applies a hop limit when the caller gave one.
fn limited(frame: Frame, hop_limit: Option<u8>) -> Frame {
    match hop_limit {
        Some(hop_limit) => frame.with_hop_limit(hop_limit),
        None => frame,
    }
}

/// Reads every field off a frame into the object Python receives.
fn describe(frame: Frame) -> MeshFrame {
    MeshFrame {
        version: frame.version(),
        src: frame.src(),
        dst: frame.dst(),
        id: frame.id(),
        hop_limit: frame.hop_limit(),
        broadcast: frame.is_broadcast(),
        payload: frame.payload().to_vec(),
        bytes: frame.as_bytes().to_vec(),
    }
}

/// Turns a mesh error into the Python exception a caller sees.
fn to_py(error: MeshError) -> PyErr {
    PamojaError::new_err(error.to_string())
}
