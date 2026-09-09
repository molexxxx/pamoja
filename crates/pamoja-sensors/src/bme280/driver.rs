//! The BME280 driven over a bus: reset, identify, calibrate, and measure on demand.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use embedded_hal::spi::SpiDevice;
use pamoja_core::Sensor;

use super::{
    image_updating, max_measurement_micros, measuring, register, Calibration, Config, CtrlHum,
    CtrlMeas, Filter, Measurement, Mode, Oversampling, RawMeasurement, CALIB_HUMIDITY_LEN,
    CALIB_TEMP_PRESS_LEN, CHIP_ID, DATA_LEN, RESET_WORD, STARTUP_MICROS,
};
use crate::driver::{I2cRegisters, RegisterBus, SpiRegisters};
use crate::error::{DriverError, SensorError};

/// How many times the status register is polled, 1 ms apart, before a conversion or
/// a reset is called overdue.
pub const STATUS_POLLS: u8 = 20;

/// A BME280 on an I2C or SPI bus, measuring on demand in forced mode.
///
/// [`Bme280::i2c`] or [`Bme280::spi`] wraps the bus. [`init`](Bme280::init) resets the
/// part, checks its chip id, reads its calibration, and writes the oversampling and
/// filter settings, in the order the datasheet requires (`ctrl_hum` takes effect only
/// after the following `ctrl_meas` write). [`measure`](Bme280::measure) runs one
/// forced-mode conversion, waits the datasheet's maximum measurement time for the
/// settings in use, confirms the part is idle, and returns the compensated
/// [`Measurement`]. As a [`Sensor`] its reading is the whole measurement; one channel
/// is selected with [`Sensor::map`].
///
/// # Examples
///
/// The part's side of the conversation, scripted: what the datasheet says a BME280
/// answers during initialization and one forced measurement.
///
/// ```
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
/// use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};
/// use pamoja_core::Sensor;
///
/// const PART: u8 = I2C_ADDRESS_PRIMARY;
/// let calibration_a = [
///     0x45, 0x6F, 0x6F, 0x68, 0x32, 0x00, 0x46, 0x91, 0x6A, 0xD6, 0xD0, 0x0B, 0x4E,
///     0x1E, 0x88, 0xFF, 0xF9, 0xFF, 0xAC, 0x26, 0x0A, 0xD8, 0xBD, 0x10, 0x00, 0x4B,
/// ];
/// let calibration_b = [0x62, 0x01, 0x00, 0x15, 0x23, 0x03, 0x1E];
/// let bus = I2cScript::new([
///     I2cStep::write(PART, [0xE0, 0xB6]),
///     I2cStep::write_read(PART, [0xF3], [0x00]),
///     I2cStep::write_read(PART, [0xD0], [0x60]),
///     I2cStep::write_read(PART, [0x88], calibration_a),
///     I2cStep::write_read(PART, [0xE1], calibration_b),
///     I2cStep::write(PART, [0xF5, 0x00]),
///     I2cStep::write(PART, [0xF2, 0x01]),
///     I2cStep::write(PART, [0xF4, 0x24]),
///     I2cStep::write(PART, [0xF4, 0x25]),
///     I2cStep::write_read(PART, [0xF3], [0x00]),
///     I2cStep::write_read(PART, [0xF7], [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x75, 0x30]),
/// ]);
///
/// let mut sensor = Bme280::i2c(bus, PART, DelayLog::new());
/// let measurement = block_on(sensor.read())?;
/// assert_eq!(measurement.temperature_centi_celsius, 2044);
///
/// let (registers, delay) = sensor.release();
/// assert!(registers.release().done());
/// assert!(delay.total_micros() >= 2_000 + 9_300);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Bme280<B, D> {
    bus: B,
    delay: D,
    temperature: Oversampling,
    pressure: Oversampling,
    humidity: Oversampling,
    filter: Filter,
    calibration: Option<Calibration>,
}

