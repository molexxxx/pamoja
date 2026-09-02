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
use pamoja_can::{dlc_to_len, len_to_dlc, CanError, CanId, Frame, J1939Id};

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
