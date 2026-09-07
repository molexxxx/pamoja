//! The error type shared by the sensor drivers.

/// What can go wrong turning a part's raw bytes into a reading.
///
/// Most of these drivers only ever decode well-formed register values and so cannot
/// fail, but parts that carry their own integrity check report a mismatch here so the
/// caller re-reads rather than trusting corrupted data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensorError {
    /// A device's own checksum did not match the bytes it covered, so the read was
    /// corrupted on the bus and must be repeated. Returned, for example, when a
    /// DS18B20 scratchpad's CRC byte disagrees with the data bytes.
    Crc,
    /// A register field holds a code the datasheet leaves undefined, so the value
    /// cannot have come from a correctly working part. Returned, for example, when an
    /// HDC1080 configuration register carries the unassigned humidity-resolution code.
    Invalid,
    /// A device's identification registers did not carry the values its datasheet
    /// fixes, so a different part, or nothing at all, answered at that address and
    /// its readings must not be trusted. Returned, for example, when an INA226's
    /// manufacturer or die ID register reads something other than TI's 0x5449 and
    /// 0x226.
    Identity,
}

impl core::fmt::Display for SensorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SensorError::Crc => f.write_str("sensor checksum mismatch"),
            SensorError::Invalid => f.write_str("sensor register field holds an undefined code"),
            SensorError::Identity => f.write_str("sensor identification mismatch"),
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, so a caller on a
// microcontroller gets the same trait a caller on a gateway does.
impl core::error::Error for SensorError {}