impl<I2C: I2c, D> Bme280<I2cRegisters<I2C>, D> {
    /// Wraps an I2C bus and the part's address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `address` - [`I2C_ADDRESS_PRIMARY`](super::I2C_ADDRESS_PRIMARY) with SDO
    ///   low, [`I2C_ADDRESS_SECONDARY`](super::I2C_ADDRESS_SECONDARY) with SDO high.
    /// * `delay` - the timer that paces the reset and the conversions.
    ///
    /// # Returns
    ///
    /// The driver, with every measurement at oversampling x1 and the filter off.
    pub fn i2c(bus: I2C, address: u8, delay: D) -> Self {
        Bme280::new(I2cRegisters::new(bus, address), delay)
    }
}

impl<SPI: SpiDevice, D> Bme280<SpiRegisters<SPI>, D> {
    /// Wraps an SPI device, the bus plus the part's chip-select line.
    ///
    /// # Arguments
    ///
    /// * `device` - the SPI device, in mode 0 or 3.
    /// * `delay` - the timer that paces the reset and the conversions.
    ///
    /// # Returns
    ///
    /// The driver, with every measurement at oversampling x1 and the filter off.
    pub fn spi(device: SPI, delay: D) -> Self {
        Bme280::new(SpiRegisters::new(device), delay)
    }
}

impl<B, D> Bme280<B, D> {
    /// Wraps any register bus.
    ///
    /// # Arguments
    ///
    /// * `bus` - the part's register map, over whichever bus reaches it.
    /// * `delay` - the timer that paces the reset and the conversions.
    ///
    /// # Returns
    ///
    /// The driver, with every measurement at oversampling x1 and the filter off.
    pub fn new(bus: B, delay: D) -> Self {
        Bme280 {
            bus,
            delay,
            temperature: Oversampling::X1,
            pressure: Oversampling::X1,
            humidity: Oversampling::X1,
            filter: Filter::Off,
            calibration: None,
        }
    }

    /// Sets the oversampling of each measurement; `Skipped` leaves one out.
    ///
    /// The settings are written to the part by [`init`](Bme280::init).
    ///
    /// # Arguments
    ///
    /// * `temperature` - temperature oversampling.
    /// * `pressure` - pressure oversampling.
    /// * `humidity` - humidity oversampling.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_oversampling(
        mut self,
        temperature: Oversampling,
        pressure: Oversampling,
        humidity: Oversampling,
    ) -> Self {
        self.temperature = temperature;
        self.pressure = pressure;
        self.humidity = humidity;
        self
    }

    /// Sets the IIR filter that smooths pressure and temperature.
    ///
    /// The setting is written to the part by [`init`](Bme280::init).
    ///
    /// # Arguments
    ///
    /// * `filter` - the filter coefficient.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }

    /// Returns the calibration read at initialization, if it has happened.
    pub fn calibration(&self) -> Option<&Calibration> {
        self.calibration.as_ref()
    }

    /// Gives back the bus and the delay.
    ///
    /// # Returns
    ///
    /// The bus and delay the driver was built from.
    pub fn release(self) -> (B, D) {
        (self.bus, self.delay)
    }
}

