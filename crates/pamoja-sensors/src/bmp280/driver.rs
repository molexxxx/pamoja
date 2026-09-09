//! The BMP280 driven over a bus: reset, identify, calibrate, and measure on demand.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use embedded_hal::spi::SpiDevice;
use pamoja_core::Sensor;

use super::{
    image_updating, max_measurement_micros, measuring, register, Calibration, Config, CtrlMeas,
    Measurement, Mode, Oversampling, Reading, CALIBRATION_LEN, CHIP_ID, DATA_LEN, RESET_WORD,
    STARTUP_MICROS,
};
use crate::driver::{I2cRegisters, RegisterBus, SpiRegisters};
use crate::error::{DriverError, SensorError};

/// How many times the status register is polled, 1 ms apart, before a conversion or
/// a reset is called overdue.
pub const STATUS_POLLS: u8 = 20;

/// A BMP280 on an I2C or SPI bus, measuring on demand in forced mode.
///
/// [`Bmp280::i2c`] or [`Bmp280::spi`] wraps the bus. [`init`](Bmp280::init) resets the
/// part, checks its chip id, reads its trimming coefficients, and writes the
/// oversampling and filter settings while the part is asleep, the one state in which
/// the datasheet promises a `config` write is not ignored. [`measure`](Bmp280::measure)
/// runs one forced-mode conversion, waits the datasheet's maximum measurement time
/// for the settings in use, confirms the part is idle, and returns the compensated
/// [`Reading`]. As a [`Sensor`] its reading is the whole measurement; one channel is
/// selected with [`Sensor::map`].
///
/// # Examples
///
/// The part's side of the conversation, scripted: what the datasheet says a BMP280
/// answers during initialization and one forced measurement.
///
/// ```
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
/// use pamoja_sensors::bmp280::{Bmp280, I2C_ADDRESS_PRIMARY};
/// use pamoja_core::Sensor;
///
/// const PART: u8 = I2C_ADDRESS_PRIMARY;
/// let trimming = [
///     0x70, 0x6B, 0x43, 0x67, 0x18, 0xFC, 0x7D, 0x8E, 0x43, 0xD6, 0xD0, 0x0B, 0x27, 0x0B,
///     0x8C, 0x00, 0xF9, 0xFF, 0x8C, 0x3C, 0xF8, 0xC6, 0x70, 0x17,
/// ];
/// let bus = I2cScript::new([
///     I2cStep::write(PART, [0xE0, 0xB6]),
///     I2cStep::write_read(PART, [0xF3], [0x00]),
///     I2cStep::write_read(PART, [0xD0], [0x58]),
///     I2cStep::write_read(PART, [0x88], trimming),
///     I2cStep::write(PART, [0xF5, 0x00]),
///     I2cStep::write(PART, [0xF4, 0x24]),
///     I2cStep::write(PART, [0xF4, 0x25]),
///     I2cStep::write_read(PART, [0xF3], [0x00]),
///     I2cStep::write_read(PART, [0xF7], [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00]),
/// ]);
///
/// let mut sensor = Bmp280::i2c(bus, PART, DelayLog::new());
/// let reading = block_on(sensor.read())?;
/// assert_eq!(reading.temperature_centi_celsius, 2508);
/// assert_eq!(reading.pascals(), 100_653);
///
/// let (registers, delay) = sensor.release();
/// assert!(registers.release().done());
/// assert!(delay.total_micros() >= 2_000 + 6_425);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Bmp280<B, D> {
    bus: B,
    delay: D,
    temperature: Oversampling,
    pressure: Oversampling,
    filter: u8,
    calibration: Option<Calibration>,
}

impl<I2C: I2c, D> Bmp280<I2cRegisters<I2C>, D> {
    /// Wraps an I2C bus and the part's address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `address` - [`I2C_ADDRESS_PRIMARY`](super::I2C_ADDRESS_PRIMARY) with SDO
    ///   to ground, [`I2C_ADDRESS_SECONDARY`](super::I2C_ADDRESS_SECONDARY) with SDO
    ///   to VDDIO.
    /// * `delay` - the timer that paces the reset and the conversions.
    ///
    /// # Returns
    ///
    /// The driver, with both measurements at oversampling x1 and the filter off.
    pub fn i2c(bus: I2C, address: u8, delay: D) -> Self {
        Bmp280::new(I2cRegisters::new(bus, address), delay)
    }
}

impl<SPI: SpiDevice, D> Bmp280<SpiRegisters<SPI>, D> {
    /// Wraps an SPI device, the bus plus the part's chip-select line.
    ///
    /// # Arguments
    ///
    /// * `device` - the SPI device, in mode 0 or 3.
    /// * `delay` - the timer that paces the reset and the conversions.
    ///
    /// # Returns
    ///
    /// The driver, with both measurements at oversampling x1 and the filter off.
    pub fn spi(device: SPI, delay: D) -> Self {
        Bmp280::new(SpiRegisters::new(device), delay)
    }
}

