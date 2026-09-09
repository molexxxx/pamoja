//! The TMP117 driven over I2C: identify, configure, and convert on demand.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use pamoja_core::Sensor;

use super::{
    data_ready, device_id, eeprom_unlock_busy, raw_from_celsius, register, revision, Alerts,
    Averaging, Configuration, ConversionMode, Reading, DEVICE_ID,
};
use crate::driver::{read_word, write_word};
use crate::error::{DriverError, SensorError};

/// How many times a status register is polled, 1 ms apart, before the part is called
/// overdue: the EEPROM unlock register at initialization, for the power-on load to
/// finish, and the configuration register after a conversion starts, for its
/// data-ready flag. One conversion takes 15.5 ms typical and 17.5 ms at most
/// (section 6.5), so 64 averaged conversions can outlast the typical wait by 128 ms;
/// the polls cover that with room to spare.
pub const STATUS_POLLS: u8 = 150;

/// A TMP117 on an I2C bus, converting on demand in one-shot mode.
///
/// [`Tmp117::new`] wraps the bus. [`init`](Tmp117::init) checks the device id, waits
/// for the EEPROM to finish loading, and writes the configuration with the part in
/// shutdown and the chosen averaging. [`measure`](Tmp117::measure) starts one
/// conversion, waits the conversion time for that averaging, polls the data-ready
/// flag, and returns the temperature register as a [`Reading`]; the part powers
/// itself down again once the conversion is done. The alert limits are written with
/// [`set_alert_limits`](Tmp117::set_alert_limits) and their flags read with
/// [`alerts`](Tmp117::alerts). As a [`Sensor`] its reading is the [`Reading`]; one
/// unit is selected with [`Sensor::map`].
///
/// # Examples
///
/// The part's side of the conversation, scripted: what the datasheet says a TMP117
/// answers during initialization and one conversion that lands on 25 °C.
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
/// use pamoja_sensors::tmp117::{address, Tmp117};
///
/// const PART: u8 = address::ADD0_GND;
/// let bus = I2cScript::new([
///     I2cStep::write_read(PART, [0x0F], [0x01, 0x17]),
///     I2cStep::write_read(PART, [0x04], [0x00, 0x00]),
///     I2cStep::write(PART, [0x01, 0x06, 0x20]),
///     I2cStep::write(PART, [0x01, 0x0E, 0x20]),
///     I2cStep::write_read(PART, [0x01], [0x0E, 0x20]),
///     I2cStep::write_read(PART, [0x01], [0x26, 0x20]),
///     I2cStep::write_read(PART, [0x00], [0x0C, 0x80]),
/// ]);
///
/// let mut sensor = Tmp117::new(bus, PART, DelayLog::new());
/// let reading = block_on(sensor.read())?;
/// assert_eq!(reading.celsius(), 25.0);
///
/// let (bus, delay) = sensor.release();
/// assert!(bus.done());
/// assert_eq!(delay.total_micros(), 125_000);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Tmp117<I2C, D> {
    bus: I2C,
    address: u8,
    delay: D,
    averaging: Averaging,
    revision: Option<u8>,
    pending: Alerts,
}

impl<I2C, D> Tmp117<I2C, D> {
    /// Wraps an I2C bus and the part's address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `address` - one of the [`address`](super::address) constants, chosen by
    ///   where the ADD0 pin is tied.
    /// * `delay` - the timer that paces the conversions.
    ///
    /// # Returns
    ///
    /// The driver, averaging 8 conversions into each result, the factory setting.
    pub fn new(bus: I2C, address: u8, delay: D) -> Self {
        Tmp117 {
            bus,
            address,
            delay,
            averaging: Averaging::X8,
            revision: None,
            pending: Alerts::default(),
        }
    }

