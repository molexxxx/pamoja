//! Generated Node bindings for mesh packet framing.
//!
//! These mirror the `pamoja-mesh` Rust API: an addressed packet that hops node to
//! node across a radio mesh, and the duplicate suppressor that stops a flood from
//! circulating forever.
//!
//! A frame is a small value rather than a resource, so it crosses as a plain
//! object carrying both its fields and the bytes to transmit. The duplicate cache
//! holds state across calls, so it is a class, fixed at
//! [`MESH_SEEN_CAPACITY`] packets because its Rust size is a const generic.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_mesh::{crc16, Frame, MeshError, SeenCache};

/// The number of recently seen packets a duplicate cache remembers.
pub const SEEN_CAPACITY: usize = 64;

/// The largest payload a single mesh frame can carry, in bytes.
#[napi]
pub const MESH_MAX_PAYLOAD: u32 = Frame::MAX_PAYLOAD as u32;

/// The largest mesh frame, in bytes, including its header and checksum.
#[napi]
pub const MESH_MAX_FRAME: u32 = Frame::MAX_LEN as u32;

/// The destination address that means every node.
#[napi]
pub const MESH_BROADCAST: u32 = pamoja_mesh::BROADCAST;

/// The hop limit a frame starts with unless one is given.
#[napi]
pub const MESH_DEFAULT_HOP_LIMIT: u8 = Frame::DEFAULT_HOP_LIMIT;

/// The number of recently seen packets a duplicate cache remembers.
#[napi]
pub const MESH_SEEN_CAPACITY: u32 = SEEN_CAPACITY as u32;

/// A mesh packet: its addressing, its payload, and the bytes to transmit.
#[napi(object)]
pub struct MeshFrame {
    /// The protocol version the frame declares.
    pub version: u8,
    /// The address of the node the frame came from.
    pub src: u32,
    /// The address the frame is addressed to.
    pub dst: u32,
    /// The sequence number identifying this packet from this source.
    pub id: u16,
    /// How many further relays the frame may take.
    pub hop_limit: u8,
    /// Whether the frame is addressed to every node.
    pub broadcast: bool,
    /// The payload the frame carries.
    pub payload: Buffer,
    /// The whole frame as it goes on the air.
    pub bytes: Buffer,
}

/// Builds a mesh frame addressed to one node.
///
/// `hopLimit` defaults to [`MESH_DEFAULT_HOP_LIMIT`] when omitted.
#[napi]
pub fn mesh_frame(
    src: u32,
    dst: u32,
    id: u16,
    payload: Buffer,
    hop_limit: Option<u8>,
) -> napi::Result<MeshFrame> {
    Frame::new(src, dst, id, payload.as_ref())
        .map(|frame| describe(limited(frame, hop_limit)))
        .map_err(to_napi)
}

/// Builds a mesh frame addressed to every node.
///
/// `hopLimit` defaults to [`MESH_DEFAULT_HOP_LIMIT`] when omitted.
#[napi]
pub fn mesh_broadcast_frame(
    src: u32,
    id: u16,
    payload: Buffer,
    hop_limit: Option<u8>,
) -> napi::Result<MeshFrame> {
    Frame::broadcast(src, id, payload.as_ref())
        .map(|frame| describe(limited(frame, hop_limit)))
        .map_err(to_napi)
}

/// Parses a frame received off a radio, rejecting anything the air mangled.
#[napi]
pub fn mesh_parse_frame(bytes: Buffer) -> napi::Result<MeshFrame> {
    Frame::parse(bytes.as_ref()).map(describe).map_err(to_napi)
}

/// Returns the same frame with one hop spent, ready to forward.
///
/// Returns `null` once the hop limit has run out, which is what stops a flood from
/// circulating forever.
#[napi]
pub fn mesh_relayed(bytes: Buffer) -> napi::Result<Option<MeshFrame>> {
    let frame = Frame::parse(bytes.as_ref()).map_err(to_napi)?;
    Ok(frame.relayed().map(describe))
}

/// Computes the CRC-16 a mesh frame carries.
#[napi]
pub fn mesh_crc16(data: Buffer) -> u16 {
    crc16(data.as_ref())
}

/// A memory of recently seen packets, so a node relays each one only once.
#[napi]
pub struct SeenPackets {
    inner: SeenCache<SEEN_CAPACITY>,
}

#[napi]
impl SeenPackets {
    /// Creates an empty cache of [`MESH_SEEN_CAPACITY`] packets.
    #[napi(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: SeenCache::new(),
        }
    }

    /// Reports whether a packet is currently remembered, without recording it.
    #[napi]
    pub fn contains(&self, src: u32, id: u16) -> bool {
        self.inner.contains((src, id))
    }

    /// Records a packet and reports whether it was new.
    ///
    /// A `true` answer is when a node should act on the packet and relay it; a
    /// `false` one means another copy already arrived by a different path.
    #[napi]
    pub fn record(&mut self, src: u32, id: u16) -> bool {
        self.inner.record((src, id))
    }

    /// How many packets this cache remembers.
    #[napi(getter)]
    pub fn capacity(&self) -> u32 {
        MESH_SEEN_CAPACITY
    }
}

/// Applies a hop limit when the caller gave one.
fn limited(frame: Frame, hop_limit: Option<u8>) -> Frame {
    match hop_limit {
        Some(hop_limit) => frame.with_hop_limit(hop_limit),
        None => frame,
    }
}

/// Reads every field off a frame into the object JavaScript receives.
fn describe(frame: Frame) -> MeshFrame {
    MeshFrame {
        version: frame.version(),
        src: frame.src(),
        dst: frame.dst(),
        id: frame.id(),
        hop_limit: frame.hop_limit(),
        broadcast: frame.is_broadcast(),
        payload: frame.payload().to_vec().into(),
        bytes: frame.as_bytes().to_vec().into(),
    }
}

/// Turns a mesh error into the JavaScript error a caller sees.
fn to_napi(error: MeshError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