impl<B, D> Bmp280<B, D> {
    /// Wraps any register bus.
    ///
    /// # Arguments
    ///
    /// * `bus` - the part's register map, over whichever bus reaches it.
    /// * `delay` - the timer that paces the reset and the conversions.
    ///
    /// # Returns
    ///
    /// The driver, with both measurements at oversampling x1 and the filter off.
    pub fn new(bus: B, delay: D) -> Self {
        Bmp280 {
            bus,
            delay,
            temperature: Oversampling::X1,
            pressure: Oversampling::X1,
            filter: 0,
            calibration: None,
        }
    }

    /// Sets the oversampling of each measurement; `Skipped` leaves one out.
    ///
    /// The settings are written to the part by [`init`](Bmp280::init).
    ///
    /// # Arguments
    ///
    /// * `temperature` - temperature oversampling.
    /// * `pressure` - pressure oversampling.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_oversampling(mut self, temperature: Oversampling, pressure: Oversampling) -> Self {
        self.temperature = temperature;
        self.pressure = pressure;
        self
    }

    /// Sets the IIR filter that smooths pressure and temperature.
    ///
    /// The setting is written to the part by [`init`](Bmp280::init).
    ///
    /// # Arguments
    ///
    /// * `code` - the raw three-bit `filter[2:0]` code, `0` for the filter off. The
    ///   datasheet lists the coefficients without publishing which code selects
    ///   which, so the code is written as given; only its low three bits are used.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_filter(mut self, code: u8) -> Self {
        self.filter = code;
        self
    }

    /// Returns the trimming coefficients read at initialization, if it has happened.
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

impl<B: RegisterBus, D: DelayNs> Bmp280<B, D> {
    /// Resets the part, checks its identity, reads its trimming coefficients, and
    /// writes the settings.
    ///
    /// The sequence is the datasheet's: the soft-reset word of section 4.3.2, which
    /// runs the complete power-on-reset procedure, the start-up time of Table 2, a
    /// wait for the `im_update` flag of Table 19 to clear, the chip id of section
    /// 4.3.1, the trimming block of Table 17, then `config` and `ctrl_meas` (sections
    /// 4.3.5 and 4.3.4) written while the part is asleep, leaving it asleep.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Timeout`] if the
    /// trimming image never finishes loading, and [`DriverError::Sensor`] with
    /// [`SensorError::Identity`] if the chip id is not the 0x58 section 4.3.1 fixes
    /// for a BMP280; the datasheet names no other value.
    pub fn init(&mut self) -> Result<(), DriverError<B::Error>> {
        self.write(register::RESET, RESET_WORD)?;
        self.delay.delay_us(STARTUP_MICROS);
        self.wait(image_updating)?;

        if self.read(register::CHIP_ID)? != CHIP_ID {
            return Err(SensorError::Identity.into());
        }

        let mut trimming = [0u8; CALIBRATION_LEN];
        self.bus
            .read_registers(register::CALIBRATION, &mut trimming)
            .map_err(DriverError::Bus)?;
        self.calibration = Some(Calibration::parse(&trimming));

        let config = Config {
            filter: self.filter,
            ..Config::default()
        };
        self.write(register::CONFIG, config.bits())?;
        self.write(register::CTRL_MEAS, self.ctrl_meas(Mode::Sleep).bits())?;
        Ok(())
    }

    /// Runs one forced-mode conversion and returns the compensated reading.
    ///
    /// Initializes the part first if [`init`](Bmp280::init) has not run. The sequence
    /// is the datasheet's forced mode (section 3.6.2): `ctrl_meas` with the forced
    /// code of Table 10, a wait of the maximum measurement time of section 3.8.1,
    /// which is how section 3.9 asks forced-mode readout to be timed, the `measuring`
    /// flag of Table 19 polled until it clears, then the single burst read of 0xF7 to
    /// 0xFC that section 3.9 prescribes.
    ///
    /// # Returns
    ///
    /// The compensated reading.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Timeout`] if the
    /// part is still measuring after the polls, or any error of
    /// [`init`](Bmp280::init).
    pub fn measure(&mut self) -> Result<Reading, DriverError<B::Error>> {
        if self.calibration.is_none() {
            self.init()?;
        }
        self.write(register::CTRL_MEAS, self.ctrl_meas(Mode::Forced).bits())?;
        self.delay
            .delay_us(max_measurement_micros(self.temperature, self.pressure));
        self.wait(measuring)?;

        let mut data = [0u8; DATA_LEN];
        self.bus
            .read_registers(register::DATA, &mut data)
            .map_err(DriverError::Bus)?;
        let raw = Measurement::parse(&data);
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

impl<B, D> Sensor for Bmp280<B, D>
where
    B: RegisterBus,
    B::Error: core::fmt::Debug,
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
    use crate::bmp280::{Standby, I2C_ADDRESS_PRIMARY, I2C_ADDRESS_SECONDARY};
    use crate::driver::{SPI_ADDRESS_MASK, SPI_READ};
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep, SpiScript, SpiStep};

