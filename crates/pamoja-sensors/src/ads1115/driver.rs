//! The ADS1115 driven over I2C: configure, start a single-shot conversion, wait, read.

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use pamoja_core::Sensor;

use super::{conversion_micros, register, Config, DataRate, Mode, Mux, Pga, Sample};
use crate::driver::{read_word, write_word};
use crate::error::{DriverError, SensorError};

/// How many times the Config register is polled, 1 ms apart, after the conversion
/// time has elapsed before the conversion is called overdue.
pub const CONVERSION_POLLS: u8 = 10;

/// An ADS1115 on an I2C bus, converting one input on demand.
///
/// The part is left in its power-down single-shot state between conversions, which
/// is its reset state. [`init`](Ads1115::init) writes the input, gain, and data
/// rate to the Config register and reads it back, which is the only identity check
/// a part without an id register offers. [`sample`](Ads1115::sample) sets the
/// operational-status bit to start a conversion, waits one period of the data rate
/// plus the datasheet's ten percent rate variation, polls the status bit until the
/// part reports idle, and reads the Conversion register. The reading carries the
/// gain it was taken at, so [`Sample::volts`] needs nothing else. As a [`Sensor`]
/// its reading is the [`Sample`]; [`Sensor::map`] with a
/// [`Calibration`](https://docs.rs/pamoja-kit/latest/pamoja_kit/struct.Calibration.html)
/// turns it into the physical quantity an analog probe measures.
///
/// # Examples
///
/// The part's side of the conversation, scripted: the Config register written and
/// read back at initialization, then one conversion of AIN0 against ground at the
/// default gain, which lands at 0.2 V.
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
/// use pamoja_sensors::ads1115::{address, Ads1115, Mux};
///
/// const PART: u8 = address::GND;
/// let bus = I2cScript::new([
///     I2cStep::write(PART, [0x01, 0x45, 0x83]),
///     I2cStep::write_read(PART, [0x01], [0x45, 0x83]),
///     I2cStep::write(PART, [0x01, 0xC5, 0x83]),
///     I2cStep::write_read(PART, [0x01], [0xC5, 0x83]),
///     I2cStep::write_read(PART, [0x00], [0x0C, 0x80]),
/// ]);
///
/// let adc = Ads1115::new(bus, PART, DelayLog::new()).with_input(Mux::Ain0Gnd);
/// let mut volts = adc.map(|sample| sample.volts());
/// let reading = block_on(volts.read())?;
/// assert!((reading - 0.2).abs() < 1e-6);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Ads1115<I2C, D> {
    bus: I2C,
    delay: D,
    address: u8,
    config: Config,
    initialized: bool,
}

impl<I2C, D> Ads1115<I2C, D> {
    /// Wraps an I2C bus and the part's address.
    ///
    /// # Arguments
    ///
    /// * `bus` - the I2C bus the part is on.
    /// * `address` - the address the ADDR pin selects, from [`address`](super::address).
    /// * `delay` - the timer that paces the conversions.
    ///
    /// # Returns
    ///
    /// The driver, converting AIN0 against AIN1 at the ±2.048 V range and 128
    /// samples per second, the part's reset settings.
    pub fn new(bus: I2C, address: u8, delay: D) -> Self {
        Ads1115 {
            bus,
            delay,
            address,
            config: Config {
                start_conversion: false,
                mode: Mode::SingleShot,
                ..Config::default()
            },
            initialized: false,
        }
    }

