//! The error type for mesh framing.

/// What can go wrong building or reading a mesh frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshError {
    /// A frame is shorter than the header and checksum a frame must at least contain.
    FrameTooShort,
    /// A frame is larger than [`Frame::MAX_LEN`](crate::Frame::MAX_LEN).
    FrameTooLong,
    /// A payload is larger than [`Frame::MAX_PAYLOAD`](crate::Frame::MAX_PAYLOAD), so it
    /// will not fit a single frame.
    PayloadTooLong,
    /// A received frame declares a protocol version this build does not understand.
    UnsupportedVersion(u8),
    /// A received frame's checksum does not match its contents, so the frame is corrupt.
    CrcMismatch {
        /// The checksum computed over the frame's contents.
        expected: u16,
        /// The checksum the frame carried.
        found: u16,
    },
}

impl core::fmt::Display for MeshError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MeshError::FrameTooShort => {
                f.write_str("mesh frame is shorter than its header and checksum")
            }
            MeshError::FrameTooLong => {
                f.write_str("mesh frame is larger than the maximum frame size")
            }
            MeshError::PayloadTooLong => {
                f.write_str("mesh payload is larger than a single frame can carry")
            }
            MeshError::UnsupportedVersion(version) => {
                write!(f, "unsupported mesh protocol version {version}")
            }
            MeshError::CrcMismatch { expected, found } => {
                write!(
                    f,
                    "mesh CRC mismatch: expected {expected:#06x}, found {found:#06x}"
                )
            }
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, so a caller on a
// microcontroller gets the same trait a caller on a gateway does.
impl core::error::Error for MeshError {}
