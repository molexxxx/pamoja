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
}

impl core::fmt::Display for SensorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SensorError::Crc => f.write_str("sensor checksum mismatch"),
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, so a caller on a
// microcontroller gets the same trait a caller on a gateway does.
impl core::error::Error for SensorError {}
