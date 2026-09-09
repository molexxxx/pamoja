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

/// What can go wrong driving a part over a bus, beyond decoding its bytes.
///
/// A driver returns this from every transfer sequence. It converts into the core
/// [`Error`](pamoja_core::Error) so a driver used as a
/// [`Sensor`](pamoja_core::Sensor) reports the way every other device does: a bus
/// fault and a timeout become [`Error::Io`](pamoja_core::Error::Io), a checksum or an
/// undefined field become [`Error::Codec`](pamoja_core::Error::Codec), and a part
/// that identifies as something else becomes [`Error::Io`](pamoja_core::Error::Io).
#[cfg(feature = "embedded-hal")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriverError<E> {
    /// The bus underneath the driver failed: a missing acknowledge, a bus fault, a
    /// pin that could not be driven.
    Bus(E),
    /// The part's bytes decoded to something its datasheet rules out.
    Sensor(SensorError),
    /// The part did not finish within the time its datasheet allows, so its data
    /// registers never became ready.
    Timeout,
}

#[cfg(feature = "embedded-hal")]
impl<E> From<SensorError> for DriverError<E> {
    fn from(error: SensorError) -> Self {
        DriverError::Sensor(error)
    }
}

#[cfg(feature = "embedded-hal")]
impl<E: core::fmt::Debug> core::fmt::Display for DriverError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DriverError::Bus(error) => write!(f, "bus error: {error:?}"),
            DriverError::Sensor(error) => write!(f, "{error}"),
            DriverError::Timeout => f.write_str("the part did not finish its conversion in time"),
        }
    }
}

#[cfg(feature = "embedded-hal")]
impl<E: core::fmt::Debug> core::error::Error for DriverError<E> {}

#[cfg(feature = "embedded-hal")]
impl<E: core::fmt::Debug> From<DriverError<E>> for pamoja_core::Error {
    fn from(error: DriverError<E>) -> Self {
        use alloc::string::ToString;

        match error {
            DriverError::Bus(bus) => pamoja_core::Error::Io(alloc::format!("bus error: {bus:?}")),
            DriverError::Timeout => pamoja_core::Error::Io(error.to_string()),
            DriverError::Sensor(SensorError::Identity) => pamoja_core::Error::Io(error.to_string()),
            DriverError::Sensor(sensor) => pamoja_core::Error::Codec(sensor.to_string()),
        }
    }
}
