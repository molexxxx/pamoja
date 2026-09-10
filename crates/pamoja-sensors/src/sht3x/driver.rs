//! The SHT3x driven over I2C: reset, confirm the part answers, and measure on demand.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use pamoja_core::Sensor;

use super::{
    command, single_shot, Measurement, Repeatability, Status, MIN_COMMAND_GAP_MICROS,
    SOFT_RESET_MICROS,
};
use crate::error::DriverError;

/// An SHT3x on an I2C bus, measuring on demand in single-shot mode.
///
/// [`Sht3x::new`] wraps the bus. [`init`](Sht3x::init) soft-resets the part, waits the
/// datasheet's reset time, and reads the status register: the part has no id
/// register, so a status word whose CRC checks is what confirms an SHT3x answers at
/// the address. [`measure`](Sht3x::measure) issues one single-shot command without
/// clock stretching, waits the datasheet's maximum measurement duration for the
/// repeatability in use, reads the six data bytes, and returns the CRC-checked
/// [`Measurement`]. Every command is followed by the wait the datasheet attaches to
/// it (the reset time, the measurement duration, or the 1 ms between commands), so
/// consecutive calls keep the spacing the part needs. As a [`Sensor`] its reading is
/// the whole measurement; one channel is selected with [`Sensor::map`].
///
/// # Examples
///
/// The part's side of the conversation, scripted: what the datasheet says an SHT3x
/// answers during initialization and one single-shot measurement, here at 25 °C and
/// 60 %RH.
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
/// use pamoja_sensors::sht3x::{
///     humidity_raw_from_relative_humidity, temperature_raw_from_celsius, Measurement,
///     Sht3x, Status, I2C_ADDRESS_A,
/// };
///
/// const PART: u8 = I2C_ADDRESS_A;
/// let status = Status::from_bits(Status::DEFAULT).to_bytes();
/// let data = Measurement {
///     temperature_raw: temperature_raw_from_celsius(25.0),
///     humidity_raw: humidity_raw_from_relative_humidity(60.0),
/// }
/// .to_bytes();
/// let bus = I2cScript::new([
///     I2cStep::write(PART, [0x30, 0xA2]),
///     I2cStep::write(PART, [0xF3, 0x2D]),
///     I2cStep::read(PART, status),
///     I2cStep::write(PART, [0x24, 0x00]),
///     I2cStep::read(PART, data),
/// ]);
///
/// let mut sensor = Sht3x::new(bus, PART, DelayLog::new());
/// let measurement = block_on(sensor.read())?;
/// assert_eq!(measurement.temperature_milli_celsius(), 25_000);
/// assert_eq!(measurement.humidity_milli_percent(), 60_000);
///
/// let (bus, delay) = sensor.release();
/// assert!(bus.done());
/// assert_eq!(delay.total_micros(), 1_500 + 1_000 + 15_000);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Sht3x<I2C, D> {
    bus: I2C,
    address: u8,
    delay: D,
    repeatability: Repeatability,
    status: Option<Status>,
}

impl<I2C, D> Sht3x<I2C, D> {
    /// Wraps an I2C bus and the part's address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `address` - [`I2C_ADDRESS_A`](super::I2C_ADDRESS_A) with ADDR low,
    ///   [`I2C_ADDRESS_B`](super::I2C_ADDRESS_B) with ADDR high.
    /// * `delay` - the timer that paces the reset, the measurements, and the gaps
    ///   between commands.
    ///
    /// # Returns
    ///
    /// The driver, measuring at high repeatability.
    pub fn new(bus: I2C, address: u8, delay: D) -> Self {
        Sht3x {
            bus,
            address,
            delay,
            repeatability: Repeatability::High,
            status: None,
        }
    }

