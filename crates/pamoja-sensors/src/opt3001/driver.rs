//! The OPT3001 driven over I2C: identify, configure, convert once, and read the result.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use pamoja_core::Sensor;

use super::{
    raw_from_milli_lux, register, Configuration, ConversionTime, Mode, Reading, DEVICE_ID,
    MANUFACTURER_ID, RANGE_AUTOMATIC,
};
use crate::driver::{read_word, write_word};
use crate::error::{DriverError, SensorError};

/// How long the driver waits between polls of the configuration register, in
/// milliseconds, once the conversion time has elapsed.
pub const POLL_INTERVAL_MILLIS: u32 = 10;

/// How many polls the driver makes before a conversion is called overdue.
///
/// A conversion in automatic range can run a full extra conversion time when the
/// light changes sharply during it, so the polls cover another 800 ms conversion.
pub const STATUS_POLLS: u8 = 100;

/// An OPT3001 on an I2C bus, measuring illuminance on demand in single-shot mode.
///
/// [`init`](Opt3001::init) checks the manufacturer and device id registers and
/// writes the configuration with the part in shutdown, its power-on state.
/// [`measure`](Opt3001::measure) writes the single-shot mode, which starts one
/// conversion at the configured integration time, waits that time, polls the
/// conversion-ready flag, which a read of the configuration register clears, then
/// reads the result register. The part returns itself to shutdown when the
/// conversion ends. As a [`Sensor`] its reading is the [`Reading`]; the lux value
/// is selected with [`Sensor::map`].
///
/// # Examples
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
/// use pamoja_sensors::opt3001::{raw_from_milli_lux, Opt3001, I2C_ADDRESS_GND};
///
/// const PART: u8 = I2C_ADDRESS_GND;
/// let result = raw_from_milli_lux(320_000).to_be_bytes();
/// let bus = I2cScript::new([
///     I2cStep::write_read(PART, [0x7E], [0x54, 0x49]),
///     I2cStep::write_read(PART, [0x7F], [0x30, 0x01]),
///     I2cStep::write(PART, [0x01, 0xC8, 0x10]),
///     I2cStep::write(PART, [0x01, 0xCA, 0x10]),
///     I2cStep::write_read(PART, [0x01], [0xC8, 0x90]),
///     I2cStep::write_read(PART, [0x00], result),
/// ]);
///
/// let mut sensor = Opt3001::new(bus, PART, DelayLog::new());
/// let mut lux = sensor.map(|reading| reading.lux());
/// let reading = block_on(lux.read())?;
/// assert!((reading - 320.0).abs() < 0.5);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Opt3001<I2C, D> {
    bus: I2C,
    delay: D,
    address: u8,
    conversion_time: ConversionTime,
    range_number: u8,
    initialized: bool,
}

impl<I2C, D> Opt3001<I2C, D> {
    /// Wraps an I2C bus and the part's address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `address` - the address the ADDR pin selects, one of the
    ///   `I2C_ADDRESS_*` constants.
    /// * `delay` - the timer that paces the conversions.
    ///
    /// # Returns
    ///
    /// The driver, with the 800 ms integration time and automatic full-scale range,
    /// the part's reset settings.
    pub fn new(bus: I2C, address: u8, delay: D) -> Self {
        Opt3001 {
            bus,
            delay,
            address,
            conversion_time: ConversionTime::Ms800,
            range_number: RANGE_AUTOMATIC,
            initialized: false,
        }
    }

    /// Sets the integration time of each conversion.
    ///
    /// # Arguments
    ///
    /// * `conversion_time` - 100 ms for speed, 800 ms for resolution.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_conversion_time(mut self, conversion_time: ConversionTime) -> Self {
        self.conversion_time = conversion_time;
        self.initialized = false;
        self
    }

    /// Sets the full-scale range number, or automatic range selection.
    ///
    /// # Arguments
    ///
    /// * `range_number` - 0 to [`RANGE_MAX`](super::RANGE_MAX) for a fixed range,
    ///   or [`RANGE_AUTOMATIC`].
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_range(mut self, range_number: u8) -> Self {
        self.range_number = range_number;
        self.initialized = false;
        self
    }

    /// Returns the configuration the driver writes, with the part in shutdown.
    pub fn configuration(&self) -> Configuration {
        self.configuration_in(Mode::Shutdown)
    }

    /// Gives back the bus and the delay.
    ///
    /// # Returns
    ///
    /// The bus and delay the driver was built from.
    pub fn release(self) -> (I2C, D) {
        (self.bus, self.delay)
    }

    fn configuration_in(&self, mode: Mode) -> Configuration {
        Configuration {
            range_number: self.range_number,
            conversion_time: self.conversion_time,
            mode,
            ..Configuration::default()
        }
    }
}