    /// Selects the input the part converts.
    ///
    /// # Arguments
    ///
    /// * `mux` - the differential pair or the single-ended input.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_input(mut self, mux: Mux) -> Self {
        self.config.mux = mux;
        self.initialized = false;
        self
    }

    /// Selects the full-scale range, and with it the size of one count.
    ///
    /// # Arguments
    ///
    /// * `pga` - the range; the input must stay within the supply regardless.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_gain(mut self, pga: Pga) -> Self {
        self.config.pga = pga;
        self.initialized = false;
        self
    }

    /// Selects the data rate, and with it the conversion time and the noise.
    ///
    /// # Arguments
    ///
    /// * `rate` - samples per second.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_data_rate(mut self, rate: DataRate) -> Self {
        self.config.data_rate = rate;
        self.initialized = false;
        self
    }

    /// Returns the configuration the driver writes to the part.
    pub fn config(&self) -> Config {
        self.config
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

impl<I2C: I2c, D: DelayNs> Ads1115<I2C, D> {
    /// Writes the input, gain, and data rate and reads the Config register back.
    ///
    /// The operational-status bit is left clear so no conversion starts. Reading the
    /// register back is the identity check: a part that does not hold what was
    /// written is not an ADS1115 in its power-down state.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, and [`DriverError::Sensor`]
    /// with [`SensorError::Identity`] if the register reads back differently.
    pub fn init(&mut self) -> Result<(), DriverError<I2C::Error>> {
        let written = self.config.bits();
        write_word(&mut self.bus, self.address, register::CONFIG, written)
            .map_err(DriverError::Bus)?;
        let read =
            read_word(&mut self.bus, self.address, register::CONFIG).map_err(DriverError::Bus)?;
        if Config::from_bits(read).bits() & 0x7FFF != written & 0x7FFF {
            return Err(SensorError::Identity.into());
        }
        self.initialized = true;
        Ok(())
    }

    /// Runs one conversion of the configured input and returns the sample.
    ///
    /// Initializes the part first if [`init`](Ads1115::init) has not run. The
    /// sequence is the datasheet's single-shot mode: the Config register written
    /// with the operational-status bit set, the conversion time for the data rate,
    /// the status bit polled until the part is idle, then the Conversion register.
    ///
    /// # Returns
    ///
    /// The sample, carrying the gain it was taken at.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if the bus fails, [`DriverError::Timeout`] if
    /// the part never reports idle, or any error of [`init`](Ads1115::init).
    pub fn sample(&mut self) -> Result<Sample, DriverError<I2C::Error>> {
        if !self.initialized {
            self.init()?;
        }
        let start = Config {
            start_conversion: true,
            ..self.config
        };
        write_word(&mut self.bus, self.address, register::CONFIG, start.bits())
            .map_err(DriverError::Bus)?;
        self.delay
            .delay_us(conversion_micros(self.config.data_rate));

        let mut idle = false;
        for _ in 0..CONVERSION_POLLS {
            let status = read_word(&mut self.bus, self.address, register::CONFIG)
                .map_err(DriverError::Bus)?;
            if Config::from_bits(status).start_conversion {
                idle = true;
                break;
            }
            self.delay.delay_ms(1);
        }
        if !idle {
            return Err(DriverError::Timeout);
        }

        let raw = read_word(&mut self.bus, self.address, register::CONVERSION)
            .map_err(DriverError::Bus)?;
        Ok(Sample {
            raw: raw as i16,
            pga: self.config.pga,
        })
    }

    /// Converts one input once without changing the configured input.
    ///
    /// # Arguments
    ///
    /// * `mux` - the input to convert this once.
    ///
    /// # Returns
    ///
    /// The sample.
    ///
    /// # Errors
    ///
    /// The errors of [`sample`](Ads1115::sample).
    pub fn sample_input(&mut self, mux: Mux) -> Result<Sample, DriverError<I2C::Error>> {
        let configured = self.config.mux;
        self.config.mux = mux;
        self.initialized = false;
        let result = self.sample();
        self.config.mux = configured;
        self.initialized = false;
        result
    }
}

impl<I2C, D> Sensor for Ads1115<I2C, D>
where
    I2C: I2c + Send,
    I2C::Error: core::fmt::Debug,
    D: DelayNs + Send,
{
    type Reading = Sample;

    async fn read(&mut self) -> pamoja_core::Result<Sample> {
        self.sample().map_err(pamoja_core::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ads1115::address;
    use alloc::vec;
    use alloc::vec::Vec;
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};

    fn config(mux: Mux, pga: Pga, rate: DataRate, start: bool) -> [u8; 2] {
        Config {
            start_conversion: start,
            mux,
            pga,
            mode: Mode::SingleShot,
            data_rate: rate,
            ..Config::default()
        }
        .bits()
        .to_be_bytes()
    }

    fn init_steps(address: u8, settings: [u8; 2]) -> Vec<I2cStep> {
        vec![
            I2cStep::write(address, [register::CONFIG, settings[0], settings[1]]),
            I2cStep::write_read(address, [register::CONFIG], settings),
        ]
    }

    fn sample_steps(address: u8, started: [u8; 2], raw: i16) -> Vec<I2cStep> {
        vec![
            I2cStep::write(address, [register::CONFIG, started[0], started[1]]),
            I2cStep::write_read(address, [register::CONFIG], started),
            I2cStep::write_read(address, [register::CONVERSION], raw.to_be_bytes()),
        ]
    }

    #[test]
    fn a_single_shot_conversion_follows_the_datasheet_sequence() {
        let (mux, pga, rate) = (Mux::Ain0Gnd, Pga::Fsr4_096, DataRate::Sps860);
        let mut steps = init_steps(address::SDA, config(mux, pga, rate, false));
        steps.extend(sample_steps(
            address::SDA,
            config(mux, pga, rate, true),
            -1024,
        ));
        let mut adc = Ads1115::new(I2cScript::new(steps), address::SDA, DelayLog::new())
            .with_input(mux)
            .with_gain(pga)
            .with_data_rate(rate);
        let sample = adc.sample().unwrap();
        assert_eq!(sample.raw, -1024);
        assert_eq!(sample.pga, pga);
        assert!((sample.volts() + 0.128).abs() < 1e-6);
        let (bus, delay) = adc.release();
        assert!(bus.done());
        assert_eq!(delay.waits_ns(), [1_280_000], "860 SPS plus ten percent");
    }

    #[test]
    fn the_readback_identity_check_refuses_a_part_that_holds_something_else() {
        let settings = config(Mux::Ain0Ain1, Pga::Fsr2_048, DataRate::Sps128, false);
        let steps = [
            I2cStep::write(address::GND, [register::CONFIG, settings[0], settings[1]]),
            I2cStep::write_read(address::GND, [register::CONFIG], [0x00, 0x00]),
        ];
        let mut adc = Ads1115::new(I2cScript::new(steps), address::GND, DelayLog::new());
        assert_eq!(adc.init(), Err(DriverError::Sensor(SensorError::Identity)));
    }

    #[test]
    fn a_conversion_that_never_finishes_is_a_timeout() {
        let settings = config(Mux::Ain0Ain1, Pga::Fsr2_048, DataRate::Sps128, false);
        let started = config(Mux::Ain0Ain1, Pga::Fsr2_048, DataRate::Sps128, true);
        let mut steps = init_steps(address::GND, settings);
        steps.push(I2cStep::write(
            address::GND,
            [register::CONFIG, started[0], started[1]],
        ));
        for _ in 0..CONVERSION_POLLS {
            steps.push(I2cStep::write_read(
                address::GND,
                [register::CONFIG],
                settings,
            ));
        }
        let mut adc = Ads1115::new(I2cScript::new(steps), address::GND, DelayLog::new());
        assert_eq!(adc.sample(), Err(DriverError::Timeout));
        let (bus, delay) = adc.release();
        assert!(bus.done());
        assert_eq!(delay.total_micros(), 8_595 + 10_000);
    }

    #[test]
    fn a_bus_fault_surfaces_as_a_bus_error() {
        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let mut adc = Ads1115::new(
            I2cScript::new([I2cStep::fault(address::GND, kind)]),
            address::GND,
            DelayLog::new(),
        );
        assert!(matches!(adc.init(), Err(DriverError::Bus(_))));
    }

    #[test]
    fn sampling_another_input_once_leaves_the_configured_one_in_place() {
        let (pga, rate) = (Pga::Fsr2_048, DataRate::Sps128);
        let mut steps = init_steps(address::GND, config(Mux::Ain1Gnd, pga, rate, false));
        steps.extend(sample_steps(
            address::GND,
            config(Mux::Ain1Gnd, pga, rate, true),
            100,
        ));
        steps.extend(init_steps(
            address::GND,
            config(Mux::Ain0Gnd, pga, rate, false),
        ));
        steps.extend(sample_steps(
            address::GND,
            config(Mux::Ain0Gnd, pga, rate, true),
            3200,
        ));
        let mut adc = Ads1115::new(I2cScript::new(steps), address::GND, DelayLog::new())
            .with_input(Mux::Ain0Gnd);
        assert_eq!(adc.sample_input(Mux::Ain1Gnd).unwrap().raw, 100);
        assert_eq!(adc.config().mux, Mux::Ain0Gnd);
        assert_eq!(adc.sample().unwrap().raw, 3200);
        assert!(adc.release().0.done());
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_volts() {
        let (mux, pga, rate) = (Mux::Ain0Gnd, Pga::Fsr2_048, DataRate::Sps128);
        let mut steps = init_steps(address::GND, config(mux, pga, rate, false));
        steps.extend(sample_steps(
            address::GND,
            config(mux, pga, rate, true),
            3200,
        ));
        let adc =
            Ads1115::new(I2cScript::new(steps), address::GND, DelayLog::new()).with_input(mux);
        let mut volts = adc.map(|sample| sample.volts());
        let reading = block_on(volts.read()).unwrap();
        assert!((reading - 0.2).abs() < 1e-6);
    }
}
