//! The error model for the MAVLink wire layer.
//!
//! A single [`MavlinkError`] keeps framing, checksum, and signing faults uniform, the
//! way [`pamoja-modbus`](https://docs.rs/pamoja-modbus) and
//! [`pamoja-lorawan`](https://docs.rs/pamoja-lorawan) each carry their own protocol
//! error type ahead of the transport layer.

use core::fmt;

/// A fault encountered while building, parsing, signing, or verifying a frame.
///
/// This enum is `#[non_exhaustive]`: new variants may be added without a breaking
/// change, so a downstream `match` must include a wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MavlinkError {
    /// The bytes are shorter than the smallest valid frame of their version.
    FrameTooShort,

    /// The first byte is neither the v1 (`0xFE`) nor the v2 (`0xFD`) start marker.
    BadMagic(u8),

    /// The buffer ended before the frame its length field promised was complete.
    Truncated,

    /// The trailing checksum did not match the one computed over the frame.
    CrcMismatch {
        /// The checksum computed over the received bytes.
        expected: u16,
        /// The checksum the frame carried.
        found: u16,
    },

    /// A message id has no known `CRC_EXTRA`, so its checksum cannot be validated and
    /// its payload cannot be decoded.
    UnknownMessage(u32),

    /// A payload, frame, or field array was larger than the protocol or buffer allows.
    PayloadTooLong,

    /// A signature was required but the frame was not signed.
    Unsigned,

    /// The frame's signature did not match the one computed with the key.
    BadSignature,

    /// The frame's signing timestamp was older than the link's replay window allows.
    ReplayedTimestamp,

    /// A typed message was decoded from a payload of the wrong size or shape.
    BadPayload,

    /// The message has no field of the requested name.
    UnknownField,

    /// A field type name is not one MAVLink defines.
    UnknownFieldType,

    /// A message shape names the same field twice.
    DuplicateField,

    /// A field was read or written as a kind its type is not.
    FieldTypeMismatch,

    /// An array element index is past the end of the field.
    FieldIndexOutOfRange,

    /// A value does not fit the field's type.
    ValueOutOfRange,

    /// The link reached end of input before a frame could be read.
    Closed,
}

impl fmt::Display for MavlinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameTooShort => f.write_str("frame is shorter than a valid frame"),
            Self::BadMagic(byte) => write!(f, "unrecognized start marker: {byte:#04x}"),
            Self::Truncated => f.write_str("frame is shorter than its length field promises"),
            Self::CrcMismatch { expected, found } => {
                write!(
                    f,
                    "checksum mismatch: expected {expected:#06x}, found {found:#06x}"
                )
            }
            Self::UnknownMessage(id) => write!(f, "no CRC_EXTRA known for message id {id}"),
            Self::PayloadTooLong => f.write_str("payload exceeds the maximum frame size"),
            Self::Unsigned => f.write_str("frame is not signed"),
            Self::BadSignature => f.write_str("signature does not verify"),
            Self::ReplayedTimestamp => f.write_str("signing timestamp is too old"),
            Self::BadPayload => f.write_str("payload does not match the message layout"),
            Self::UnknownField => f.write_str("message has no field of that name"),
            Self::UnknownFieldType => f.write_str("unrecognized MAVLink field type"),
            Self::DuplicateField => f.write_str("message names the same field twice"),
            Self::FieldTypeMismatch => f.write_str("field is not of the requested kind"),
            Self::FieldIndexOutOfRange => f.write_str("array index is past the end of the field"),
            Self::ValueOutOfRange => f.write_str("value does not fit the field type"),
            Self::Closed => f.write_str("link reached end of input"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MavlinkError {}

/// A specialized [`core::result::Result`] whose error type is [`MavlinkError`].
pub type Result<T> = core::result::Result<T, MavlinkError>;
