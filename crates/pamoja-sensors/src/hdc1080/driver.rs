//! The HDC1080 driven over I2C: identify, configure, trigger, wait, and read both results.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use pamoja_core::Sensor;

use super::{
    register, AcquisitionMode, Configuration, HumidityResolution, Measurement,
    TemperatureResolution, DEVICE_ID, I2C_ADDRESS, MANUFACTURER_ID,
};
use crate::driver::{read_word, write_word};
use crate::error::{DriverError, SensorError};

/// An HDC1080 on an I2C bus, measuring temperature and humidity on demand.
///
/// [`init`](Hdc1080::init) checks the manufacturer and device id registers and
/// writes the configuration: the mode that measures temperature and humidity in one
/// sequence, at the chosen resolutions. [`measure`](Hdc1080::measure) follows the
/// datasheet's transaction: a pointer write to the temperature register triggers
/// the acquisition, the conversion time for both resolutions passes, and one
/// four-byte read returns temperature then humidity. The part does not acknowledge
/// a read before the results are ready, which surfaces as a bus error rather than a
/// stale value. As a [`Sensor`] its reading is the [`Measurement`]; one channel is
/// selected with [`Sensor::map`].
///
/// # Examples
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
/// use pamoja_sensors::hdc1080::{Hdc1080, Measurement, I2C_ADDRESS};
///
/// const PART: u8 = I2C_ADDRESS;
/// let bus = I2cScript::new([
///     I2cStep::write_read(PART, [0xFE], [0x54, 0x49]),
///     I2cStep::write_read(PART, [0xFF], [0x10, 0x50]),
///     I2cStep::write(PART, [0x02, 0x10, 0x00]),
///     I2cStep::write(PART, [0x00]),
///     I2cStep::read(PART, Measurement::from_physical(21_500, 45_000).to_bytes()),
/// ]);
///
/// let mut sensor = Hdc1080::new(bus, DelayLog::new());
/// let measurement = block_on(sensor.read())?;
/// assert_eq!(measurement, Measurement::from_physical(21_500, 45_000));
/// assert!((measurement.milli_celsius() - 21_500).abs() < 10);
/// let (bus, delay) = sensor.release();
/// assert!(bus.done());
/// assert_eq!(delay.total_micros(), 6_350 + 6_500);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Hdc1080<I2C, D> {
    bus: I2C,
    delay: D,
    configuration: Configuration,
    initialized: bool,
}

impl<I2C, D> Hdc1080<I2C, D> {
    /// Wraps an I2C bus; the part has one address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `delay` - the timer that paces the acquisitions.
    ///
    /// # Returns
    ///
    /// The driver, measuring both channels at 14 bits.
    pub fn new(bus: I2C, delay: D) -> Self {
        Hdc1080 {
            bus,
            delay,
            configuration: Configuration {
                mode: AcquisitionMode::TemperatureThenHumidity,
                ..Configuration::default()
            },
            initialized: false,
        }
    }

    /// Sets the resolution of each channel, which sets the conversion time.
    ///
    /// # Arguments
    ///
    /// * `temperature` - 11 or 14 bits.
    /// * `humidity` - 8, 11, or 14 bits.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_resolutions(
        mut self,
        temperature: TemperatureResolution,
        humidity: HumidityResolution,
    ) -> Self {
        self.configuration.temperature_resolution = temperature;
        self.configuration.humidity_resolution = humidity;
        self.initialized = false;
        self
    }

    /// Returns the configuration the driver writes.
    pub fn configuration(&self) -> Configuration {
        self.configuration
    }

    /// Gives back the bus and the delay.
    ///
    /// # Returns
    ///
    /// The bus and delay the driver was built from.
    pub fn release(self) -> (I2C, D) {
        (self.bus, self.delay)
    }
}

impl<I2C: I2c, D: DelayNs> Hdc1080<I2C, D> {
    /// Checks the part's identity and writes the configuration.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, and [`DriverError::Sensor`]
    /// with [`SensorError::Identity`] if either id register is not an HDC1080's.
    pub fn init(&mut self) -> Result<(), DriverError<I2C::Error>> {
        if self.read(register::MANUFACTURER_ID)? != MANUFACTURER_ID
            || self.read(register::DEVICE_ID)? != DEVICE_ID
        {
            return Err(SensorError::Identity.into());
        }
        self.write_configuration()?;
        self.initialized = true;
        Ok(())
    }

    /// Triggers one acquisition of both channels and reads the results.
    ///
    /// Initializes the part first if [`init`](Hdc1080::init) has not run.
    ///
    /// # Returns
    ///
    /// The temperature and humidity registers as a [`Measurement`].
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, which includes the part not
    /// acknowledging a read before its results are ready, or any error of
    /// [`init`](Hdc1080::init).
    pub fn measure(&mut self) -> Result<Measurement, DriverError<I2C::Error>> {
        if !self.initialized {
            self.init()?;
        }
        self.bus
            .write(I2C_ADDRESS, &[register::TEMPERATURE])
            .map_err(DriverError::Bus)?;
        self.delay
            .delay_us(self.configuration.conversion_time_micros());
        let mut bytes = [0u8; 4];
        self.bus
            .read(I2C_ADDRESS, &mut bytes)
            .map_err(DriverError::Bus)?;
        Ok(Measurement::parse(&bytes))
    }

    /// Switches the heater, which runs only during acquisitions, on or off.
    ///
    /// # Arguments
    ///
    /// * `on` - whether the heater runs.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn heater(&mut self, on: bool) -> Result<(), DriverError<I2C::Error>> {
        self.configuration.heater = on;
        self.write_configuration()
    }

    fn write_configuration(&mut self) -> Result<(), DriverError<I2C::Error>> {
        write_word(
            &mut self.bus,
            I2C_ADDRESS,
            register::CONFIGURATION,
            self.configuration.to_register(),
        )
        .map_err(DriverError::Bus)
    }

    fn read(&mut self, register: u8) -> Result<u16, DriverError<I2C::Error>> {
        read_word(&mut self.bus, I2C_ADDRESS, register).map_err(DriverError::Bus)
    }
}

