//! The INA219 driven over I2C: configure, calibrate, trigger, and read all four results.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use pamoja_core::Sensor;

use super::{
    calibration, conversion_ready, minimum_current_lsb_microamps, register, Configuration, Mode,
    Reading,
};
use crate::driver::{read_word, write_word};
use crate::error::{DriverError, SensorError};

/// How many times the Bus Voltage register is polled, 1 ms apart, after the
/// conversion time has elapsed before the conversion is called overdue.
pub const STATUS_POLLS: u8 = 20;

/// An INA219 on an I2C bus, measuring shunt and bus voltage, current, and power on
/// demand.
///
/// [`init`](Ina219::init) resets the part, writes the configuration, programs the
/// calibration register for the shunt and the current resolution, and reads the
/// calibration back, which is the identity check a part without an id register
/// offers. [`measure`](Ina219::measure) writes the triggered shunt-and-bus mode,
/// which starts one conversion of each, waits the conversion time for the ADC
/// settings, polls the conversion-ready flag in the Bus Voltage register, then reads
/// the shunt, current, and power registers, power last, because reading it clears
/// the flag. As a [`Sensor`] its reading is the [`Reading`]; one quantity is
/// selected with [`Sensor::map`].
///
/// # Examples
///
/// A 100 mΩ shunt on a 12 V bus carrying 1 A, with the part expected to see up to
/// 3.2 A, the design example the datasheet works through.
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
/// use pamoja_sensors::ina219::{
///     bus_register, current_register, power_register, shunt_register, Ina219, BASE_ADDRESS,
/// };
///
/// const PART: u8 = BASE_ADDRESS;
/// const LSB: u32 = 98;
/// let bus = I2cScript::new([
///     I2cStep::write(PART, [0x00, 0xB9, 0x9F]),
///     I2cStep::write(PART, [0x00, 0x39, 0x9F]),
///     I2cStep::write(PART, [0x05, 0x10, 0x53]),
///     I2cStep::write_read(PART, [0x05], [0x10, 0x53]),
///     I2cStep::write(PART, [0x00, 0x39, 0x9B]),
///     I2cStep::write_read(PART, [0x02], (bus_register(12_000) | 0x02).to_be_bytes()),
///     I2cStep::write_read(PART, [0x01], shunt_register(100_000).to_be_bytes()),
///     I2cStep::write_read(PART, [0x04], current_register(1_000_000, LSB).to_be_bytes()),
///     I2cStep::write_read(PART, [0x03], power_register(12_000_000, LSB).to_be_bytes()),
/// ]);
///
/// let mut monitor = Ina219::new(bus, PART, DelayLog::new()).with_shunt(100, 3_200_000);
/// let reading = block_on(monitor.read())?;
/// assert_eq!(reading.bus_millivolts(), 12_000);
/// assert_eq!(reading.shunt_microvolts(), 100_000);
/// assert!((reading.current_microamps() - 1_000_000).abs() < LSB as i32);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Ina219<I2C, D> {
    bus: I2C,
    delay: D,
    address: u8,
    configuration: Configuration,
    shunt_milliohms: u32,
    current_lsb_microamps: u32,
    initialized: bool,
}

impl<I2C, D> Ina219<I2C, D> {
    /// Wraps an I2C bus and the part's address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `address` - [`BASE_ADDRESS`](super::BASE_ADDRESS) plus what the A1 and A0
    ///   pins add.
    /// * `delay` - the timer that paces the conversions.
    ///
    /// # Returns
    ///
    /// The driver, with the reset configuration and a 100 mΩ shunt sized for 3.2 A,
    /// the common breakout.
    pub fn new(bus: I2C, address: u8, delay: D) -> Self {
        Ina219 {
            bus,
            delay,
            address,
            configuration: Configuration::default(),
            shunt_milliohms: 100,
            current_lsb_microamps: minimum_current_lsb_microamps(3_200_000),
            initialized: false,
        }
    }

