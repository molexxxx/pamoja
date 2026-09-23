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
    /// A request or a reply named a number of values one frame cannot carry: none, or more
    /// than its function allows.
    InvalidValueCount,
    /// A response PDU is truncated or its declared byte count does not match its data.
    MalformedResponse,
    /// A unit address outside 1 to 247: 0 is the broadcast address, and 248 to 255 are
    /// reserved.
    UnitOutOfRange {
        /// The address given.
        unit: u8,
    },
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
                f.write_str("the number of values is outside what one modbus frame carries")
            }
            ModbusError::MalformedResponse => f.write_str("modbus response PDU is malformed"),
            ModbusError::UnitOutOfRange { unit } => write!(
                f,
                "unit {unit} is not a device address: 0 is broadcast, and 248 to 255 are reserved"
            ),
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, so a caller on a
// microcontroller gets the same trait a caller on a gateway does.
impl core::error::Error for ModbusError {}
