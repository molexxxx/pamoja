//! Generated Node bindings for CAN bus framing.
//!
//! These mirror the `pamoja-can` Rust API: classic CAN 2.0 and CAN-FD frames, the
//! length encoding CAN-FD uses above eight bytes, and the J1939 identifier that
//! trucks, tractors, and gensets ride on top of it.
//!
//! A frame is a small value rather than a resource, so it crosses as a plain
//! object; the identifier a J1939 message decodes to does the same, with a
//! `destination` of `null` for a broadcast rather than a flag to check first.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_can::{
    dlc_to_len, len_to_dlc, priority, CanError, CanId, Frame, J1939Id, Signals, BROADCAST_ADDRESS,
    NOT_AVAILABLE,
};

/// A CAN frame: an identifier, its flags, and its payload.
#[napi(object)]
pub struct CanFrame {
    /// The arbitration identifier, already masked to 11 or 29 bits.
    pub id: u32,
    /// Whether the identifier is a 29-bit extended one.
    pub extended: bool,
    /// Whether this is a CAN-FD frame rather than classic CAN 2.0.
    pub fd: bool,
    /// Whether this is a remote transmission request, which carries no payload.
    pub remote: bool,
    /// The data length: the payload length, or the length a remote frame requests.
    pub len: u8,
    /// The data length code as it appears on the wire.
    pub dlc: u8,
    /// The payload, empty for a remote frame.
    pub data: Buffer,
}

/// The fields J1939 packs into an extended CAN identifier.
#[napi(object)]
pub struct J1939Message {
    /// The parameter group number, which names what the message carries.
    pub pgn: u32,
    /// The message priority, 0 (highest) to 7.
    pub priority: u8,
    /// The source address: the node that sent the message.
    pub source: u8,
    /// The PDU format byte of the parameter group.
    pub pdu_format: u8,
    /// The destination address for an addressed (PDU1) message, or `null` for a
    /// broadcast (PDU2) one.
    pub destination: Option<u8>,
    /// Whether the message is a broadcast.
    pub broadcast: bool,
}

/// Builds a classic CAN 2.0 frame, which carries up to eight bytes.
#[napi]
pub fn can_frame(id: u32, extended: bool, data: Buffer) -> napi::Result<CanFrame> {
    Frame::new(identifier(id, extended), data.as_ref())
        .map(describe)
        .map_err(to_napi)
}

/// Builds a CAN-FD frame, which carries up to 64 bytes at the discrete CAN-FD lengths.
#[napi]
pub fn can_fd_frame(id: u32, extended: bool, data: Buffer) -> napi::Result<CanFrame> {
    Frame::fd(identifier(id, extended), data.as_ref())
        .map(describe)
        .map_err(to_napi)
}

/// Builds a remote transmission request, which asks another node to send.
#[napi]
pub fn can_remote_frame(id: u32, extended: bool, len: u8) -> CanFrame {
    describe(Frame::remote(identifier(id, extended), len as usize))
}

/// Returns the data length code that encodes a payload length.
#[napi]
pub fn can_len_to_dlc(len: u32) -> u8 {
    len_to_dlc(len as usize)
}

/// Returns the payload length a data length code encodes.
#[napi]
pub fn can_dlc_to_len(dlc: u8) -> u32 {
    dlc_to_len(dlc) as u32
}

/// Decodes the J1939 fields out of an extended CAN identifier.
///
/// Returns `null` for a standard 11-bit identifier, which J1939 does not use.
#[napi]
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
#[napi]
pub fn j1939_compose(priority: u8, pgn: u32, source: u8, destination: u8) -> u32 {
    J1939Id::from_parts(priority, pgn, source, destination)
        .to_id()
        .raw()
}

/// Composes the identifier of a J1939 broadcast, which every node on the bus reads.
///
/// This is the ordinary case: most parameter groups are broadcast, and a caller
/// should not have to know that a broadcast is addressed to `0xFF`.
#[napi]
pub fn j1939_broadcast(priority: u8, pgn: u32, source: u8) -> u32 {
    J1939Id::broadcast(priority, pgn, source).to_id().raw()
}

/// The byte a J1939 sender writes for a signal it is not reporting.
#[napi]
pub const J1939_NOT_AVAILABLE: u8 = NOT_AVAILABLE;

/// The destination address every node on the bus reads.
#[napi]
pub const J1939_BROADCAST_ADDRESS: u8 = BROADCAST_ADDRESS;

/// The priority a control message takes, ahead of ordinary traffic.
#[napi]
pub const J1939_PRIORITY_CONTROL: u8 = priority::CONTROL;

/// The priority ordinary traffic takes.
#[napi]
pub const J1939_PRIORITY_DEFAULT: u8 = priority::DEFAULT;

/// The priority that yields to everything else on the bus.
#[napi]
pub const J1939_PRIORITY_LOWEST: u8 = priority::LOWEST;

/// The eight data bytes of a J1939 frame, addressed by the signals inside them.
///
/// A parameter group places each signal at a fixed byte offset, little-endian. A
/// payload starts with every signal marked not available, so a controller writes
/// only the signals it actually reports.
#[napi(js_name = "Signals")]
pub struct CanSignals {
    inner: Signals,
}

impl Default for CanSignals {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl CanSignals {
    /// Builds a payload with every signal marked not available.
    #[napi(constructor)]
    pub fn new() -> CanSignals {
        CanSignals {
            inner: Signals::new(),
        }
    }

    /// Reads the eight data bytes of a frame that arrived off the bus.
    #[napi(factory)]
    pub fn from_bytes(bytes: Buffer) -> napi::Result<CanSignals> {
        let bytes: [u8; 8] = bytes
            .as_ref()
            .try_into()
            .map_err(|_| napi::Error::from_reason("a J1939 payload is exactly eight bytes"))?;
        Ok(CanSignals {
            inner: Signals::from_bytes(bytes),
        })
    }

    /// Writes a one-byte signal at the offset its parameter group defines.
    #[napi]
    pub fn set_u8(&mut self, at: u32, value: u8) {
        self.inner.set_u8(at as usize, value);
    }

    /// Writes a two-byte little-endian signal at the offset its group defines.
    #[napi]
    pub fn set_u16(&mut self, at: u32, value: u16) {
        self.inner.set_u16(at as usize, value);
    }

    /// Reads a one-byte signal, or `null` if the offset is past the payload.
    #[napi]
    pub fn u8(&self, at: u32) -> Option<u8> {
        self.inner.u8(at as usize)
    }

    /// Reads a two-byte little-endian signal, or `null` if it would run past the
    /// payload.
    #[napi]
    pub fn u16(&self, at: u32) -> Option<u16> {
        self.inner.u16(at as usize)
    }

    /// The eight data bytes, ready to put in a frame.
    #[napi(getter)]
    pub fn bytes(&self) -> Buffer {
        self.inner.as_bytes().to_vec().into()
    }
}

/// Describes a built frame as the plain object JavaScript receives.
fn describe(frame: Frame) -> CanFrame {
    CanFrame {
        id: frame.id().raw(),
        extended: frame.id().is_extended(),
        fd: frame.is_fd(),
        remote: frame.is_remote(),
        len: frame.len() as u8,
        dlc: frame.dlc(),
        data: frame.data().into(),
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

/// Maps a framing error onto a thrown exception.
fn to_napi(error: CanError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