    /// Sets the shunt and the largest current expected through it.
    ///
    /// The current resolution is the smallest the datasheet allows for that maximum,
    /// which gives the Current register its finest scale; the calibration register
    /// follows from the two by the datasheet's equation.
    ///
    /// # Arguments
    ///
    /// * `milliohms` - the shunt resistance.
    /// * `max_microamps` - the largest current the shunt will carry.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_shunt(mut self, milliohms: u32, max_microamps: u32) -> Self {
        self.shunt_milliohms = milliohms;
        self.current_lsb_microamps = minimum_current_lsb_microamps(max_microamps);
        self.initialized = false;
        self
    }

    /// Sets the current resolution outright, for a round figure such as 100 µA.
    ///
    /// # Arguments
    ///
    /// * `microamps` - microamps per count of the Current register.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_current_lsb(mut self, microamps: u32) -> Self {
        self.current_lsb_microamps = microamps;
        self.initialized = false;
        self
    }

    /// Sets the range, gain, and ADC settings; the mode is chosen per conversion.
    ///
    /// # Arguments
    ///
    /// * `configuration` - the settings to write.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_configuration(mut self, configuration: Configuration) -> Self {
        self.configuration = configuration;
        self.initialized = false;
        self
    }

    /// Returns the current resolution in microamps per count.
    pub fn current_lsb_microamps(&self) -> u32 {
        self.current_lsb_microamps
    }

    /// Returns the calibration word the driver programs.
    pub fn calibration(&self) -> u16 {
        calibration(self.current_lsb_microamps, self.shunt_milliohms)
    }

    /// Gives back the bus and the delay.
    ///
    /// # Returns
    ///
    /// The bus and delay the driver was built from.
    pub fn release(self) -> (I2C, D) {
        (self.bus, self.delay)
    }

    fn triggered(&self) -> Configuration {
        Configuration {
            reset: false,
            mode: Mode::ShuntAndBusTriggered,
            ..self.configuration
        }
    }
}

impl<I2C: I2c, D: DelayNs> Ina219<I2C, D> {
    /// Resets the part, writes the configuration and calibration, and reads the
    /// calibration back.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, and [`DriverError::Sensor`]
    /// with [`SensorError::Identity`] if the calibration register does not hold what
    /// was written.
    pub fn init(&mut self) -> Result<(), DriverError<I2C::Error>> {
        let reset = Configuration {
            reset: true,
            ..Configuration::default()
        };
        self.write(register::CONFIGURATION, reset.bits())?;
        let settings = Configuration {
            reset: false,
            ..self.configuration
        };
        self.write(register::CONFIGURATION, settings.bits())?;
        let calibration = self.calibration();
        self.write(register::CALIBRATION, calibration)?;
        if self.read(register::CALIBRATION)? != calibration {
            return Err(SensorError::Identity.into());
        }
        self.initialized = true;
        Ok(())
    }

    /// Triggers one shunt and bus conversion and returns every result.
    ///
    /// Initializes the part first if [`init`](Ina219::init) has not run.
    ///
    /// # Returns
    ///
    /// The four registers with the current resolution they scale by.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Timeout`] if
    /// the conversion-ready flag never sets, or any error of [`init`](Ina219::init).
    pub fn measure(&mut self) -> Result<Reading, DriverError<I2C::Error>> {
        if !self.initialized {
            self.init()?;
        }
        let triggered = self.triggered();
        self.write(register::CONFIGURATION, triggered.bits())?;
        self.delay.delay_us(triggered.conversion_micros());

        let mut bus = None;
        for _ in 0..STATUS_POLLS {
            let word = self.read(register::BUS_VOLTAGE)?;
            if conversion_ready(word) {
                bus = Some(word);
                break;
            }
            self.delay.delay_ms(1);
        }
        let bus = bus.ok_or(DriverError::Timeout)?;

        let shunt = self.read(register::SHUNT_VOLTAGE)? as i16;
        let current = self.read(register::CURRENT)? as i16;
        let power = self.read(register::POWER)?;
        Ok(Reading {
            shunt,
            bus,
            current,
            power,
            current_lsb_microamps: self.current_lsb_microamps,
        })
    }

    fn read(&mut self, register: u8) -> Result<u16, DriverError<I2C::Error>> {
        read_word(&mut self.bus, self.address, register).map_err(DriverError::Bus)
    }

    fn write(&mut self, register: u8, value: u16) -> Result<(), DriverError<I2C::Error>> {
        write_word(&mut self.bus, self.address, register, value).map_err(DriverError::Bus)
    }
}

