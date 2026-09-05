//! The error type for on-board bus addressing and pin logic.

/// What can go wrong forming an I2C address frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpioError {
    /// An address is outside its range: a 7-bit address above `0x7F`, or a 10-bit address
    /// above `0x3FF`.
    AddressOutOfRange,
    /// The caller's output buffer is too small to hold the address frame. A 7-bit address
    /// needs one byte and a 10-bit address two;
    /// [`Address::frame_len`](crate::i2c::Address::frame_len) gives the exact count.
    BufferTooSmall,
}

impl core::fmt::Display for GpioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GpioError::AddressOutOfRange => f.write_str("I2C address is out of range"),
            GpioError::BufferTooSmall => {
                f.write_str("output buffer is too small for the I2C address frame")
            }
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, so a caller on a
// microcontroller gets the same trait a caller on a gateway does.
impl core::error::Error for GpioError {}