impl<B: RegisterBus, D: DelayNs> Bme280<B, D> {
    /// Resets the part, checks its identity, reads its calibration, and writes the
    /// settings.
    ///
    /// The sequence is the datasheet's: the soft-reset word, the start-up time, a wait
    /// for the calibration image to finish loading, the chip id, the two calibration
    /// blocks, then `config`, `ctrl_hum`, and `ctrl_meas` in that order, leaving the
    /// part asleep.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Timeout`] if the
    /// calibration image never finishes loading, and
    /// [`DriverError::Sensor`] with [`SensorError::Identity`] if the chip id is not a
    /// BME280's.
    pub fn init(&mut self) -> Result<(), DriverError<B::Error>> {
        self.write(register::RESET, RESET_WORD)?;
        self.delay.delay_us(STARTUP_MICROS);
        self.wait(image_updating)?;

        if self.read(register::CHIP_ID)? != CHIP_ID {
            return Err(SensorError::Identity.into());
        }

        let mut temp_press = [0u8; CALIB_TEMP_PRESS_LEN];
        let mut humidity = [0u8; CALIB_HUMIDITY_LEN];
        self.bus
            .read_registers(register::CALIB_TEMP_PRESS, &mut temp_press)
            .map_err(DriverError::Bus)?;
        self.bus
            .read_registers(register::CALIB_HUMIDITY, &mut humidity)
            .map_err(DriverError::Bus)?;
        self.calibration = Some(Calibration::from_registers(&temp_press, &humidity));

        let config = Config {
            filter: self.filter,
            ..Config::default()
        };
        self.write(register::CONFIG, config.bits())?;
        let ctrl_hum = CtrlHum {
            humidity: self.humidity,
        };
        self.write(register::CTRL_HUM, ctrl_hum.bits())?;
        self.write(register::CTRL_MEAS, self.ctrl_meas(Mode::Sleep).bits())?;
        Ok(())
    }

    /// Runs one forced-mode conversion and returns the compensated measurement.
    ///
    /// Initializes the part first if [`init`](Bme280::init) has not run. The wait is
    /// the datasheet's maximum measurement time for the oversampling in use, after
    /// which the status register is polled until the part reports idle.
    ///
    /// # Returns
    ///
    /// The compensated measurement.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Timeout`] if the
    /// part is still measuring after the polls, or any error of
    /// [`init`](Bme280::init).
    pub fn measure(&mut self) -> Result<Measurement, DriverError<B::Error>> {
        if self.calibration.is_none() {
            self.init()?;
        }
        self.write(register::CTRL_MEAS, self.ctrl_meas(Mode::Forced).bits())?;
        self.delay.delay_us(max_measurement_micros(
            self.temperature,
            self.pressure,
            self.humidity,
        ));
        self.wait(measuring)?;

        let mut data = [0u8; DATA_LEN];
        self.bus
            .read_registers(register::DATA, &mut data)
            .map_err(DriverError::Bus)?;
        let raw = RawMeasurement::from_registers(&data);
        let calibration = self.calibration.as_ref().expect("read at initialization");
        Ok(calibration.compensate(&raw))
    }

    fn ctrl_meas(&self, mode: Mode) -> CtrlMeas {
        CtrlMeas {
            temperature: self.temperature,
            pressure: self.pressure,
            mode,
        }
    }

    fn read(&mut self, register: u8) -> Result<u8, DriverError<B::Error>> {
        self.bus.read_register(register).map_err(DriverError::Bus)
    }

    fn write(&mut self, register: u8, value: u8) -> Result<(), DriverError<B::Error>> {
        self.bus
            .write_register(register, value)
            .map_err(DriverError::Bus)
    }

    fn wait(&mut self, busy: fn(u8) -> bool) -> Result<(), DriverError<B::Error>> {
        for _ in 0..STATUS_POLLS {
            if !busy(self.read(register::STATUS)?) {
                return Ok(());
            }
            self.delay.delay_ms(1);
        }
        Err(DriverError::Timeout)
    }
}