    /// Sets how many conversions are averaged into each result.
    ///
    /// The setting is written to the part by [`init`](Tmp117::init) and with every
    /// conversion; more averaging lengthens the conversion, see
    /// [`Averaging::conversion_micros`].
    ///
    /// # Arguments
    ///
    /// * `averaging` - the number of conversions per result.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_averaging(mut self, averaging: Averaging) -> Self {
        self.averaging = averaging;
        self
    }

    /// Returns the silicon revision read at initialization, if it has happened.
    pub fn revision(&self) -> Option<u8> {
        self.revision
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

impl<I2C: I2c, D: DelayNs> Tmp117<I2C, D> {
    /// Checks the part's identity, waits for its EEPROM to finish loading, and writes
    /// the settings with the part in shutdown.
    ///
    /// The identity is the device id field of the Device_ID register (section
    /// 7.6.11). The EEPROM_Busy flag of the EEPROM unlock register is then polled
    /// until it clears, because the part ignores writes to the configuration and limit
    /// registers while its power-on load runs (section 7.5.1.1). The configuration
    /// register (Table 7-6) is written last, with the shutdown mode and the averaging:
    /// that ends any continuous conversion and leaves the part powered down between
    /// the one-shot conversions [`measure`](Tmp117::measure) starts (sections 7.4.2
    /// and 7.4.3).
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Sensor`] with
    /// [`SensorError::Identity`] if the device id is not a TMP117's, and
    /// [`DriverError::Timeout`] if the EEPROM never reports ready.
    pub fn init(&mut self) -> Result<(), DriverError<I2C::Error>> {
        let id = self.read(register::DEVICE_ID)?;
        if device_id(id) != DEVICE_ID {
            return Err(SensorError::Identity.into());
        }
        self.wait(register::EEPROM_UL, eeprom_unlock_busy)?;
        self.write(
            register::CONFIGURATION,
            self.configuration(ConversionMode::Shutdown).bits(),
        )?;
        self.revision = Some(revision(id));
        Ok(())
    }

    /// Runs one conversion and returns the temperature.
    ///
    /// Initializes the part first if [`init`](Tmp117::init) has not run. The
    /// configuration register is written with the one-shot mode and the averaging,
    /// which starts the conversion (section 7.4.3), and read back once so that a
    /// Data_Ready flag left over from an earlier conversion is cleared rather than
    /// trusted, since only a read clears it (Table 7-6). The wait is the conversion
    /// time for the averaging in use, the no-standby cycle of Table 7-7, after which
    /// the configuration register is polled until Data_Ready is set and the
    /// temperature register is read (section 7.6.2).
    ///
    /// # Returns
    ///
    /// The temperature register as a [`Reading`].
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Timeout`] if the
    /// part never reports data ready, or any error of [`init`](Tmp117::init).
    pub fn measure(&mut self) -> Result<Reading, DriverError<I2C::Error>> {
        if self.revision.is_none() {
            self.init()?;
        }
        self.write(
            register::CONFIGURATION,
            self.configuration(ConversionMode::OneShot).bits(),
        )?;
        let stale = self.read(register::CONFIGURATION)?;
        self.pending = self.pending | Alerts::from_bits(stale);
        self.delay.delay_us(self.averaging.conversion_micros());
        let status = self.wait(register::CONFIGURATION, |config| !data_ready(config))?;
        self.pending = self.pending | Alerts::from_bits(status);
        let word = self.read(register::TEMP_RESULT)?;
        Ok(Reading::new(word as i16))
    }

    /// Writes the high and low limit registers.
    ///
    /// The limits share the temperature register's format, 7.8125 m°C per count over
    /// ±256 °C (sections 7.6.4 and 7.6.5); each is rounded to the nearest count and
    /// saturated at the register range. At the end of every conversion the part sets
    /// HIGH_Alert when the result is above the high limit and LOW_Alert when it is
    /// below the low limit (section 7.4.4.1); [`alerts`](Tmp117::alerts) reads them.
    ///
    /// # Arguments
    ///
    /// * `high_celsius` - the high limit in degrees Celsius; the factory value is
    ///   192 °C.
    /// * `low_celsius` - the low limit in degrees Celsius; the factory value is
    ///   -256 °C.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn set_alert_limits(
        &mut self,
        high_celsius: f32,
        low_celsius: f32,
    ) -> Result<(), DriverError<I2C::Error>> {
        self.write(register::THIGH_LIMIT, raw_from_celsius(high_celsius) as u16)?;
        self.write(register::TLOW_LIMIT, raw_from_celsius(low_celsius) as u16)
    }

    /// Reads the HIGH_Alert and LOW_Alert flags: whether a result since the previous
    /// call was above the high limit or below the low limit.
    ///
    /// The part sets the flags at the end of every conversion and, in alert mode,
    /// clears them whenever the configuration register is read (Table 7-6), the reads
    /// [`measure`](Tmp117::measure) makes to see a conversion finish included. The
    /// driver keeps the flags those reads consumed, folds them into this answer, and
    /// forgets them, so the register's read-to-clear behavior holds across the
    /// driver's own reads.
    ///
    /// # Returns
    ///
    /// The two flags.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn alerts(&mut self) -> Result<Alerts, DriverError<I2C::Error>> {
        let live = Alerts::from_bits(self.read(register::CONFIGURATION)?);
        Ok(live | core::mem::take(&mut self.pending))
    }

    fn configuration(&self, mode: ConversionMode) -> Configuration {
        Configuration {
            mode,
            averaging: self.averaging,
            ..Configuration::default()
        }
    }

    fn read(&mut self, register: u8) -> Result<u16, DriverError<I2C::Error>> {
        read_word(&mut self.bus, self.address, register).map_err(DriverError::Bus)
    }

    fn write(&mut self, register: u8, value: u16) -> Result<(), DriverError<I2C::Error>> {
        write_word(&mut self.bus, self.address, register, value).map_err(DriverError::Bus)
    }

    fn wait(
        &mut self,
        register: u8,
        busy: fn(u16) -> bool,
    ) -> Result<u16, DriverError<I2C::Error>> {
        for _ in 0..STATUS_POLLS {
            let word = self.read(register)?;
            if !busy(word) {
                return Ok(word);
            }
            self.delay.delay_ms(1);
        }
        Err(DriverError::Timeout)
    }
}

