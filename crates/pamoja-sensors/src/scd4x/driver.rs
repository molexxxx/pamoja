//! The SCD4x driven over I2C: identify, run periodic measurements, and fetch each one.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use pamoja_core::Sensor;

use super::{
    command, command_frame, data_ready, max_duration_ms, serial_number, temperature_offset_word,
    word, write_frame, Measurement, I2C_ADDRESS,
};
use crate::error::DriverError;

/// How long the driver waits between data-ready polls, in milliseconds.
pub const DATA_READY_POLL_MILLIS: u32 = 100;

/// How many data-ready polls the driver makes before a measurement is called
/// overdue: a little more than one five-second signal update interval.
pub const DATA_READY_POLLS: u8 = 60;

/// An SCD40 or SCD41 on an I2C bus, running periodic measurements.
///
/// [`init`](Scd4x::init) stops any periodic measurement a previous run left going,
/// reads the serial number, which is the identity a part without an id register
/// offers, and starts periodic measurement, after which the part produces a new
/// result every five seconds. [`measure`](Scd4x::measure) polls the data-ready
/// status until a result waits, then reads it. Every command follows the
/// datasheet's sequence rules: the two command bytes go out most significant first,
/// the command's execution time passes before the read header is sent, and the CRC
/// on every word read is checked. As a [`Sensor`] its reading is the
/// [`Measurement`]; one channel is selected with [`Sensor::map`].
///
/// # Examples
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
/// use pamoja_sensors::scd4x::{serial_number_frame, word_frame, Measurement, Scd4x, I2C_ADDRESS};
///
/// const PART: u8 = I2C_ADDRESS;
/// let bus = I2cScript::new([
///     I2cStep::write(PART, [0x3F, 0x86]),
///     I2cStep::write(PART, [0x36, 0x82]),
///     I2cStep::read(PART, serial_number_frame(0x1234_5678_9ABC)),
///     I2cStep::write(PART, [0x21, 0xB1]),
///     I2cStep::write(PART, [0xE4, 0xB8]),
///     I2cStep::read(PART, word_frame(0x8006)),
///     I2cStep::write(PART, [0xEC, 0x05]),
///     I2cStep::read(PART, Measurement::from_physical(500, 25_000, 37_000).to_bytes()),
/// ]);
///
/// let mut sensor = Scd4x::new(bus, DelayLog::new());
/// let measurement = block_on(sensor.read())?;
/// assert_eq!(measurement.co2_ppm, 500);
/// assert_eq!(sensor.serial(), Some(0x1234_5678_9ABC));
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Scd4x<I2C, D> {
    bus: I2C,
    delay: D,
    serial: Option<u64>,
}

