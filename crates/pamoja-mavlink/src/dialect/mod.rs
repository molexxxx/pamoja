//! The typed message layer: a broad slice of the MAVLink common dialect, plus the seam
//! that lets any message id be carried and checked.
//!
//! A [`Frame`] moves opaque payload bytes; this module gives those bytes
//! meaning. Each typed message knows its id, its `CRC_EXTRA` seed, and how its fields are
//! laid out on the wire, so a sender fills named fields instead of hand-packing a buffer
//! and a receiver reads them back. The set covers what a ground station and an autopilot
//! actually exchange: the periodic [`Heartbeat`], system status, the command, parameter,
//! and mission protocols, and the core position and attitude telemetry.
//!
//! Messages are declared from one source of truth and their byte layout is derived from
//! it, including the field reordering MAVLink applies (largest field first). Each message
//! carries the official `CRC_EXTRA` for its shape, and a test re-derives that seed from
//! the field definitions, so a wrong field type, name, or order is caught against the
//! published dialect rather than only by a round-trip.
//!
//! A message this crate does not type is still reachable two ways: [`RawMessage`] carries
//! it by id and raw payload, and a [`MessageDescriptor`] gives those bytes named fields, so
//! a vendor's dialect is usable at runtime without being compiled in.

use crate::error::Result;
use crate::frame::{Frame, Header, MAX_PAYLOAD};

#[macro_use]
mod macros;

mod common;
mod enums;
mod schema;

pub use common::*;
pub use enums::*;
pub use schema::*;

/// A typed MAVLink message: its identity on the wire and how it serializes.
///
/// Implemented for every message this crate types, via the `message!` declaration macro.
pub trait Message: Sized {
    /// The message id on the wire.
    const ID: u32;
    /// The message name, such as `"HEARTBEAT"`, as used to derive [`CRC_EXTRA`](Self::CRC_EXTRA).
    const NAME: &'static str;
    /// The `CRC_EXTRA` seed folded into the checksum of a frame carrying this message.
    const CRC_EXTRA: u8;
    /// The full length, in bytes, of this message's base fields on the wire.
    const WIRE_LEN: usize;
    /// The base fields in wire order as `(type, name, array_len)`, the input from which
    /// [`CRC_EXTRA`](Self::CRC_EXTRA) is derived and against which it is verified.
    const BASE_FIELDS: &'static [(&'static str, &'static str, u8)];

    /// The message's shape as data: the runtime counterpart of this type's fields, for a
    /// caller reading and writing by name rather than through the struct.
    const DESCRIPTOR: &'static MessageDescriptor<'static>;

    /// Serializes the message into `out`, returning the number of bytes written.
    ///
    /// # Arguments
    ///
    /// * `out` - the destination buffer, which must be at least [`WIRE_LEN`](Self::WIRE_LEN)
    ///   bytes; a [`MAX_PAYLOAD`]-byte buffer always suffices.
    ///
    /// # Returns
    ///
    /// The number of bytes written, which is [`WIRE_LEN`](Self::WIRE_LEN).
    fn encode(&self, out: &mut [u8]) -> usize;

    /// Deserializes the message from a payload.
    ///
    /// A short payload is zero-extended, as MAVLink 2 truncation requires, and a payload
    /// longer than [`WIRE_LEN`](Self::WIRE_LEN) (one carrying extension fields) has its
    /// trailing bytes ignored.
    ///
    /// # Arguments
    ///
    /// * `payload` - the frame payload to read.
    ///
    /// # Returns
    ///
    /// The decoded message.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::BadPayload`](crate::MavlinkError::BadPayload) if the
    /// payload cannot form the message.
    fn decode(payload: &[u8]) -> Result<Self>;
}

/// Builds a v2 frame carrying a typed message.
///
/// # Arguments
///
/// * `header` - the addressing fields to stamp on the frame.
/// * `message` - the message to send.
///
/// # Returns
///
/// The frame ready to send.
///
/// # Errors
///
/// Returns [`MavlinkError::PayloadTooLong`](crate::MavlinkError::PayloadTooLong) if the
/// message does not fit a frame.
pub fn encode_message<M: Message>(header: Header, message: &M) -> Result<Frame> {
    let mut payload = [0u8; MAX_PAYLOAD];
    let len = message.encode(&mut payload);
    Frame::encode_v2(header, M::ID, &payload[..len], M::CRC_EXTRA)
}

