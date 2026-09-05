//! The error type for CAN framing.

/// What can go wrong building a CAN frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanError {
    /// The data is longer than the frame kind allows (8 bytes for classic CAN, 64 for
    /// CAN-FD).
    DataTooLong,
    /// The data length is not one a CAN-FD frame can carry. CAN-FD allows 0 to 8 bytes,
    /// then only 12, 16, 20, 24, 32, 48, and 64.
    InvalidFdLength,
}

impl core::fmt::Display for CanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CanError::DataTooLong => f.write_str("can frame data exceeds the maximum length"),
            CanError::InvalidFdLength => {
                f.write_str("can-fd data length is not a valid frame length")
            }
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, so a caller on a
// microcontroller gets the same trait a caller on a gateway does.
impl core::error::Error for CanError {}
