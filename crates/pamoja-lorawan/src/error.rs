//! The error type for LoRaWAN framing.

/// What can go wrong building or reading a LoRaWAN frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LorawanError {
    /// A payload (or its frame options) is too large to fit a single frame.
    PayloadTooLong,
    /// A received frame is shorter than its fixed header and MIC require.
    FrameTooShort,
    /// A received frame's message type is not one this crate decodes here.
    UnsupportedMType(u8),
    /// A received frame's MIC does not match its contents, so it is forged or corrupt.
    MicMismatch,
    /// A received frame's counter does not match the counter expected for it.
    FcntMismatch,
    /// A received frame is structurally invalid in some other way.
    MalformedFrame,
}

impl core::fmt::Display for LorawanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LorawanError::PayloadTooLong => {
                f.write_str("lorawan payload does not fit a single frame")
            }
            LorawanError::FrameTooShort => {
                f.write_str("lorawan frame is shorter than its header and MIC")
            }
            LorawanError::UnsupportedMType(mtype) => {
                write!(f, "lorawan message type {mtype:#04x} is not decoded here")
            }
            LorawanError::MicMismatch => f.write_str("lorawan MIC does not match the frame"),
            LorawanError::FcntMismatch => {
                f.write_str("lorawan frame counter does not match the one expected")
            }
            LorawanError::MalformedFrame => f.write_str("lorawan frame is malformed"),
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, so a caller on a
// microcontroller gets the same trait a caller on a gateway does.
impl core::error::Error for LorawanError {}
