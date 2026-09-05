//! The error type for Modbus framing.

/// What can go wrong building or reading a Modbus RTU frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModbusError {
    /// A frame is shorter than the smallest valid RTU ADU (address, function, CRC).
    FrameTooShort,
    /// A frame or PDU is longer than the 256-byte RTU maximum allows.
    FrameTooLong,
    /// A received frame's CRC does not match its contents, so the frame is corrupt.
    CrcMismatch {
        /// The CRC computed over the frame's contents.
        expected: u16,
        /// The CRC the frame carried.
        found: u16,
    },
    /// A write request named a number of values a single request cannot carry (it must
    /// be between one and the function's maximum).
    InvalidValueCount,
    /// A response PDU is truncated or its declared byte count does not match its data.
    MalformedResponse,
}

impl core::fmt::Display for ModbusError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ModbusError::FrameTooShort => {
                f.write_str("modbus frame is shorter than a valid RTU ADU")
            }
            ModbusError::FrameTooLong => {
                f.write_str("modbus frame exceeds the 256-byte RTU maximum")
            }
            ModbusError::CrcMismatch { expected, found } => {
                write!(
                    f,
                    "modbus CRC mismatch: expected {expected:#06x}, found {found:#06x}"
                )
            }
            ModbusError::InvalidValueCount => {
                f.write_str("modbus write request value count is out of range")
            }
            ModbusError::MalformedResponse => f.write_str("modbus response PDU is malformed"),
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, so a caller on a
// microcontroller gets the same trait a caller on a gateway does.
impl core::error::Error for ModbusError {}