    /// Sets the repeatability of each measurement, which trades noise against
    /// measurement time and energy.
    ///
    /// # Arguments
    ///
    /// * `repeatability` - the repeatability to measure at.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_repeatability(mut self, repeatability: Repeatability) -> Self {
        self.repeatability = repeatability;
        self
    }

    /// Returns the part's 7-bit address.
    pub fn address(&self) -> u8 {
        self.address
    }

    /// Returns the status register as it was last read, if it has been.
    pub fn status(&self) -> Option<Status> {
        self.status
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

impl<I2C: I2c, D: DelayNs> Sht3x<I2C, D> {
    /// Resets the part and reads its status register.
    ///
    /// The sequence is the datasheet's: the soft-reset command of Table 14, the
    /// soft-reset time of Table 4, then the status read of Table 17. The part has no
    /// id register, so a status word whose CRC checks is the confirmation that an
    /// SHT3x answers at the address. The reset also switches the heater off.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails and [`DriverError::Sensor`] with
    /// [`SensorError::Crc`](crate::SensorError::Crc) if the status word does not
    /// check, so whatever answered is not a working SHT3x.
    pub fn init(&mut self) -> Result<(), DriverError<I2C::Error>> {
        self.send(command::SOFT_RESET)?;
        self.delay.delay_us(SOFT_RESET_MICROS);
        self.read_status()?;
        Ok(())
    }

    /// Runs one single-shot measurement and returns the CRC-checked result.
    ///
    /// Initializes the part first if [`init`](Sht3x::init) has not run. The sequence
    /// is sections 4.2 to 4.4 of the datasheet: the Table 9 command for the
    /// repeatability in use with clock stretching disabled, a wait of Table 4's
    /// maximum measurement duration, then one read of the temperature word, its CRC,
    /// the humidity word, and its CRC. Table 4 holds from 2.4 V up; below that
    /// (Table 5) the part can take 0.5 ms longer and answers a read that comes early
    /// with a NACK, which the bus reports as its error.
    ///
    /// # Returns
    ///
    /// The measurement.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Sensor`] with
    /// [`SensorError::Crc`](crate::SensorError::Crc) if either data word does not
    /// check, or any error of [`init`](Sht3x::init).
    pub fn measure(&mut self) -> Result<Measurement, DriverError<I2C::Error>> {
        self.initialize_once()?;
        self.send(single_shot(self.repeatability, false))?;
        let wait = self.repeatability.max_measurement_micros();
        self.delay.delay_us(wait);
        let mut bytes = [0u8; 6];
        self.bus
            .read(self.address, &mut bytes)
            .map_err(DriverError::Bus)?;
        Measurement::parse(&bytes).map_err(DriverError::Sensor)
    }

    /// Reads the status register.
    ///
    /// The command of Table 17, the three bytes that answer it (the register most
    /// significant byte first, then its CRC), and the 1 ms the part needs before its
    /// next command.
    ///
    /// # Returns
    ///
    /// The decoded status, which [`status`](Sht3x::status) keeps as well.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails and [`DriverError::Sensor`] with
    /// [`SensorError::Crc`](crate::SensorError::Crc) if the word does not check.
    pub fn read_status(&mut self) -> Result<Status, DriverError<I2C::Error>> {
        self.send(command::READ_STATUS)?;
        let mut bytes = [0u8; 3];
        self.bus
            .read(self.address, &mut bytes)
            .map_err(DriverError::Bus)?;
        self.delay.delay_us(MIN_COMMAND_GAP_MICROS);
        let status = Status::parse(&bytes).map_err(DriverError::Sensor)?;
        self.status = Some(status);
        Ok(status)
    }

    /// Switches the plausibility-check heater on.
    ///
    /// Initializes the part first if [`init`](Sht3x::init) has not run, since the
    /// reset in it would switch the heater off again. Sends the enable command of
    /// Table 16 and waits the 1 ms the part needs before its next command. The
    /// heater's state shows in [`Status::heater_on`] on the next
    /// [`read_status`](Sht3x::read_status).
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, or any error of
    /// [`init`](Sht3x::init).
    pub fn heater_on(&mut self) -> Result<(), DriverError<I2C::Error>> {
        self.initialize_once()?;
        self.send(command::HEATER_ENABLE)?;
        self.delay.delay_us(MIN_COMMAND_GAP_MICROS);
        Ok(())
    }

    /// Switches the heater off, which is its state after any reset.
    ///
    /// Initializes the part first if [`init`](Sht3x::init) has not run, then sends
    /// the disable command of Table 16 and waits the 1 ms the part needs before its
    /// next command.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, or any error of
    /// [`init`](Sht3x::init).
    pub fn heater_off(&mut self) -> Result<(), DriverError<I2C::Error>> {
        self.initialize_once()?;
        self.send(command::HEATER_DISABLE)?;
        self.delay.delay_us(MIN_COMMAND_GAP_MICROS);
        Ok(())
    }

    fn initialize_once(&mut self) -> Result<(), DriverError<I2C::Error>> {
        if self.status.is_none() {
            self.init()?;
        }
        Ok(())
    }

    fn send(&mut self, word: u16) -> Result<(), DriverError<I2C::Error>> {
        self.bus
            .write(self.address, &word.to_be_bytes())
            .map_err(DriverError::Bus)
    }
}

impl<I2C, D> Sensor for Sht3x<I2C, D>
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
    use crate::sht3x::{
        humidity_raw_from_relative_humidity, temperature_raw_from_celsius, I2C_ADDRESS_A,
        I2C_ADDRESS_B,
    };
    use crate::SensorError;
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};

    const SOFT_RESET: [u8; 2] = [0x30, 0xA2];
    const READ_STATUS: [u8; 2] = [0xF3, 0x2D];
    const SINGLE_SHOT_HIGH: [u8; 2] = [0x24, 0x00];
    const HEATER_ENABLE: [u8; 2] = [0x30, 0x6D];
    const HEATER_DISABLE: [u8; 2] = [0x30, 0x66];

    fn part_at(celsius: f32, percent: f32) -> Measurement {
        Measurement {
            temperature_raw: temperature_raw_from_celsius(celsius),
            humidity_raw: humidity_raw_from_relative_humidity(percent),
        }
    }

    fn init_steps(address: u8) -> Vec<I2cStep> {
        vec![
            I2cStep::write(address, SOFT_RESET),
            I2cStep::write(address, READ_STATUS),
            I2cStep::read(address, Status::from_bits(Status::DEFAULT).to_bytes()),
        ]
    }

    fn measure_steps(address: u8, command: [u8; 2], reply: [u8; 6]) -> Vec<I2cStep> {
        vec![
            I2cStep::write(address, command),
            I2cStep::read(address, reply),
        ]
    }

    #[test]
    fn init_resets_and_confirms_the_part_and_measure_follows_the_single_shot_sequence() {
        let part = part_at(25.0, 60.0);
        let mut steps = init_steps(I2C_ADDRESS_A);
        steps.extend(measure_steps(
            I2C_ADDRESS_A,
            SINGLE_SHOT_HIGH,
            part.to_bytes(),
        ));
        let mut sensor = Sht3x::new(I2cScript::new(steps), I2C_ADDRESS_A, DelayLog::new());
        assert_eq!(sensor.address(), I2C_ADDRESS_A);
        assert_eq!(sensor.status(), None);
        sensor.init().unwrap();
        assert_eq!(sensor.status(), Some(Status::from_bits(Status::DEFAULT)));

        let measurement = sensor.measure().unwrap();
        assert_eq!(measurement, part);
        assert_eq!(measurement.temperature_milli_celsius(), 25_000);
        assert_eq!(measurement.humidity_milli_percent(), 60_000);

        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(
            delay.waits_ns(),
            [1_500_000, 1_000_000, 15_000_000],
            "the 1.5 ms soft-reset time, the 1 ms command gap, then 15 ms at high repeatability"
        );
    }

    #[test]
    fn the_repeatability_picks_the_table_9_command_and_the_table_4_wait() {
        let part = part_at(-10.5, 12.5);
        let table = [
            (Repeatability::Low, [0x24, 0x16], 4_000_000),
            (Repeatability::Medium, [0x24, 0x0B], 6_000_000),
            (Repeatability::High, [0x24, 0x00], 15_000_000),
        ];
        for (repeatability, command, wait_ns) in table {
            let mut steps = init_steps(I2C_ADDRESS_B);
            steps.extend(measure_steps(I2C_ADDRESS_B, command, part.to_bytes()));
            let mut sensor = Sht3x::new(I2cScript::new(steps), I2C_ADDRESS_B, DelayLog::new())
                .with_repeatability(repeatability);
            assert_eq!(sensor.measure().unwrap(), part, "{repeatability:?}");
            let (bus, delay) = sensor.release();
            assert!(bus.done(), "{repeatability:?}");
            assert_eq!(delay.waits_ns()[2], wait_ns, "{repeatability:?}");
        }
    }

    #[test]
    fn a_corrupted_word_is_a_crc_error() {
        let mut data = part_at(25.0, 60.0).to_bytes();
        data[4] ^= 0x80;
        let mut steps = init_steps(I2C_ADDRESS_A);
        steps.extend(measure_steps(I2C_ADDRESS_A, SINGLE_SHOT_HIGH, data));
        let mut sensor = Sht3x::new(I2cScript::new(steps), I2C_ADDRESS_A, DelayLog::new());
        assert_eq!(sensor.measure(), Err(DriverError::Sensor(SensorError::Crc)));
        assert!(sensor.release().0.done());

        let mut status = Status::from_bits(Status::DEFAULT).to_bytes();
        status[2] ^= 0x01;
        let steps = [
            I2cStep::write(I2C_ADDRESS_A, SOFT_RESET),
            I2cStep::write(I2C_ADDRESS_A, READ_STATUS),
            I2cStep::read(I2C_ADDRESS_A, status),
        ];
        let mut sensor = Sht3x::new(I2cScript::new(steps), I2C_ADDRESS_A, DelayLog::new());
        assert_eq!(sensor.init(), Err(DriverError::Sensor(SensorError::Crc)));
        assert_eq!(
            sensor.status(),
            None,
            "a word that fails its CRC is not kept"
        );
        assert!(sensor.release().0.done());
    }

    #[test]
    fn a_bus_fault_surfaces_as_a_bus_error_and_maps_to_the_core_io_error() {
        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let steps = [I2cStep::fault(I2C_ADDRESS_A, kind)];
        let mut sensor = Sht3x::new(I2cScript::new(steps), I2C_ADDRESS_A, DelayLog::new());
        let error = sensor.init().unwrap_err();
        assert!(matches!(error, DriverError::Bus(_)));
        let core: pamoja_core::Error = error.into();
        assert!(matches!(core, pamoja_core::Error::Io(_)));

        let mut steps = init_steps(I2C_ADDRESS_A);
        steps.push(I2cStep::write(I2C_ADDRESS_A, SINGLE_SHOT_HIGH));
        steps.push(I2cStep::fault(I2C_ADDRESS_A, kind));
        let mut sensor = Sht3x::new(I2cScript::new(steps), I2C_ADDRESS_A, DelayLog::new());
        assert!(
            matches!(sensor.measure(), Err(DriverError::Bus(_))),
            "the NACK the part answers a read header with before data is ready is a bus error"
        );
        assert!(sensor.release().0.done());
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_one_channel() {
        let part = part_at(23.73, 63.37);
        let mut steps = init_steps(I2C_ADDRESS_A);
        steps.extend(measure_steps(
            I2C_ADDRESS_A,
            SINGLE_SHOT_HIGH,
            part.to_bytes(),
        ));
        let sensor = Sht3x::new(I2cScript::new(steps), I2C_ADDRESS_A, DelayLog::new());
        let mut celsius = sensor.map(|measurement| measurement.temperature_celsius());
        let reading = block_on(celsius.read()).unwrap();
        assert!((reading - 23.73).abs() < 0.002);
        assert!(celsius.into_inner().release().0.done());
    }

    #[test]
    fn the_heater_commands_are_table_16_and_the_status_shows_the_heater() {
        let heating = Status::from_bits(Status::HEATER_ON);
        let mut steps = init_steps(I2C_ADDRESS_A);
        steps.extend([
            I2cStep::write(I2C_ADDRESS_A, HEATER_ENABLE),
            I2cStep::write(I2C_ADDRESS_A, READ_STATUS),
            I2cStep::read(I2C_ADDRESS_A, heating.to_bytes()),
            I2cStep::write(I2C_ADDRESS_A, HEATER_DISABLE),
        ]);
        let mut sensor = Sht3x::new(I2cScript::new(steps), I2C_ADDRESS_A, DelayLog::new());
        sensor.heater_on().unwrap();
        assert!(
            !sensor.status().unwrap().heater_on(),
            "the reset leaves the heater off"
        );
        assert_eq!(sensor.read_status().unwrap(), heating);
        assert!(sensor.status().unwrap().heater_on());
        sensor.heater_off().unwrap();
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(
            delay.waits_ns(),
            [1_500_000, 1_000_000, 1_000_000, 1_000_000, 1_000_000],
            "the reset time, then the 1 ms gap after each of the four commands"
        );
    }
}