impl<I2C, D> Sensor for Ina219<I2C, D>
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
    use crate::ina219::{
        bus_register, current_register, power_register, shunt_register, Adc, BASE_ADDRESS,
        CONFIG_RESET,
    };
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};

    const LSB: u32 = 98;

    fn word(address: u8, register: u8, value: u16) -> I2cStep {
        let [high, low] = value.to_be_bytes();
        I2cStep::write(address, [register, high, low])
    }

    fn reply(address: u8, register: u8, value: u16) -> I2cStep {
        I2cStep::write_read(address, [register], value.to_be_bytes())
    }

    fn init_steps(address: u8, settings: u16, calibration: u16) -> Vec<I2cStep> {
        vec![
            word(address, register::CONFIGURATION, CONFIG_RESET | 0x8000),
            word(address, register::CONFIGURATION, settings),
            word(address, register::CALIBRATION, calibration),
            reply(address, register::CALIBRATION, calibration),
        ]
    }

    fn measure_steps(address: u8, triggered: u16) -> Vec<I2cStep> {
        vec![
            word(address, register::CONFIGURATION, triggered),
            reply(address, register::BUS_VOLTAGE, bus_register(12_000) | 0x02),
            reply(
                address,
                register::SHUNT_VOLTAGE,
                shunt_register(100_000) as u16,
            ),
            reply(
                address,
                register::CURRENT,
                current_register(1_000_000, LSB) as u16,
            ),
            reply(address, register::POWER, power_register(12_000_000, LSB)),
        ]
    }

    #[test]
    fn init_resets_configures_calibrates_and_reads_the_calibration_back() {
        let calibration = calibration(LSB, 100);
        assert_eq!(calibration, 4179, "the datasheet design example");
        let mut steps = init_steps(BASE_ADDRESS, CONFIG_RESET, calibration);
        steps.extend(measure_steps(BASE_ADDRESS, CONFIG_RESET & !0x0004));
        let mut monitor = Ina219::new(I2cScript::new(steps), BASE_ADDRESS, DelayLog::new())
            .with_shunt(100, 3_200_000);
        assert_eq!(monitor.current_lsb_microamps(), LSB);
        monitor.init().unwrap();
        let reading = monitor.measure().unwrap();
        assert_eq!(reading.bus_millivolts(), 12_000);
        assert_eq!(reading.shunt_microvolts(), 100_000);
        assert_eq!(
            reading.current_microamps(),
            current_register(1_000_000, LSB) as i32 * LSB as i32
        );
        assert!(!reading.math_overflow());
        let (bus, delay) = monitor.release();
        assert!(bus.done());
        assert_eq!(
            delay.waits_ns(),
            [1_064_000],
            "12-bit shunt and bus conversions"
        );
    }

    #[test]
    fn the_adc_settings_set_the_wait_and_reach_the_register() {
        let configuration = Configuration {
            bus_adc: Adc::Samples16,
            shunt_adc: Adc::Samples16,
            ..Configuration::default()
        };
        let settings = configuration.bits();
        let triggered = (settings & !0x0007) | 0x0003;
        let mut steps = init_steps(0x41, settings, calibration(100, 100));
        steps.extend(measure_steps(0x41, triggered));
        let mut monitor = Ina219::new(I2cScript::new(steps), 0x41, DelayLog::new())
            .with_shunt(100, 3_200_000)
            .with_current_lsb(100)
            .with_configuration(configuration);
        monitor.measure().unwrap();
        let (bus, delay) = monitor.release();
        assert!(bus.done());
        assert_eq!(delay.waits_ns(), [17_020_000]);
    }

    #[test]
    fn a_calibration_that_does_not_read_back_is_an_identity_failure() {
        let calibration = calibration(LSB, 100);
        let steps = [
            word(BASE_ADDRESS, register::CONFIGURATION, CONFIG_RESET | 0x8000),
            word(BASE_ADDRESS, register::CONFIGURATION, CONFIG_RESET),
            word(BASE_ADDRESS, register::CALIBRATION, calibration),
            reply(BASE_ADDRESS, register::CALIBRATION, 0x0000),
        ];
        let mut monitor = Ina219::new(I2cScript::new(steps), BASE_ADDRESS, DelayLog::new());
        assert_eq!(
            monitor.init(),
            Err(DriverError::Sensor(SensorError::Identity))
        );
    }

    #[test]
    fn a_conversion_that_never_finishes_is_a_timeout() {
        let calibration = calibration(LSB, 100);
        let mut steps = init_steps(BASE_ADDRESS, CONFIG_RESET, calibration);
        steps.push(word(
            BASE_ADDRESS,
            register::CONFIGURATION,
            CONFIG_RESET & !0x0004,
        ));
        for _ in 0..STATUS_POLLS {
            steps.push(reply(
                BASE_ADDRESS,
                register::BUS_VOLTAGE,
                bus_register(12_000) & !0x0002,
            ));
        }
        let mut monitor = Ina219::new(I2cScript::new(steps), BASE_ADDRESS, DelayLog::new());
        assert_eq!(monitor.measure(), Err(DriverError::Timeout));
        assert!(monitor.release().0.done());
    }

    #[test]
    fn a_bus_fault_surfaces_as_a_bus_error() {
        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let mut monitor = Ina219::new(
            I2cScript::new([I2cStep::fault(BASE_ADDRESS, kind)]),
            BASE_ADDRESS,
            DelayLog::new(),
        );
        assert!(matches!(monitor.init(), Err(DriverError::Bus(_))));
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_one_quantity() {
        let mut steps = init_steps(BASE_ADDRESS, CONFIG_RESET, calibration(LSB, 100));
        steps.extend(measure_steps(BASE_ADDRESS, CONFIG_RESET & !0x0004));
        let monitor = Ina219::new(I2cScript::new(steps), BASE_ADDRESS, DelayLog::new());
        let mut watts = monitor.map(|reading| reading.power_microwatts() as f32 / 1e6);
        let reading = block_on(watts.read()).unwrap();
        assert!((reading - 12.0).abs() < 0.01);
    }
}