impl<I2C, D> Scd4x<I2C, D> {
    /// Wraps an I2C bus; the part has one address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `delay` - the timer that paces the commands and the polls.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn new(bus: I2C, delay: D) -> Self {
        Scd4x {
            bus,
            delay,
            serial: None,
        }
    }

    /// Returns the serial number read at initialization, if it has happened.
    pub fn serial(&self) -> Option<u64> {
        self.serial
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

impl<I2C: I2c, D: DelayNs> Scd4x<I2C, D> {
    /// Stops any running measurement, reads the serial number, and starts periodic
    /// measurement.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, and [`DriverError::Sensor`]
    /// with [`SensorError::Crc`] if the serial number arrives corrupted.
    pub fn init(&mut self) -> Result<(), DriverError<I2C::Error>> {
        self.send(command::STOP_PERIODIC_MEASUREMENT)?;
        self.send(command::GET_SERIAL_NUMBER)?;
        let mut frame = [0u8; 9];
        self.bus
            .read(I2C_ADDRESS, &mut frame)
            .map_err(DriverError::Bus)?;
        let serial = serial_number(&frame)?;
        self.send(command::START_PERIODIC_MEASUREMENT)?;
        self.serial = Some(serial);
        Ok(())
    }

    /// Waits for the next periodic result and reads it.
    ///
    /// Initializes the part first if [`init`](Scd4x::init) has not run. The
    /// data-ready status is polled every [`DATA_READY_POLL_MILLIS`] up to
    /// [`DATA_READY_POLLS`] times, a little over one signal update interval.
    ///
    /// # Returns
    ///
    /// The CO2, temperature, and humidity words as a [`Measurement`].
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Timeout`] if
    /// no result becomes ready, [`DriverError::Sensor`] with [`SensorError::Crc`] if
    /// a word arrives corrupted, or any error of [`init`](Scd4x::init).
    pub fn measure(&mut self) -> Result<Measurement, DriverError<I2C::Error>> {
        if self.serial.is_none() {
            self.init()?;
        }
        let mut ready = false;
        for _ in 0..DATA_READY_POLLS {
            if self.data_ready()? {
                ready = true;
                break;
            }
            self.delay.delay_ms(DATA_READY_POLL_MILLIS);
        }
        if !ready {
            return Err(DriverError::Timeout);
        }
        self.read_measurement()
    }

    /// Runs one on-demand measurement on an SCD41, which takes five seconds.
    ///
    /// The part must not be in periodic measurement: call [`stop`](Scd4x::stop)
    /// first, or use this instead of [`init`](Scd4x::init) on a bus where the part
    /// is idle.
    ///
    /// # Returns
    ///
    /// The measurement.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, and [`DriverError::Sensor`]
    /// with [`SensorError::Crc`] if a word arrives corrupted.
    pub fn measure_single_shot(&mut self) -> Result<Measurement, DriverError<I2C::Error>> {
        self.send(command::MEASURE_SINGLE_SHOT)?;
        self.read_measurement()
    }

    /// Asks whether a periodic result is waiting.
    ///
    /// # Returns
    ///
    /// `true` when [`measure`](Scd4x::measure) would read without waiting.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, and [`DriverError::Sensor`]
    /// with [`SensorError::Crc`] if the status word arrives corrupted.
    pub fn data_ready(&mut self) -> Result<bool, DriverError<I2C::Error>> {
        self.send(command::GET_DATA_READY_STATUS)?;
        let mut frame = [0u8; 3];
        self.bus
            .read(I2C_ADDRESS, &mut frame)
            .map_err(DriverError::Bus)?;
        Ok(data_ready(word(&frame)?))
    }

    /// Stops periodic measurement, after which the settings commands are allowed.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn stop(&mut self) -> Result<(), DriverError<I2C::Error>> {
        self.send(command::STOP_PERIODIC_MEASUREMENT)
    }

    /// Starts periodic measurement.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn start(&mut self) -> Result<(), DriverError<I2C::Error>> {
        self.send(command::START_PERIODIC_MEASUREMENT)
    }

    /// Sets the temperature offset that compensates the part's self-heating.
    ///
    /// Periodic measurement is stopped, the offset written, and measurement started
    /// again, since the part only takes settings while idle. The setting lasts until
    /// power is lost unless persisted by the part's own command.
    ///
    /// # Arguments
    ///
    /// * `milli_celsius` - the offset to subtract, in millidegrees Celsius.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn set_temperature_offset(
        &mut self,
        milli_celsius: u32,
    ) -> Result<(), DriverError<I2C::Error>> {
        self.stop()?;
        self.write(
            command::SET_TEMPERATURE_OFFSET,
            temperature_offset_word(milli_celsius),
        )?;
        self.start()
    }

    /// Sets the altitude the part corrects its CO2 reading for.
    ///
    /// Periodic measurement is stopped, the altitude written, and measurement
    /// started again.
    ///
    /// # Arguments
    ///
    /// * `meters` - the altitude above sea level.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails.
    pub fn set_sensor_altitude(&mut self, meters: u16) -> Result<(), DriverError<I2C::Error>> {
        self.stop()?;
        self.write(command::SET_SENSOR_ALTITUDE, meters)?;
        self.start()
    }

    fn read_measurement(&mut self) -> Result<Measurement, DriverError<I2C::Error>> {
        self.send(command::READ_MEASUREMENT)?;
        let mut frame = [0u8; 9];
        self.bus
            .read(I2C_ADDRESS, &mut frame)
            .map_err(DriverError::Bus)?;
        Ok(Measurement::parse(&frame)?)
    }

    fn send(&mut self, command: u16) -> Result<(), DriverError<I2C::Error>> {
        self.bus
            .write(I2C_ADDRESS, &command_frame(command))
            .map_err(DriverError::Bus)?;
        self.wait(command);
        Ok(())
    }

    fn write(&mut self, command: u16, value: u16) -> Result<(), DriverError<I2C::Error>> {
        self.bus
            .write(I2C_ADDRESS, &write_frame(command, value))
            .map_err(DriverError::Bus)?;
        self.wait(command);
        Ok(())
    }

    fn wait(&mut self, command: u16) {
        if let Some(millis) = max_duration_ms(command) {
            self.delay.delay_ms(u32::from(millis));
        }
    }
}

impl<I2C, D> Sensor for Scd4x<I2C, D>
where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
    D: DelayNs,
{
    type Reading = Measurement;

    async fn read(&mut self) -> pamoja_core::Result<Measurement> {
        self.measure().map_err(pamoja_core::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scd4x::{serial_number_frame, word_frame, PERIODIC_MEASUREMENT_INTERVAL_MS};
    use crate::SensorError;
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};

    const SERIAL: u64 = 0x1234_5678_9ABC;

    fn send(command: u16) -> I2cStep {
        I2cStep::write(I2C_ADDRESS, command_frame(command))
    }

    fn init_steps() -> Vec<I2cStep> {
        vec![
            send(command::STOP_PERIODIC_MEASUREMENT),
            send(command::GET_SERIAL_NUMBER),
            I2cStep::read(I2C_ADDRESS, serial_number_frame(SERIAL)),
            send(command::START_PERIODIC_MEASUREMENT),
        ]
    }

    fn sample() -> Measurement {
        Measurement::from_physical(500, 25_000, 37_000)
    }

    #[test]
    fn init_stops_identifies_and_starts_then_measure_polls_and_reads() {
        let mut steps = init_steps();
        steps.extend([
            send(command::GET_DATA_READY_STATUS),
            I2cStep::read(I2C_ADDRESS, word_frame(0x8000)),
            send(command::GET_DATA_READY_STATUS),
            I2cStep::read(I2C_ADDRESS, word_frame(0x8006)),
            send(command::READ_MEASUREMENT),
            I2cStep::read(I2C_ADDRESS, sample().to_bytes()),
        ]);
        let mut sensor = Scd4x::new(I2cScript::new(steps), DelayLog::new());
        sensor.init().unwrap();
        assert_eq!(sensor.serial(), Some(SERIAL));
        let measurement = sensor.measure().unwrap();
        assert_eq!(measurement, sample());
        assert_eq!(measurement.co2_ppm, 500);
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(
            delay.waits_ns(),
            [500_000_000, 1_000_000, 1_000_000, 100_000_000, 1_000_000, 1_000_000],
            "stop waits 500 ms, each read waits its 1 ms execution time, a not-ready poll waits 100 ms"
        );
    }

    #[test]
    fn a_result_that_never_comes_is_a_timeout() {
        let mut steps = init_steps();
        for _ in 0..DATA_READY_POLLS {
            steps.push(send(command::GET_DATA_READY_STATUS));
            steps.push(I2cStep::read(I2C_ADDRESS, word_frame(0x8000)));
        }
        let mut sensor = Scd4x::new(I2cScript::new(steps), DelayLog::new());
        assert_eq!(sensor.measure(), Err(DriverError::Timeout));
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        let polls = u64::from(DATA_READY_POLLS);
        assert_eq!(delay.total_millis(), 500 + 1 + polls * (1 + 100));
        assert!(delay.total_millis() > u64::from(PERIODIC_MEASUREMENT_INTERVAL_MS));
    }

    #[test]
    fn a_corrupted_word_is_a_crc_failure_and_a_bus_fault_a_bus_error() {
        let mut corrupted = serial_number_frame(SERIAL);
        corrupted[2] ^= 0x01;
        let steps = [
            send(command::STOP_PERIODIC_MEASUREMENT),
            send(command::GET_SERIAL_NUMBER),
            I2cStep::read(I2C_ADDRESS, corrupted),
        ];
        let mut sensor = Scd4x::new(I2cScript::new(steps), DelayLog::new());
        assert_eq!(sensor.init(), Err(DriverError::Sensor(SensorError::Crc)));
        assert_eq!(sensor.serial(), None);

        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let mut sensor = Scd4x::new(
            I2cScript::new([I2cStep::fault(I2C_ADDRESS, kind)]),
            DelayLog::new(),
        );
        assert!(matches!(sensor.init(), Err(DriverError::Bus(_))));
    }

    #[test]
    fn a_single_shot_waits_five_seconds_then_reads() {
        let steps = [
            send(command::MEASURE_SINGLE_SHOT),
            send(command::READ_MEASUREMENT),
            I2cStep::read(I2C_ADDRESS, sample().to_bytes()),
        ];
        let mut sensor = Scd4x::new(I2cScript::new(steps), DelayLog::new());
        assert_eq!(sensor.measure_single_shot().unwrap(), sample());
        let (bus, delay) = sensor.release();
        assert!(bus.done());
        assert_eq!(delay.total_millis(), 5_001);
    }

    #[test]
    fn settings_stop_write_and_restart_the_periodic_measurement() {
        let mut steps = init_steps();
        steps.extend([
            send(command::STOP_PERIODIC_MEASUREMENT),
            I2cStep::write(
                I2C_ADDRESS,
                write_frame(
                    command::SET_TEMPERATURE_OFFSET,
                    temperature_offset_word(4_000),
                ),
            ),
            send(command::START_PERIODIC_MEASUREMENT),
            send(command::STOP_PERIODIC_MEASUREMENT),
            I2cStep::write(
                I2C_ADDRESS,
                write_frame(command::SET_SENSOR_ALTITUDE, 1_500),
            ),
            send(command::START_PERIODIC_MEASUREMENT),
        ]);
        let mut sensor = Scd4x::new(I2cScript::new(steps), DelayLog::new());
        sensor.init().unwrap();
        sensor.set_temperature_offset(4_000).unwrap();
        sensor.set_sensor_altitude(1_500).unwrap();
        assert!(sensor.release().0.done());
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_co2() {
        let mut steps = init_steps();
        steps.extend([
            send(command::GET_DATA_READY_STATUS),
            I2cStep::read(I2C_ADDRESS, word_frame(0x8006)),
            send(command::READ_MEASUREMENT),
            I2cStep::read(I2C_ADDRESS, sample().to_bytes()),
        ]);
        let sensor = Scd4x::new(I2cScript::new(steps), DelayLog::new());
        let mut co2 = sensor.map(|measurement| measurement.co2_ppm);
        assert_eq!(block_on(co2.read()).unwrap(), 500);
    }
}
