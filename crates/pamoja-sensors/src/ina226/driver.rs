//! The INA226 driven over I2C: identify, configure, calibrate, trigger, and read.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use pamoja_core::Sensor;

use super::{
    calibration, identify, minimum_current_lsb_microamps, register, Configuration, DieId,
    MaskEnable, Mode, Reading,
};
use crate::driver::{read_word, write_word};
use crate::error::{DriverError, SensorError};

/// How many times the Mask/Enable register is polled, 1 ms apart, after the
/// conversion time has elapsed before the conversion is called overdue.
pub const STATUS_POLLS: u8 = 20;

/// An INA226 on an I2C bus, measuring shunt and bus voltage, current, and power on
/// demand.
///
/// [`init`](Ina226::init) resets the part, checks the manufacturer and die id
/// registers, writes the configuration, programs the calibration register for the
/// shunt and the current resolution, and reads the calibration back.
/// [`measure`](Ina226::measure) writes the triggered shunt-and-bus mode, which
/// starts one conversion of each, waits the conversion time for the averaging and
/// conversion-time settings, polls the conversion-ready flag in the Mask/Enable
/// register, then reads the four data registers. Reading the Mask/Enable register
/// clears the flag, which is why the poll is the only place it is read. As a
/// [`Sensor`] its reading is the [`Reading`]; one quantity is selected with
/// [`Sensor::map`].
///
/// # Examples
///
/// A 100 mΩ shunt on a 12 V bus carrying 1 A, sized for 3.2 A at most.
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
/// use pamoja_sensors::ina226::{
///     bus_register, current_register, power_register, shunt_register, Ina226, BASE_ADDRESS,
/// };
///
/// const PART: u8 = BASE_ADDRESS;
/// const LSB: u32 = 98;
/// let bus = I2cScript::new([
///     I2cStep::write(PART, [0x00, 0xC1, 0x27]),
///     I2cStep::write_read(PART, [0xFE], [0x54, 0x49]),
///     I2cStep::write_read(PART, [0xFF], [0x22, 0x60]),
///     I2cStep::write(PART, [0x00, 0x41, 0x27]),
///     I2cStep::write(PART, [0x05, 0x02, 0x0A]),
///     I2cStep::write_read(PART, [0x05], [0x02, 0x0A]),
///     I2cStep::write(PART, [0x00, 0x41, 0x23]),
///     I2cStep::write_read(PART, [0x06], [0x00, 0x08]),
///     I2cStep::write_read(PART, [0x01], shunt_register(50_000_000).to_be_bytes()),
///     I2cStep::write_read(PART, [0x02], bus_register(12_000_000).to_be_bytes()),
///     I2cStep::write_read(PART, [0x04], current_register(1_000_000, LSB).to_be_bytes()),
///     I2cStep::write_read(PART, [0x03], power_register(12_000_000, LSB).to_be_bytes()),
/// ]);
///
/// let mut monitor = Ina226::new(bus, PART, DelayLog::new()).with_shunt(100, 3_200_000);
/// let reading = block_on(monitor.read())?;
/// assert_eq!(reading.bus_microvolts(), 12_000_000);
/// assert!((reading.current_amps() - 1.0).abs() < 0.001);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Ina226<I2C, D> {
    bus: I2C,
    delay: D,
    address: u8,
    configuration: Configuration,
    shunt_milliohms: u32,
    current_lsb_microamps: u32,
    die: Option<DieId>,
}

impl<I2C, D> Ina226<I2C, D> {
    /// Wraps an I2C bus and the part's address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `address` - the address the A1 and A0 pins select, from
    ///   [`address`](super::address).
    /// * `delay` - the timer that paces the conversions.
    ///
    /// # Returns
    ///
    /// The driver, with the reset configuration and a 100 mΩ shunt sized for 3.2 A.
    pub fn new(bus: I2C, address: u8, delay: D) -> Self {
        Ina226 {
            bus,
            delay,
            address,
            configuration: Configuration::RESET,
            shunt_milliohms: 100,
            current_lsb_microamps: minimum_current_lsb_microamps(3_200_000),
            die: None,
        }
    }

