//! The error type for serial-line framing.

/// What can go wrong framing or unframing a serial packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerialError {
    /// The caller's output buffer is too small to hold the encoded frame or the decoded
    /// payload. The `max_encoded_len` helpers size a buffer that always fits.
    BufferTooSmall,
    /// A SLIP escape byte was followed by a byte that is neither the escaped-delimiter nor
    /// the escaped-escape marker, so the frame is corrupt.
    InvalidEscape,
    /// A frame ended early: a SLIP frame stopped in the middle of an escape sequence, or a
    /// COBS code byte claimed more data than the frame actually carried.
    TruncatedFrame,
}

impl core::fmt::Display for SerialError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SerialError::BufferTooSmall => {
                f.write_str("serial output buffer is too small for the frame")
            }
            SerialError::InvalidEscape => {
                f.write_str("serial frame contains an invalid SLIP escape sequence")
            }
            SerialError::TruncatedFrame => f.write_str("serial frame is truncated"),
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, so a caller on a
// microcontroller gets the same trait a caller on a gateway does.
impl core::error::Error for SerialError {}