    // The Table 17 layout bytes and the burst read the decode module pins its
    // reference vector to: 25.08 degrees C and 100653 Pa.
    const CALIBRATION: [u8; CALIBRATION_LEN] = [
        0x70, 0x6B, 0x43, 0x67, 0x18, 0xFC, 0x7D, 0x8E, 0x43, 0xD6, 0xD0, 0x0B, 0x27, 0x0B, 0x8C,
        0x00, 0xF9, 0xFF, 0x8C, 0x3C, 0xF8, 0xC6, 0x70, 0x17,
    ];
    const DATA: [u8; DATA_LEN] = [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00];

    fn init_steps(address: u8, config: u8, ctrl_meas: u8) -> Vec<I2cStep> {
        vec![
            I2cStep::write(address, [register::RESET, RESET_WORD]),
            I2cStep::write_read(address, [register::STATUS], [0x00]),
            I2cStep::write_read(address, [register::CHIP_ID], [CHIP_ID]),
            I2cStep::write_read(address, [register::CALIBRATION], CALIBRATION),
            I2cStep::write(address, [register::CONFIG, config]),
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
        let mut steps = init_steps(I2C_ADDRESS_PRIMARY, 0x00, 0x24);
        steps.extend(measure_steps(I2C_ADDRESS_PRIMARY, 0x25));
        let mut sensor = Bmp280::i2c(I2cScript::new(steps), I2C_ADDRESS_PRIMARY, DelayLog::new());
        sensor.init().unwrap();
        let expected = Calibration::parse(&CALIBRATION);
        assert_eq!(sensor.calibration(), Some(&expected));

        let reading = sensor.measure().unwrap();
        assert_eq!(reading, expected.compensate(&Measurement::parse(&DATA)));
        assert_eq!(reading.temperature_centi_celsius, 2508);
        assert_eq!(reading.pascals(), 100_653);

        let (bus, delay) = sensor.release();
        assert!(bus.release().done());
        assert_eq!(delay.waits_ns()[0], STARTUP_MICROS * 1_000);
        assert_eq!(
            delay.waits_ns()[1],
            6_425 * 1_000,
            "x1, x1 is 6.425 ms at most"
        );
    }

    #[test]
    fn oversampling_and_filter_settings_reach_the_control_registers() {
        let config = Config {
            standby: Standby::Ms0_5,
            filter: 0b100,
            spi_3wire: false,
        }
        .bits();
        let ctrl_meas_sleep = CtrlMeas {
            temperature: Oversampling::X2,
            pressure: Oversampling::X16,
            mode: Mode::Sleep,
        }
        .bits();
        let mut steps = init_steps(I2C_ADDRESS_SECONDARY, config, ctrl_meas_sleep);
        steps.extend(measure_steps(I2C_ADDRESS_SECONDARY, ctrl_meas_sleep | 0x01));
        let mut sensor = Bmp280::i2c(
            I2cScript::new(steps),
            I2C_ADDRESS_SECONDARY,
            DelayLog::new(),
        )
        .with_oversampling(Oversampling::X2, Oversampling::X16)
        .with_filter(0b100);
        sensor.measure().unwrap();
        let (bus, delay) = sensor.release();
        assert!(bus.release().done());
        let expected = max_measurement_micros(Oversampling::X2, Oversampling::X16);
        assert_eq!(delay.waits_ns()[1], expected * 1_000);
        assert_eq!(
            expected, 43_225,
            "the ultra high resolution row of Table 13, 43.2 ms"
        );
    }

    #[test]
    fn init_waits_while_the_trimming_image_is_still_loading() {
        let mut steps = vec![
            I2cStep::write(0x76, [register::RESET, RESET_WORD]),
            I2cStep::write_read(0x76, [register::STATUS], [0x01]),
            I2cStep::write_read(0x76, [register::STATUS], [0x01]),
        ];
        steps.extend(init_steps(0x76, 0x00, 0x24).into_iter().skip(1));
        let mut sensor = Bmp280::i2c(I2cScript::new(steps), 0x76, DelayLog::new());
        sensor.init().unwrap();
        let (bus, delay) = sensor.release();
        assert!(bus.release().done());
        assert_eq!(
            delay.waits_ns(),
            [STARTUP_MICROS * 1_000, 1_000_000, 1_000_000]
        );

        let mut steps = vec![I2cStep::write(0x76, [register::RESET, RESET_WORD])];
        for _ in 0..STATUS_POLLS {
            steps.push(I2cStep::write_read(0x76, [register::STATUS], [0x01]));
        }
        let mut sensor = Bmp280::i2c(I2cScript::new(steps), 0x76, DelayLog::new());
        assert_eq!(sensor.init(), Err(DriverError::Timeout));
        assert!(sensor.calibration().is_none());
        assert!(sensor.release().0.release().done());
    }

    #[test]
    fn a_part_that_is_not_a_bmp280_is_refused_before_anything_else() {
        let steps = [
            I2cStep::write(0x76, [register::RESET, RESET_WORD]),
            I2cStep::write_read(0x76, [register::STATUS], [0x00]),
            I2cStep::write_read(0x76, [register::CHIP_ID], [0x60]),
        ];
        let mut sensor = Bmp280::i2c(I2cScript::new(steps), 0x76, DelayLog::new());
        assert_eq!(
            sensor.init(),
            Err(DriverError::Sensor(SensorError::Identity))
        );
        assert!(sensor.calibration().is_none());
        assert!(sensor.release().0.release().done());
    }

    #[test]
    fn a_conversion_that_never_finishes_is_a_timeout() {
        let mut steps = init_steps(0x76, 0x00, 0x24);
        steps.push(I2cStep::write(0x76, [register::CTRL_MEAS, 0x25]));
        for _ in 0..STATUS_POLLS {
            steps.push(I2cStep::write_read(0x76, [register::STATUS], [0x08]));
        }
        let mut sensor = Bmp280::i2c(I2cScript::new(steps), 0x76, DelayLog::new());
        assert_eq!(sensor.measure(), Err(DriverError::Timeout));
        assert!(sensor.release().0.release().done());
    }

    #[test]
    fn a_bus_fault_surfaces_as_a_bus_error_and_maps_to_the_core_io_error() {
        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let steps = [I2cStep::fault(0x76, kind)];
        let mut sensor = Bmp280::i2c(I2cScript::new(steps), 0x76, DelayLog::new());
        let error = sensor.init().unwrap_err();
        assert!(matches!(error, DriverError::Bus(_)));
        let core: pamoja_core::Error = error.into();
        assert!(matches!(core, pamoja_core::Error::Io(_)));
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_one_channel() {
        let mut steps = init_steps(0x76, 0x00, 0x24);
        steps.extend(measure_steps(0x76, 0x25));
        let sensor = Bmp280::i2c(I2cScript::new(steps), 0x76, DelayLog::new());
        let mut pascals = sensor.map(|reading| reading.pascals());
        assert_eq!(block_on(pascals.read()).unwrap(), 100_653);
        assert!(pascals.into_inner().release().0.release().done());
    }

    #[test]
    fn over_spi_the_same_sequence_uses_the_bosch_control_bytes() {
        // Section 5.3: 0xF7 is written through control byte 0x77 and read through
        // 0xF7; Figure 10 writes ctrl_meas through 0x74.
        assert_eq!(register::DATA & SPI_ADDRESS_MASK, 0x77);
        assert_eq!(register::DATA | SPI_READ, 0xF7);
        assert_eq!(register::CTRL_MEAS & SPI_ADDRESS_MASK, 0x74);
        let steps = [
            SpiStep::write([register::RESET & SPI_ADDRESS_MASK, RESET_WORD]),
            SpiStep::write([register::STATUS | SPI_READ]),
            SpiStep::read([0x00]),
            SpiStep::write([register::CHIP_ID | SPI_READ]),
            SpiStep::read([CHIP_ID]),
            SpiStep::write([register::CALIBRATION | SPI_READ]),
            SpiStep::read(CALIBRATION),
            SpiStep::write([register::CONFIG & SPI_ADDRESS_MASK, 0x00]),
            SpiStep::write([register::CTRL_MEAS & SPI_ADDRESS_MASK, 0x24]),
            SpiStep::write([register::CTRL_MEAS & SPI_ADDRESS_MASK, 0x25]),
            SpiStep::write([register::STATUS | SPI_READ]),
            SpiStep::read([0x00]),
            SpiStep::write([register::DATA | SPI_READ]),
            SpiStep::read(DATA),
        ];
        let mut sensor = Bmp280::spi(SpiScript::new(steps), DelayLog::new());
        let reading = sensor.measure().unwrap();
        assert_eq!(reading.temperature_centi_celsius, 2508);
        assert!(sensor.release().0.release().done());
    }
}