    /// Sets the shunt and the largest current expected through it.
    ///
    /// The current resolution is the smallest the datasheet allows for that maximum;
    /// the calibration register follows from the two by the datasheet's equation.
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
        self.die = None;
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
        self.die = None;
        self
    }

    /// Sets the averaging and conversion times; the mode is chosen per conversion.
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
        self.die = None;
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

    /// Returns the die id read at initialization, if it has happened.
    pub fn die_id(&self) -> Option<DieId> {
        self.die
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

impl<I2C: I2c, D: DelayNs> Ina226<I2C, D> {
    /// Resets the part, checks its identity, writes the configuration and
    /// calibration, and reads the calibration back.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, and [`DriverError::Sensor`]
    /// with [`SensorError::Identity`] if the id registers are not an INA226's or the
    /// calibration register does not hold what was written.
    pub fn init(&mut self) -> Result<(), DriverError<I2C::Error>> {
        let reset = Configuration {
            reset: true,
            ..Configuration::RESET
        };
        self.write(register::CONFIGURATION, reset.to_register())?;
        let manufacturer = self.read(register::MANUFACTURER_ID)?;
        let die = self.read(register::DIE_ID)?;
        let die = identify(manufacturer, die)?;

        let settings = Configuration {
            reset: false,
            ..self.configuration
        };
        self.write(register::CONFIGURATION, settings.to_register())?;
        let calibration = self.calibration();
        self.write(register::CALIBRATION, calibration)?;
        if self.read(register::CALIBRATION)? != calibration {
            return Err(SensorError::Identity.into());
        }
        self.die = Some(die);
        Ok(())
    }

    /// Triggers one shunt and bus conversion and returns every result.
    ///
    /// Initializes the part first if [`init`](Ina226::init) has not run.
    ///
    /// # Returns
    ///
    /// The four registers with the current resolution they scale by.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Timeout`] if
    /// the conversion-ready flag never sets, or any error of [`init`](Ina226::init).
    pub fn measure(&mut self) -> Result<Reading, DriverError<I2C::Error>> {
        if self.die.is_none() {
            self.init()?;
        }
        let triggered = self.triggered();
        self.write(register::CONFIGURATION, triggered.to_register())?;
        self.delay.delay_us(triggered.update_microseconds());

        let mut status = None;
        for _ in 0..STATUS_POLLS {
            let mask = MaskEnable::from_register(self.read(register::MASK_ENABLE)?);
            if mask.conversion_ready_flag {
                status = Some(mask);
                break;
            }
            self.delay.delay_ms(1);
        }
        let status = status.ok_or(DriverError::Timeout)?;

        let shunt = self.read(register::SHUNT_VOLTAGE)? as i16;
        let bus = self.read(register::BUS_VOLTAGE)?;
        let current = self.read(register::CURRENT)? as i16;
        let power = self.read(register::POWER)?;
        Ok(Reading {
            shunt,
            bus,
            current,
            power,
            current_lsb_microamps: self.current_lsb_microamps,
            math_overflow: status.math_overflow,
        })
    }

    /// Programs the alert pin: which limit it watches and the limit itself.
    ///
    /// # Arguments
    ///
    /// * `mask` - the Mask/Enable settings; one alert function at a time.
    /// * `limit` - the Alert Limit register value, in the units of the register the
    ///   function compares against.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn set_alert(
        &mut self,
        mask: MaskEnable,
        limit: u16,
    ) -> Result<(), DriverError<I2C::Error>> {
        self.write(register::ALERT_LIMIT, limit)?;
        self.write(register::MASK_ENABLE, mask.to_register())
    }

    fn read(&mut self, register: u8) -> Result<u16, DriverError<I2C::Error>> {
        read_word(&mut self.bus, self.address, register).map_err(DriverError::Bus)
    }

    fn write(&mut self, register: u8, value: u16) -> Result<(), DriverError<I2C::Error>> {
        write_word(&mut self.bus, self.address, register, value).map_err(DriverError::Bus)
    }
}