impl<I2C, D> Sensor for Tmp117<I2C, D>
where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    D: DelayNs,
{
    type Reading = Reading;

    async fn read(&mut self) -> pamoja_core::Result<Reading> {
        self.measure().map_err(pamoja_core::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmp117::address::{ADD0_GND, ADD0_SCL};
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};

    // Table 7-15: revision 0, device id 117h.
    const ID: [u8; 2] = [0x01, 0x17];
    // Table 7-10: EUN clear and EEPROM_Busy clear.
    const EEPROM_READY: [u8; 2] = [0x00, 0x00];
    // Table 7-1: 25 °C is 0C80h.
    const AT_25_C: [u8; 2] = [0x0C, 0x80];
    // Table 7-6: Data_Ready is bit 13, HIGH_Alert bit 15, LOW_Alert bit 14.
    const DATA_READY: u16 = 1 << 13;
    const HIGH_ALERT: u16 = 1 << 15;
    const LOW_ALERT: u16 = 1 << 14;

    fn config(mode: ConversionMode, averaging: Averaging) -> u16 {
        Configuration {
            mode,
            averaging,
            ..Configuration::default()
        }
        .bits()
    }

    fn write(address: u8, register: u8, value: u16) -> I2cStep {
        let [high, low] = value.to_be_bytes();
        I2cStep::write(address, [register, high, low])
    }

    fn read(address: u8, register: u8, value: u16) -> I2cStep {
        I2cStep::write_read(address, [register], value.to_be_bytes())
    }

    fn init_steps(address: u8, averaging: Averaging) -> Vec<I2cStep> {
        vec![
            I2cStep::write_read(address, [register::DEVICE_ID], ID),
            I2cStep::write_read(address, [register::EEPROM_UL], EEPROM_READY),
            write(
                address,
                register::CONFIGURATION,
                config(ConversionMode::Shutdown, averaging),
            ),
        ]
    }

    // The one-shot write, the read that clears a stale flag, one poll answered with
    // `status`, and the result.
    fn measure_steps(
        address: u8,
        averaging: Averaging,
        status: u16,
        result: [u8; 2],
    ) -> Vec<I2cStep> {
        let one_shot = config(ConversionMode::OneShot, averaging);
        vec![
            write(address, register::CONFIGURATION, one_shot),
            read(address, register::CONFIGURATION, one_shot),
            read(address, register::CONFIGURATION, status),
            I2cStep::write_read(address, [register::TEMP_RESULT], result),
        ]
    }

    fn ready(averaging: Averaging) -> u16 {
        DATA_READY | config(ConversionMode::Shutdown, averaging)
    }

    #[test]
    fn init_follows_the_datasheet_sequence_and_measure_reads_the_result() {
        let mut steps = init_steps(ADD0_GND, Averaging::X8);
        steps.extend(measure_steps(
            ADD0_GND,
            Averaging::X8,
            ready(Averaging::X8),
            AT_25_C,
        ));
        let mut sensor = Tmp117::new(I2cScript::new(steps), ADD0_GND, DelayLog::new());
        assert_eq!(sensor.revision(), None);
        sensor.init().unwrap();
        assert_eq!(sensor.revision(), Some(0));

        let reading = sensor.measure().unwrap();
        assert_eq!(reading, Reading::new(0x0C80));
        assert_eq!(reading.celsius(), 25.0);

        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(
            delay.waits_ns(),
            [125_000 * 1_000],
            "8 averages convert in 125 ms"
        );
    }

    #[test]
    fn the_averaging_reaches_the_configuration_and_sets_the_wait() {
        for averaging in [
            Averaging::None,
            Averaging::X8,
            Averaging::X32,
            Averaging::X64,
        ] {
            let mut steps = init_steps(ADD0_SCL, averaging);
            steps.extend(measure_steps(
                ADD0_SCL,
                averaging,
                ready(averaging),
                AT_25_C,
            ));
            let mut sensor = Tmp117::new(I2cScript::new(steps), ADD0_SCL, DelayLog::new())
                .with_averaging(averaging);
            assert_eq!(sensor.measure().unwrap().celsius(), 25.0);
            let (bus, delay) = sensor.release();
            assert!(bus.done());
            assert_eq!(delay.waits_ns(), [averaging.conversion_micros() * 1_000]);
        }
        // Table 7-6: MOD 11, CONV 100, AVG 11; Table 7-7: 64 averages take 1 s.
        assert_eq!(config(ConversionMode::OneShot, Averaging::X64), 0x0E60);
        assert_eq!(Averaging::X64.conversion_micros(), 1_000_000);
    }

    #[test]
    fn a_stale_data_ready_flag_is_cleared_before_the_wait_and_the_poll_repeats() {
        let one_shot = config(ConversionMode::OneShot, Averaging::X8);
        let mut steps = init_steps(ADD0_GND, Averaging::X8);
        steps.extend([
            write(ADD0_GND, register::CONFIGURATION, one_shot),
            read(ADD0_GND, register::CONFIGURATION, DATA_READY | one_shot),
            read(ADD0_GND, register::CONFIGURATION, one_shot),
            read(ADD0_GND, register::CONFIGURATION, one_shot),
            read(ADD0_GND, register::CONFIGURATION, ready(Averaging::X8)),
            I2cStep::write_read(ADD0_GND, [register::TEMP_RESULT], AT_25_C),
        ]);
        let mut sensor = Tmp117::new(I2cScript::new(steps), ADD0_GND, DelayLog::new());
        assert_eq!(sensor.measure().unwrap().celsius(), 25.0);
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(delay.waits_ns(), [125_000_000, 1_000_000, 1_000_000]);
    }

    #[test]
    fn a_part_that_is_not_a_tmp117_is_refused_and_the_revision_is_kept() {
        let steps = [I2cStep::write_read(
            ADD0_GND,
            [register::DEVICE_ID],
            [0x01, 0x16],
        )];
        let mut sensor = Tmp117::new(I2cScript::new(steps), ADD0_GND, DelayLog::new());
        assert_eq!(
            sensor.init(),
            Err(DriverError::Sensor(SensorError::Identity))
        );
        assert_eq!(sensor.revision(), None);
        assert!(sensor.release().0.done());

        // Table 7-15: Rev[3:0] sits above DID[11:0], so a later revision still passes.
        let mut steps = init_steps(ADD0_GND, Averaging::X8);
        steps[0] = I2cStep::write_read(ADD0_GND, [register::DEVICE_ID], [0x21, 0x17]);
        let mut sensor = Tmp117::new(I2cScript::new(steps), ADD0_GND, DelayLog::new());
        sensor.init().unwrap();
        assert_eq!(sensor.revision(), Some(2));
        assert!(sensor.release().0.done());
    }

    #[test]
    fn a_conversion_that_never_finishes_is_a_timeout() {
        let one_shot = config(ConversionMode::OneShot, Averaging::X8);
        let mut steps = init_steps(ADD0_GND, Averaging::X8);
        steps.push(write(ADD0_GND, register::CONFIGURATION, one_shot));
        steps.push(read(ADD0_GND, register::CONFIGURATION, one_shot));
        for _ in 0..STATUS_POLLS {
            steps.push(read(ADD0_GND, register::CONFIGURATION, one_shot));
        }
        let mut sensor = Tmp117::new(I2cScript::new(steps), ADD0_GND, DelayLog::new());
        assert_eq!(sensor.measure(), Err(DriverError::Timeout));
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(delay.waits_ns().len(), 1 + usize::from(STATUS_POLLS));
        assert_eq!(delay.waits_ns()[0], 125_000_000);
        assert_eq!(
            delay.total_micros(),
            125_000 + 1_000 * u64::from(STATUS_POLLS)
        );
    }

    #[test]
    fn an_eeprom_that_never_finishes_loading_is_a_timeout() {
        // Table 7-10: EEPROM_Busy is bit 14 of the unlock register.
        let mut steps = vec![I2cStep::write_read(ADD0_GND, [register::DEVICE_ID], ID)];
        for _ in 0..STATUS_POLLS {
            steps.push(read(ADD0_GND, register::EEPROM_UL, 1 << 14));
        }
        let mut sensor = Tmp117::new(I2cScript::new(steps), ADD0_GND, DelayLog::new());
        assert_eq!(sensor.init(), Err(DriverError::Timeout));
        assert_eq!(sensor.revision(), None);
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(delay.total_millis(), u64::from(STATUS_POLLS));
    }

    #[test]
    fn a_bus_fault_surfaces_as_a_bus_error_and_maps_to_the_core_io_error() {
        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let steps = [I2cStep::fault(ADD0_GND, kind)];
        let mut sensor = Tmp117::new(I2cScript::new(steps), ADD0_GND, DelayLog::new());
        let error = sensor.init().unwrap_err();
        assert!(matches!(error, DriverError::Bus(_)));
        let core: pamoja_core::Error = error.into();
        assert!(matches!(core, pamoja_core::Error::Io(_)));
        assert!(sensor.release().0.done());
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_one_channel() {
        let mut steps = init_steps(ADD0_GND, Averaging::X8);
        // Table 7-1: -25 °C is F380h.
        steps.extend(measure_steps(
            ADD0_GND,
            Averaging::X8,
            ready(Averaging::X8),
            [0xF3, 0x80],
        ));
        let sensor = Tmp117::new(I2cScript::new(steps), ADD0_GND, DelayLog::new());
        let mut celsius = sensor.map(|reading| reading.celsius());
        assert_eq!(block_on(celsius.read()).unwrap(), -25.0);
        assert!(celsius.into_inner().release().0.done());
    }

    #[test]
    fn alert_limits_are_written_in_register_format_and_flags_survive_the_polls() {
        let shutdown = config(ConversionMode::Shutdown, Averaging::X8);
        let mut steps = init_steps(ADD0_GND, Averaging::X8);
        // Table 7-1's format: 30 °C is 3840 counts, 0F00h; -10 °C is -1280, FB00h.
        steps.push(write(ADD0_GND, register::THIGH_LIMIT, 0x0F00));
        steps.push(write(ADD0_GND, register::TLOW_LIMIT, 0xFB00));
        // 32 °C, above the high limit, so the poll that sees the conversion finish
        // also sees HIGH_Alert and clears it.
        steps.extend(measure_steps(
            ADD0_GND,
            Averaging::X8,
            HIGH_ALERT | ready(Averaging::X8),
            [0x10, 0x00],
        ));
        steps.push(read(ADD0_GND, register::CONFIGURATION, shutdown));
        steps.push(read(ADD0_GND, register::CONFIGURATION, shutdown));
        steps.push(read(
            ADD0_GND,
            register::CONFIGURATION,
            LOW_ALERT | shutdown,
        ));
        let mut sensor = Tmp117::new(I2cScript::new(steps), ADD0_GND, DelayLog::new());
        sensor.init().unwrap();
        sensor.set_alert_limits(30.0, -10.0).unwrap();
        assert_eq!(sensor.measure().unwrap().celsius(), 32.0);
        assert_eq!(
            sensor.alerts().unwrap(),
            Alerts {
                high: true,
                low: false
            }
        );
        assert_eq!(sensor.alerts().unwrap(), Alerts::default());
        assert_eq!(
            sensor.alerts().unwrap(),
            Alerts {
                high: false,
                low: true
            }
        );
        assert!(sensor.release().0.done());
    }
}