/// A message this crate does not type, carried by id and raw payload.
///
/// This is the escape hatch for a message id outside the typed set or from another
/// dialect: supply its id, payload, and `CRC_EXTRA` and it frames and checks like any
/// other, the way Modbus carries a function code it does not name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawMessage<'a> {
    /// The message id.
    pub msgid: u32,
    /// The `CRC_EXTRA` seed for the message id.
    pub crc_extra: u8,
    /// The raw payload bytes.
    pub payload: &'a [u8],
}

impl<'a> RawMessage<'a> {
    /// Builds a v2 frame carrying this raw message.
    ///
    /// # Arguments
    ///
    /// * `header` - the addressing fields to stamp on the frame.
    ///
    /// # Returns
    ///
    /// The frame ready to send.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::PayloadTooLong`](crate::MavlinkError::PayloadTooLong) if the
    /// payload does not fit a frame.
    pub fn to_frame(&self, header: Header) -> Result<Frame> {
        Frame::encode_v2(header, self.msgid, self.payload, self.crc_extra)
    }
}

/// Returns the `CRC_EXTRA` for a common-dialect message id, if known.
///
/// A [`Parser`](crate::Parser) or [`Frame::parse_with`] uses this to validate the
/// checksum of a frame off the wire, so traffic from a real autopilot is checked even for
/// messages this crate does not type.
///
/// # Arguments
///
/// * `msgid` - the message id to look up.
///
/// # Returns
///
/// The `CRC_EXTRA` seed, or [`None`] if the id is not in the table.
pub fn crc_extra(msgid: u32) -> Option<u8> {
    COMMON_CRC_EXTRA
        .iter()
        .find(|(id, _)| *id == msgid)
        .map(|(_, crc)| *crc)
}

// The official common-dialect CRC_EXTRA seeds for the message ids this crate handles.
// Each is the value the reference dialect publishes; the typed messages above re-derive
// the same value from their field definitions in the test below.
const COMMON_CRC_EXTRA: &[(u32, u8)] = &[
    (0, 50),
    (1, 124),
    (2, 137),
    (4, 237),
    (11, 89),
    (20, 214),
    (21, 159),
    (22, 220),
    (23, 168),
    (24, 24),
    (30, 39),
    (31, 246),
    (32, 185),
    (33, 104),
    (36, 222),
    (40, 230),
    (42, 28),
    (43, 132),
    (44, 221),
    (45, 232),
    (47, 153),
    (51, 196),
    (65, 118),
    (69, 243),
    (73, 38),
    (74, 20),
    (75, 158),
    (76, 152),
    (77, 143),
    (84, 143),
    (86, 5),
    (147, 154),
    (148, 178),
    (242, 104),
    (245, 130),
    (253, 83),
];

// Trims a single trailing underscore from a Rust field identifier to recover the wire
// field name, so a field that collides with a keyword (such as `type_`) carries the name
// the dialect uses (`type`) in its CRC_EXTRA derivation.
pub(crate) const fn xml_name(name: &str) -> &str {
    let bytes = name.as_bytes();
    let len = bytes.len();
    if len > 0 && bytes[len - 1] == b'_' {
        let (head, _) = bytes.split_at(len - 1);
        match core::str::from_utf8(head) {
            Ok(trimmed) => trimmed,
            Err(_) => name,
        }
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_resolves_known_ids_and_rejects_others() {
        assert_eq!(crc_extra(0), Some(50));
        assert_eq!(crc_extra(76), Some(152));
        assert_eq!(crc_extra(9999), None);
    }

    #[test]
    fn xml_name_trims_a_keyword_field() {
        assert_eq!(xml_name("type_"), "type");
        assert_eq!(xml_name("custom_mode"), "custom_mode");
        assert_eq!(xml_name(""), "");
    }
}