impl<I2C, D> Sensor for Ina226<I2C, D>
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
    use crate::ina226::{
        bus_register, current_register, power_register, shunt_register, Averaging, ConversionTime,
        BASE_ADDRESS, CONFIG_RESET, MANUFACTURER_ID,
    };
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};

    const LSB: u32 = 98;
    const DIE: u16 = 0x2260;

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
            reply(address, register::MANUFACTURER_ID, MANUFACTURER_ID),
            reply(address, register::DIE_ID, DIE),
            word(address, register::CONFIGURATION, settings),
            word(address, register::CALIBRATION, calibration),
            reply(address, register::CALIBRATION, calibration),
        ]
    }

    fn measure_steps(address: u8, triggered: u16, mask: u16) -> Vec<I2cStep> {
        vec![
            word(address, register::CONFIGURATION, triggered),
            reply(address, register::MASK_ENABLE, mask),
            reply(
                address,
                register::SHUNT_VOLTAGE,
                shunt_register(50_000_000) as u16,
            ),
            reply(address, register::BUS_VOLTAGE, bus_register(12_000_000)),
            reply(
                address,
                register::CURRENT,
                current_register(1_000_000, LSB) as u16,
            ),
            reply(address, register::POWER, power_register(12_000_000, LSB)),
        ]
    }

    #[test]
    fn init_resets_identifies_configures_and_calibrates_then_measure_reads_all_four() {
        let calibration = calibration(LSB, 100);
        let mut steps = init_steps(BASE_ADDRESS, CONFIG_RESET, calibration);
        steps.extend(measure_steps(BASE_ADDRESS, 0x4123, 0x0008));
        let mut monitor = Ina226::new(I2cScript::new(steps), BASE_ADDRESS, DelayLog::new())
            .with_shunt(100, 3_200_000);
        assert_eq!(monitor.die_id(), None);
        monitor.init().unwrap();
        assert_eq!(monitor.die_id().map(|die| die.device), Some(0x226));

        let reading = monitor.measure().unwrap();
        assert_eq!(reading.bus_microvolts(), 12_000_000);
        assert_eq!(reading.shunt_nanovolts(), 50_000_000);
        assert!(!reading.math_overflow);
        let (bus, delay) = monitor.release();
        assert!(bus.done());
        assert_eq!(
            delay.waits_ns(),
            [2_200_000],
            "two 1.1 ms conversions, one sample"
        );
    }

    #[test]
    fn averaging_and_conversion_times_set_the_wait_and_the_overflow_flag_is_kept() {
        let configuration = Configuration {
            averaging: Averaging::Samples16,
            bus_conversion_time: ConversionTime::Us8244,
            shunt_conversion_time: ConversionTime::Us8244,
            ..Configuration::RESET
        };
        let settings = configuration.to_register();
        let triggered = (settings & !0x0007) | 0x0003;
        let mut steps = init_steps(0x45, settings, calibration(100, 100));
        steps.extend(measure_steps(0x45, triggered, 0x0008 | 0x0004));
        let mut monitor = Ina226::new(I2cScript::new(steps), 0x45, DelayLog::new())
            .with_shunt(100, 3_200_000)
            .with_current_lsb(100)
            .with_configuration(configuration);
        let reading = monitor.measure().unwrap();
        assert!(reading.math_overflow);
        let (bus, delay) = monitor.release();
        assert!(bus.done());
        let expected = Configuration {
            mode: Mode::ShuntAndBusTriggered,
            ..configuration
        }
        .update_microseconds();
        assert_eq!(delay.waits_ns(), [expected * 1_000]);
        assert_eq!(expected, 2 * 8_244 * 16);
    }

    #[test]
    fn a_part_that_is_not_an_ina226_is_refused_at_the_id_registers() {
        let steps = [
            word(BASE_ADDRESS, register::CONFIGURATION, CONFIG_RESET | 0x8000),
            reply(BASE_ADDRESS, register::MANUFACTURER_ID, MANUFACTURER_ID),
            reply(BASE_ADDRESS, register::DIE_ID, 0x2190),
        ];
        let mut monitor = Ina226::new(I2cScript::new(steps), BASE_ADDRESS, DelayLog::new());
        assert_eq!(
            monitor.init(),
            Err(DriverError::Sensor(SensorError::Identity))
        );
        assert!(monitor.release().0.done());
    }

    #[test]
    fn a_conversion_that_never_finishes_is_a_timeout() {
        let mut steps = init_steps(BASE_ADDRESS, CONFIG_RESET, calibration(LSB, 100));
        steps.push(word(BASE_ADDRESS, register::CONFIGURATION, 0x4123));
        for _ in 0..STATUS_POLLS {
            steps.push(reply(BASE_ADDRESS, register::MASK_ENABLE, 0x0000));
        }
        let mut monitor = Ina226::new(I2cScript::new(steps), BASE_ADDRESS, DelayLog::new());
        assert_eq!(monitor.measure(), Err(DriverError::Timeout));
        assert!(monitor.release().0.done());
    }

    #[test]
    fn a_bus_fault_surfaces_as_a_bus_error() {
        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let mut monitor = Ina226::new(
            I2cScript::new([I2cStep::fault(BASE_ADDRESS, kind)]),
            BASE_ADDRESS,
            DelayLog::new(),
        );
        assert!(matches!(monitor.init(), Err(DriverError::Bus(_))));
    }

    #[test]
    fn an_alert_writes_the_limit_then_the_mask() {
        let mut steps = init_steps(BASE_ADDRESS, CONFIG_RESET, calibration(LSB, 100));
        let mask = MaskEnable {
            bus_over_limit: true,
            ..MaskEnable::default()
        };
        steps.push(word(
            BASE_ADDRESS,
            register::ALERT_LIMIT,
            bus_register(13_000_000),
        ));
        steps.push(word(
            BASE_ADDRESS,
            register::MASK_ENABLE,
            mask.to_register(),
        ));
        let mut monitor = Ina226::new(I2cScript::new(steps), BASE_ADDRESS, DelayLog::new());
        monitor.init().unwrap();
        monitor.set_alert(mask, bus_register(13_000_000)).unwrap();
        assert!(monitor.release().0.done());
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_one_quantity() {
        let mut steps = init_steps(BASE_ADDRESS, CONFIG_RESET, calibration(LSB, 100));
        steps.extend(measure_steps(BASE_ADDRESS, 0x4123, 0x0008));
        let monitor = Ina226::new(I2cScript::new(steps), BASE_ADDRESS, DelayLog::new());
        let mut volts = monitor.map(|reading| reading.bus_volts());
        assert!((block_on(volts.read()).unwrap() - 12.0).abs() < 0.001);
    }
}