impl<I2C, D> Sensor for Hdc1080<I2C, D>
where
    I2C: I2c + Send,
    I2C::Error: core::fmt::Debug,
    D: DelayNs + Send,
{
    type Reading = Measurement;

    async fn read(&mut self) -> pamoja_core::Result<Measurement> {
        self.measure().map_err(pamoja_core::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};

    fn init_steps(configuration: u16) -> Vec<I2cStep> {
        let [high, low] = configuration.to_be_bytes();
        vec![
            I2cStep::write_read(
                I2C_ADDRESS,
                [register::MANUFACTURER_ID],
                MANUFACTURER_ID.to_be_bytes(),
            ),
            I2cStep::write_read(I2C_ADDRESS, [register::DEVICE_ID], DEVICE_ID.to_be_bytes()),
            I2cStep::write(I2C_ADDRESS, [register::CONFIGURATION, high, low]),
        ]
    }

    #[test]
    fn a_measurement_is_triggered_by_the_pointer_write_and_read_after_the_conversion() {
        let mut steps = init_steps(0x1000);
        steps.push(I2cStep::write(I2C_ADDRESS, [register::TEMPERATURE]));
        steps.push(I2cStep::read(
            I2C_ADDRESS,
            Measurement::from_physical(21_500, 45_000).to_bytes(),
        ));
        let mut sensor = Hdc1080::new(I2cScript::new(steps), DelayLog::new());
        let measurement = sensor.measure().unwrap();
        assert_eq!(measurement, Measurement::from_physical(21_500, 45_000));
        assert!((measurement.milli_celsius() - 21_500).abs() < 10);
        assert!(measurement.milli_percent().abs_diff(45_000) < 10);
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(
            delay.waits_ns(),
            [12_850_000],
            "14-bit temperature plus 14-bit humidity"
        );
    }

    #[test]
    fn lower_resolutions_shorten_the_wait_and_reach_the_configuration() {
        let configuration = Configuration {
            mode: AcquisitionMode::TemperatureThenHumidity,
            temperature_resolution: TemperatureResolution::Bits11,
            humidity_resolution: HumidityResolution::Bits8,
            ..Configuration::default()
        };
        let mut steps = init_steps(configuration.to_register());
        steps.push(I2cStep::write(I2C_ADDRESS, [register::TEMPERATURE]));
        steps.push(I2cStep::read(
            I2C_ADDRESS,
            Measurement::from_physical(0, 0).to_bytes(),
        ));
        let mut sensor = Hdc1080::new(I2cScript::new(steps), DelayLog::new())
            .with_resolutions(TemperatureResolution::Bits11, HumidityResolution::Bits8);
        assert_eq!(sensor.configuration(), configuration);
        sensor.measure().unwrap();
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(delay.waits_ns(), [(3_650 + 2_500) * 1_000]);
    }

    #[test]
    fn a_part_with_the_wrong_id_is_refused_at_either_register() {
        let steps = [
            I2cStep::write_read(I2C_ADDRESS, [register::MANUFACTURER_ID], [0x54, 0x49]),
            I2cStep::write_read(I2C_ADDRESS, [register::DEVICE_ID], [0x10, 0x00]),
        ];
        let mut sensor = Hdc1080::new(I2cScript::new(steps), DelayLog::new());
        assert_eq!(
            sensor.init(),
            Err(DriverError::Sensor(SensorError::Identity))
        );
        let steps = [I2cStep::write_read(
            I2C_ADDRESS,
            [register::MANUFACTURER_ID],
            [0x00, 0x00],
        )];
        let mut sensor = Hdc1080::new(I2cScript::new(steps), DelayLog::new());
        assert_eq!(
            sensor.init(),
            Err(DriverError::Sensor(SensorError::Identity))
        );
    }

    #[test]
    fn a_read_the_part_does_not_acknowledge_is_a_bus_error() {
        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let mut steps = init_steps(0x1000);
        steps.push(I2cStep::write(I2C_ADDRESS, [register::TEMPERATURE]));
        steps.push(I2cStep::fault(I2C_ADDRESS, kind));
        let mut sensor = Hdc1080::new(I2cScript::new(steps), DelayLog::new());
        assert!(matches!(sensor.measure(), Err(DriverError::Bus(_))));
        assert!(sensor.release().0.done());
    }

    #[test]
    fn the_heater_bit_is_written_through_the_configuration() {
        let mut steps = init_steps(0x1000);
        steps.push(I2cStep::write(
            I2C_ADDRESS,
            [register::CONFIGURATION, 0x30, 0x00],
        ));
        let mut sensor = Hdc1080::new(I2cScript::new(steps), DelayLog::new());
        sensor.init().unwrap();
        sensor.heater(true).unwrap();
        assert!(sensor.configuration().heater);
        assert!(sensor.release().0.done());
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_humidity() {
        let mut steps = init_steps(0x1000);
        steps.push(I2cStep::write(I2C_ADDRESS, [register::TEMPERATURE]));
        steps.push(I2cStep::read(
            I2C_ADDRESS,
            Measurement::from_physical(21_500, 45_000).to_bytes(),
        ));
        let sensor = Hdc1080::new(I2cScript::new(steps), DelayLog::new());
        let mut humidity = sensor.map(|measurement| measurement.relative_humidity());
        assert!((block_on(humidity.read()).unwrap() - 45.0).abs() < 0.01);
    }
}