impl<B, D> Sensor for Bme280<B, D>
where
    B: RegisterBus,
    B::Error: core::fmt::Debug,
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
    use crate::bme280::{Standby, I2C_ADDRESS_PRIMARY, I2C_ADDRESS_SECONDARY};
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep, SpiScript, SpiStep};

    const CALIBRATION_A: [u8; CALIB_TEMP_PRESS_LEN] = [
        0x45, 0x6F, 0x6F, 0x68, 0x32, 0x00, 0x46, 0x91, 0x6A, 0xD6, 0xD0, 0x0B, 0x4E, 0x1E, 0x88,
        0xFF, 0xF9, 0xFF, 0xAC, 0x26, 0x0A, 0xD8, 0xBD, 0x10, 0x00, 0x4B,
    ];
    const CALIBRATION_B: [u8; CALIB_HUMIDITY_LEN] = [0x62, 0x01, 0x00, 0x15, 0x23, 0x03, 0x1E];
    const DATA: [u8; DATA_LEN] = [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x75, 0x30];

    fn init_steps(address: u8, config: u8, ctrl_hum: u8, ctrl_meas: u8) -> Vec<I2cStep> {
        vec![
            I2cStep::write(address, [register::RESET, RESET_WORD]),
            I2cStep::write_read(address, [register::STATUS], [0x00]),
            I2cStep::write_read(address, [register::CHIP_ID], [CHIP_ID]),
            I2cStep::write_read(address, [register::CALIB_TEMP_PRESS], CALIBRATION_A),
            I2cStep::write_read(address, [register::CALIB_HUMIDITY], CALIBRATION_B),
            I2cStep::write(address, [register::CONFIG, config]),
            I2cStep::write(address, [register::CTRL_HUM, ctrl_hum]),
            I2cStep::write(address, [register::CTRL_MEAS, ctrl_meas]),
        ]
    }

    fn measure_steps(address: u8, ctrl_meas: u8) -> Vec<I2cStep> {
        vec![
            I2cStep::write(address, [register::CTRL_MEAS, ctrl_meas]),
            I2cStep::write_read(address, [register::STATUS], [0x00]),
            I2cStep::write_read(address, [register::DATA], DATA),
        ]
    }

    #[test]
    fn init_follows_the_datasheet_sequence_and_keeps_the_calibration() {
        let mut steps = init_steps(I2C_ADDRESS_PRIMARY, 0x00, 0x01, 0x24);
        steps.extend(measure_steps(I2C_ADDRESS_PRIMARY, 0x25));
        let mut sensor = Bme280::i2c(I2cScript::new(steps), I2C_ADDRESS_PRIMARY, DelayLog::new());
        sensor.init().unwrap();
        let expected = Calibration::from_registers(&CALIBRATION_A, &CALIBRATION_B);
        assert_eq!(sensor.calibration(), Some(&expected));

        let measurement = sensor.measure().unwrap();
        assert_eq!(
            measurement,
            expected.compensate(&RawMeasurement::from_registers(&DATA))
        );
        assert_eq!(measurement.temperature_centi_celsius, 2044);

        let (bus, delay) = sensor.release();
        assert!(bus.release().done());
        assert_eq!(delay.waits_ns()[0], STARTUP_MICROS * 1_000);
        assert_eq!(
            delay.waits_ns()[1],
            9_300 * 1_000,
            "x1, x1, x1 is 9.3 ms at most"
        );
    }

    #[test]
    fn oversampling_and_filter_settings_reach_the_control_registers() {
        let config = Config {
            standby: Standby::Ms0_5,
            filter: Filter::X16,
            spi_3wire: false,
        }
        .bits();
        let ctrl_meas_sleep = CtrlMeas {
            temperature: Oversampling::X2,
            pressure: Oversampling::X16,
            mode: Mode::Sleep,
        }
        .bits();
        let mut steps = init_steps(I2C_ADDRESS_SECONDARY, config, 0x00, ctrl_meas_sleep);
        steps.extend(measure_steps(I2C_ADDRESS_SECONDARY, ctrl_meas_sleep | 0x01));
        let mut sensor = Bme280::i2c(
            I2cScript::new(steps),
            I2C_ADDRESS_SECONDARY,
            DelayLog::new(),
        )
        .with_oversampling(Oversampling::X2, Oversampling::X16, Oversampling::Skipped)
        .with_filter(Filter::X16);
        sensor.measure().unwrap();
        let (bus, delay) = sensor.release();
        assert!(bus.release().done());
        let expected =
            max_measurement_micros(Oversampling::X2, Oversampling::X16, Oversampling::Skipped);
        assert_eq!(delay.waits_ns()[1], expected * 1_000);
    }

    #[test]
    fn a_part_that_is_not_a_bme280_is_refused_before_anything_else() {
        let steps = [
            I2cStep::write(0x76, [register::RESET, RESET_WORD]),
            I2cStep::write_read(0x76, [register::STATUS], [0x00]),
            I2cStep::write_read(0x76, [register::CHIP_ID], [0x58]),
        ];
        let mut sensor = Bme280::i2c(I2cScript::new(steps), 0x76, DelayLog::new());
        assert_eq!(
            sensor.init(),
            Err(DriverError::Sensor(SensorError::Identity))
        );
        assert!(sensor.calibration().is_none());
        assert!(sensor.release().0.release().done());
    }

    #[test]
    fn a_conversion_that_never_finishes_is_a_timeout() {
        let mut steps = init_steps(0x76, 0x00, 0x01, 0x24);
        steps.push(I2cStep::write(0x76, [register::CTRL_MEAS, 0x25]));
        for _ in 0..STATUS_POLLS {
            steps.push(I2cStep::write_read(0x76, [register::STATUS], [0x08]));
        }
        let mut sensor = Bme280::i2c(I2cScript::new(steps), 0x76, DelayLog::new());
        assert_eq!(sensor.measure(), Err(DriverError::Timeout));
        assert!(sensor.release().0.release().done());
    }

    #[test]
    fn a_bus_fault_surfaces_as_a_bus_error_and_maps_to_the_core_io_error() {
        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let steps = [I2cStep::fault(0x76, kind)];
        let mut sensor = Bme280::i2c(I2cScript::new(steps), 0x76, DelayLog::new());
        let error = sensor.init().unwrap_err();
        assert!(matches!(error, DriverError::Bus(_)));
        let core: pamoja_core::Error = error.into();
        assert!(matches!(core, pamoja_core::Error::Io(_)));
        let identity: pamoja_core::Error =
            DriverError::<ErrorKind>::Sensor(SensorError::Identity).into();
        assert!(matches!(identity, pamoja_core::Error::Io(_)));
        let crc: pamoja_core::Error = DriverError::<ErrorKind>::Sensor(SensorError::Crc).into();
        assert!(matches!(crc, pamoja_core::Error::Codec(_)));
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_one_channel() {
        let mut steps = init_steps(0x76, 0x00, 0x01, 0x24);
        steps.extend(measure_steps(0x76, 0x25));
        let sensor = Bme280::i2c(I2cScript::new(steps), 0x76, DelayLog::new());
        let mut celsius = sensor.map(|measurement| measurement.celsius());
        let reading = block_on(celsius.read()).unwrap();
        assert!((reading - 20.44).abs() < 0.001);
        assert!(celsius.into_inner().release().0.release().done());
    }

    #[test]
    fn over_spi_the_same_sequence_uses_the_bosch_control_bytes() {
        let steps = [
            SpiStep::write([register::RESET & 0x7F, RESET_WORD]),
            SpiStep::write([register::STATUS | 0x80]),
            SpiStep::read([0x00]),
            SpiStep::write([register::CHIP_ID | 0x80]),
            SpiStep::read([CHIP_ID]),
            SpiStep::write([register::CALIB_TEMP_PRESS | 0x80]),
            SpiStep::read(CALIBRATION_A),
            SpiStep::write([register::CALIB_HUMIDITY | 0x80]),
            SpiStep::read(CALIBRATION_B),
            SpiStep::write([register::CONFIG & 0x7F, 0x00]),
            SpiStep::write([register::CTRL_HUM & 0x7F, 0x01]),
            SpiStep::write([register::CTRL_MEAS & 0x7F, 0x24]),
            SpiStep::write([register::CTRL_MEAS & 0x7F, 0x25]),
            SpiStep::write([register::STATUS | 0x80]),
            SpiStep::read([0x00]),
            SpiStep::write([register::DATA | 0x80]),
            SpiStep::read(DATA),
        ];
        let mut sensor = Bme280::spi(SpiScript::new(steps), DelayLog::new());
        let measurement = sensor.measure().unwrap();
        assert_eq!(measurement.temperature_centi_celsius, 2044);
        assert!(sensor.release().0.release().done());
    }
}