impl<I2C: I2c, D: DelayNs> Opt3001<I2C, D> {
    /// Checks the part's identity and writes the settings with the part in shutdown.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, and [`DriverError::Sensor`]
    /// with [`SensorError::Identity`] if either id register is not an OPT3001's.
    pub fn init(&mut self) -> Result<(), DriverError<I2C::Error>> {
        if self.read(register::MANUFACTURER_ID)? != MANUFACTURER_ID
            || self.read(register::DEVICE_ID)? != DEVICE_ID
        {
            return Err(SensorError::Identity.into());
        }
        self.write(register::CONFIGURATION, self.configuration().bits())?;
        self.initialized = true;
        Ok(())
    }

    /// Runs one conversion and returns the result.
    ///
    /// Initializes the part first if [`init`](Opt3001::init) has not run.
    ///
    /// # Returns
    ///
    /// The result register as a [`Reading`].
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Timeout`] if
    /// the conversion-ready flag never sets, or any error of [`init`](Opt3001::init).
    pub fn measure(&mut self) -> Result<Reading, DriverError<I2C::Error>> {
        if !self.initialized {
            self.init()?;
        }
        let single_shot = self.configuration_in(Mode::SingleShot);
        self.write(register::CONFIGURATION, single_shot.bits())?;
        self.delay
            .delay_ms(u32::from(self.conversion_time.millis()));

        let mut ready = false;
        for _ in 0..STATUS_POLLS {
            let status = Configuration::from_bits(self.read(register::CONFIGURATION)?);
            if status.conversion_ready {
                ready = true;
                break;
            }
            self.delay.delay_ms(POLL_INTERVAL_MILLIS);
        }
        if !ready {
            return Err(DriverError::Timeout);
        }
        Ok(Reading::new(self.read(register::RESULT)?))
    }

    /// Writes the low and high limit registers the interrupt pin compares against.
    ///
    /// # Arguments
    ///
    /// * `low_milli_lux` - the low limit in millilux.
    /// * `high_milli_lux` - the high limit in millilux.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn set_limits(
        &mut self,
        low_milli_lux: u32,
        high_milli_lux: u32,
    ) -> Result<(), DriverError<I2C::Error>> {
        self.write(register::LOW_LIMIT, raw_from_milli_lux(low_milli_lux))?;
        self.write(register::HIGH_LIMIT, raw_from_milli_lux(high_milli_lux))
    }

    fn read(&mut self, register: u8) -> Result<u16, DriverError<I2C::Error>> {
        read_word(&mut self.bus, self.address, register).map_err(DriverError::Bus)
    }

    fn write(&mut self, register: u8, value: u16) -> Result<(), DriverError<I2C::Error>> {
        write_word(&mut self.bus, self.address, register, value).map_err(DriverError::Bus)
    }
}

impl<I2C, D> Sensor for Opt3001<I2C, D>
where
    I2C: I2c + Send,
    I2C::Error: core::fmt::Debug,
    D: DelayNs + Send,
{
    type Reading = Reading;

    async fn read(&mut self) -> pamoja_core::Result<Reading> {
        self.measure().map_err(pamoja_core::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opt3001::{I2C_ADDRESS_GND, I2C_ADDRESS_SDA};
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};

    const CONVERSION_READY: u16 = 1 << 7;

    fn word(address: u8, register: u8, value: u16) -> I2cStep {
        let [high, low] = value.to_be_bytes();
        I2cStep::write(address, [register, high, low])
    }

    fn reply(address: u8, register: u8, value: u16) -> I2cStep {
        I2cStep::write_read(address, [register], value.to_be_bytes())
    }

    fn init_steps(address: u8, shutdown: u16) -> Vec<I2cStep> {
        vec![
            reply(address, register::MANUFACTURER_ID, MANUFACTURER_ID),
            reply(address, register::DEVICE_ID, DEVICE_ID),
            word(address, register::CONFIGURATION, shutdown),
        ]
    }

    #[test]
    fn a_single_shot_conversion_follows_the_datasheet_sequence() {
        let raw = raw_from_milli_lux(320_000);
        let mut steps = init_steps(I2C_ADDRESS_GND, 0xC810);
        steps.extend([
            word(I2C_ADDRESS_GND, register::CONFIGURATION, 0xCA10),
            reply(
                I2C_ADDRESS_GND,
                register::CONFIGURATION,
                0xC810 | CONVERSION_READY,
            ),
            reply(I2C_ADDRESS_GND, register::RESULT, raw),
        ]);
        let mut sensor = Opt3001::new(I2cScript::new(steps), I2C_ADDRESS_GND, DelayLog::new());
        let reading = sensor.measure().unwrap();
        assert_eq!(reading.raw(), raw);
        assert_eq!(reading.milli_lux(), super::super::milli_lux(raw));
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(delay.waits_ns(), [800_000_000]);
    }

    #[test]
    fn the_integration_time_and_range_reach_the_configuration_and_the_wait() {
        let configuration = Configuration {
            range_number: 5,
            conversion_time: ConversionTime::Ms100,
            mode: Mode::Shutdown,
            ..Configuration::default()
        };
        let shutdown = configuration.bits();
        let single_shot = Configuration {
            mode: Mode::SingleShot,
            ..configuration
        }
        .bits();
        let mut steps = init_steps(I2C_ADDRESS_SDA, shutdown);
        steps.extend([
            word(I2C_ADDRESS_SDA, register::CONFIGURATION, single_shot),
            reply(I2C_ADDRESS_SDA, register::CONFIGURATION, shutdown),
            reply(
                I2C_ADDRESS_SDA,
                register::CONFIGURATION,
                shutdown | CONVERSION_READY,
            ),
            reply(I2C_ADDRESS_SDA, register::RESULT, 0x0C80),
        ]);
        let mut sensor = Opt3001::new(I2cScript::new(steps), I2C_ADDRESS_SDA, DelayLog::new())
            .with_conversion_time(ConversionTime::Ms100)
            .with_range(5);
        assert_eq!(sensor.configuration(), configuration);
        assert_eq!(sensor.measure().unwrap().milli_lux(), 32_000);
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(delay.waits_ns(), [100_000_000, 10_000_000]);
    }

    #[test]
    fn a_part_with_the_wrong_id_is_refused() {
        let steps = [
            reply(I2C_ADDRESS_GND, register::MANUFACTURER_ID, MANUFACTURER_ID),
            reply(I2C_ADDRESS_GND, register::DEVICE_ID, 0x3002),
        ];
        let mut sensor = Opt3001::new(I2cScript::new(steps), I2C_ADDRESS_GND, DelayLog::new());
        assert_eq!(
            sensor.init(),
            Err(DriverError::Sensor(SensorError::Identity))
        );
    }

    #[test]
    fn a_conversion_that_never_finishes_is_a_timeout() {
        let mut steps = init_steps(I2C_ADDRESS_GND, 0xC810);
        steps.push(word(I2C_ADDRESS_GND, register::CONFIGURATION, 0xCA10));
        for _ in 0..STATUS_POLLS {
            steps.push(reply(I2C_ADDRESS_GND, register::CONFIGURATION, 0xC810));
        }
        let mut sensor = Opt3001::new(I2cScript::new(steps), I2C_ADDRESS_GND, DelayLog::new());
        assert_eq!(sensor.measure(), Err(DriverError::Timeout));
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(delay.total_millis(), 800 + 10 * u64::from(STATUS_POLLS));
    }

    #[test]
    fn a_bus_fault_surfaces_as_a_bus_error() {
        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let mut sensor = Opt3001::new(
            I2cScript::new([I2cStep::fault(I2C_ADDRESS_GND, kind)]),
            I2C_ADDRESS_GND,
            DelayLog::new(),
        );
        assert!(matches!(sensor.init(), Err(DriverError::Bus(_))));
    }

    #[test]
    fn limits_are_written_in_the_result_format() {
        let mut steps = init_steps(I2C_ADDRESS_GND, 0xC810);
        steps.push(word(
            I2C_ADDRESS_GND,
            register::LOW_LIMIT,
            raw_from_milli_lux(10_000),
        ));
        steps.push(word(
            I2C_ADDRESS_GND,
            register::HIGH_LIMIT,
            raw_from_milli_lux(5_000_000),
        ));
        let mut sensor = Opt3001::new(I2cScript::new(steps), I2C_ADDRESS_GND, DelayLog::new());
        sensor.init().unwrap();
        sensor.set_limits(10_000, 5_000_000).unwrap();
        assert!(sensor.release().0.done());
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_lux() {
        let mut steps = init_steps(I2C_ADDRESS_GND, 0xC810);
        steps.extend([
            word(I2C_ADDRESS_GND, register::CONFIGURATION, 0xCA10),
            reply(
                I2C_ADDRESS_GND,
                register::CONFIGURATION,
                0xC810 | CONVERSION_READY,
            ),
            reply(I2C_ADDRESS_GND, register::RESULT, 0x0C80),
        ]);
        let sensor = Opt3001::new(I2cScript::new(steps), I2C_ADDRESS_GND, DelayLog::new());
        let mut lux = sensor.map(|reading| reading.lux());
        assert!((block_on(lux.read()).unwrap() - 32.0).abs() < 1e-3);
    }
}
