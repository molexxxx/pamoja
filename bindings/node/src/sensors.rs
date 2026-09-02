//! Generated Node bindings for the sensor drivers.
//!
//! These mirror the `pamoja-sensors` Rust API: the decode half of four common
//! parts, turning the register bytes a bus driver read into the physical reading
//! the datasheet says they mean.
//!
//! A BME280's calibration is read once at start-up and reused for every
//! measurement, so it is a class. Everything else is a plain function over the
//! bytes or the register value a caller already holds.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_sensors::{ads1115, bme280, ds18b20, ina219, SensorError};

/// A compensated BME280 reading.
#[napi(object)]
pub struct Bme280Measurement {
    /// The temperature in degrees Celsius.
    pub celsius: f64,
    /// The pressure in pascals.
    pub pascals: u32,
    /// The pressure in hectopascals, the unit a barometer is usually quoted in.
    pub hectopascals: f64,
    /// The relative humidity as a percentage.
    pub relative_humidity_percent: f64,
}

/// A decoded DS18B20 scratchpad.
#[napi(object, js_name = "Ds18b20Reading")]
pub struct Ds18b20Reading {
    /// The raw temperature register, 1/16 degree Celsius per count.
    pub raw_temperature: i16,
    /// The temperature in micro-degrees Celsius, exact in integer arithmetic.
    pub micro_celsius: i32,
    /// The temperature in degrees Celsius.
    pub celsius: f64,
    /// The high alarm threshold in whole degrees Celsius.
    pub alarm_high: i8,
    /// The low alarm threshold in whole degrees Celsius.
    pub alarm_low: i8,
    /// The configured resolution, as a number of bits: 9, 10, 11, or 12.
    pub resolution_bits: u8,
}

/// An ADS1115 configuration register, field by field.
#[napi(object)]
pub struct Ads1115Config {
    /// Whether writing this starts a single conversion.
    pub start_conversion: bool,
    /// The input multiplexer code, `0..=7`.
    pub mux: u8,
    /// The gain code, `0..=7`, which sets the full-scale range.
    pub pga: u8,
    /// Whether to convert once per request and power down, rather than continuously.
    pub single_shot: bool,
    /// The data rate code, `0..=7`.
    pub data_rate: u8,
    /// Whether to use the window comparator rather than the traditional one.
    pub window_comparator: bool,
    /// Whether the ALERT/RDY pin is active high.
    pub comparator_active_high: bool,
    /// Whether the comparator latches until the conversion is read.
    pub comparator_latching: bool,
    /// The comparator queue code, `0..=3`, where `3` disables the comparator.
    pub comparator_queue: u8,
}

/// A BME280's factory calibration, read once and reused for every measurement.
#[napi]
pub struct Bme280Calibration {
    inner: bme280::Calibration,
}

#[napi]
impl Bme280Calibration {
    /// Builds a calibration from the bytes read out of the device's registers.
    ///
    /// `tempPress` is the 26-byte block and `humidity` the 7-byte one.
    #[napi(constructor)]
    pub fn new(temp_press: Buffer, humidity: Buffer) -> napi::Result<Self> {
        let temp_press: [u8; 26] = temp_press
            .as_ref()
            .try_into()
            .map_err(|_| length_error("temperature and pressure calibration", 26))?;
        let humidity: [u8; 7] = humidity
            .as_ref()
            .try_into()
            .map_err(|_| length_error("humidity calibration", 7))?;
        Ok(Self {
            inner: bme280::Calibration::from_registers(&temp_press, &humidity),
        })
    }

    /// Turns an eight-byte burst read into a compensated reading.
    #[napi]
    pub fn compensate(&self, measurement: Buffer) -> napi::Result<Bme280Measurement> {
        let registers: [u8; 8] = measurement
            .as_ref()
            .try_into()
            .map_err(|_| length_error("measurement", 8))?;
        let reading = self
            .inner
            .compensate(&bme280::RawMeasurement::from_registers(&registers));
        Ok(Bme280Measurement {
            celsius: f64::from(reading.celsius()),
            pascals: reading.pascals(),
            hectopascals: f64::from(reading.hectopascals()),
            relative_humidity_percent: f64::from(reading.relative_humidity_percent()),
        })
    }
}

/// Parses and CRC-checks a nine-byte DS18B20 scratchpad.
#[napi(js_name = "ds18b20ParseScratchpad")]
pub fn ds18b20_parse_scratchpad(bytes: Buffer) -> napi::Result<Ds18b20Reading> {
    let scratchpad: [u8; 9] = bytes
        .as_ref()
        .try_into()
        .map_err(|_| length_error("scratchpad", 9))?;
    let reading = ds18b20::Scratchpad::parse(&scratchpad).map_err(to_napi)?;
    Ok(Ds18b20Reading {
        raw_temperature: reading.raw_temperature(),
        micro_celsius: reading.temperature_micro_celsius(),
        celsius: f64::from(reading.temperature_celsius()),
        alarm_high: reading.alarm_high(),
        alarm_low: reading.alarm_low(),
        resolution_bits: reading.resolution().bits(),
    })
}

/// Computes the Maxim CRC-8 a 1-Wire device checks its own bytes with.
#[napi(js_name = "ds18b20Crc8")]
pub fn ds18b20_crc8(data: Buffer) -> u8 {
    ds18b20::crc8(data.as_ref())
}

/// Converts a raw DS18B20 temperature register to micro-degrees Celsius.
#[napi(js_name = "ds18b20MicroCelsius")]
pub fn ds18b20_micro_celsius(raw: i16) -> i32 {
    ds18b20::temperature_to_micro_celsius(raw)
}

/// Converts a raw DS18B20 temperature register to degrees Celsius.
#[napi(js_name = "ds18b20Celsius")]
pub fn ds18b20_celsius(raw: i16) -> f64 {
    f64::from(ds18b20::temperature_to_celsius(raw))
}

/// Returns the configuration byte that selects a DS18B20 resolution.
#[napi(js_name = "ds18b20ConfigByte")]
pub fn ds18b20_config_byte(bits: u8) -> napi::Result<u8> {
    Ok(resolution(bits)?.config_byte())
}

/// Returns the resolution a DS18B20 configuration byte selects, in bits.
#[napi(js_name = "ds18b20ResolutionBits")]
pub fn ds18b20_resolution_bits(config_byte: u8) -> u8 {
    ds18b20::Resolution::from_config_byte(config_byte).bits()
}

/// Returns the temperature step a DS18B20 resolution resolves, in micro-degrees.
#[napi(js_name = "ds18b20StepMicroCelsius")]
pub fn ds18b20_step_micro_celsius(bits: u8) -> napi::Result<u32> {
    Ok(resolution(bits)?.step_micro_celsius())
}

/// Returns how long a DS18B20 conversion may take at a resolution, in microseconds.
#[napi(js_name = "ds18b20MaxConversionMicros")]
pub fn ds18b20_max_conversion_micros(bits: u8) -> napi::Result<u32> {
    Ok(resolution(bits)?.max_conversion_micros())
}

/// Computes the INA219 calibration register for a shunt and current resolution.
#[napi]
pub fn ina219_calibration(current_lsb_microamps: u32, shunt_milliohms: u32) -> u16 {
    ina219::calibration(current_lsb_microamps, shunt_milliohms)
}

/// Returns the smallest current resolution that still covers an expected maximum.
#[napi]
pub fn ina219_minimum_current_lsb_microamps(max_expected_microamps: u32) -> u32 {
    ina219::minimum_current_lsb_microamps(max_expected_microamps)
}

/// Converts a raw INA219 shunt-voltage register to microvolts.
#[napi]
pub fn ina219_shunt_microvolts(raw: i16) -> i32 {
    ina219::shunt_microvolts(raw)
}

/// Converts a raw INA219 bus-voltage register to millivolts.
#[napi]
pub fn ina219_bus_millivolts(raw: u16) -> u32 {
    ina219::bus_millivolts(raw)
}

/// Reports whether an INA219 bus-voltage register says a conversion is ready.
#[napi]
pub fn ina219_conversion_ready(raw: u16) -> bool {
    ina219::conversion_ready(raw)
}

/// Reports whether an INA219 bus-voltage register flags a math overflow.
#[napi]
pub fn ina219_math_overflow(raw: u16) -> bool {
    ina219::math_overflow(raw)
}

/// Converts a raw INA219 current register to microamps.
#[napi]
pub fn ina219_current_microamps(raw: i16, current_lsb_microamps: u32) -> i32 {
    ina219::current_microamps(raw, current_lsb_microamps)
}

/// Converts a raw INA219 power register to microwatts.
#[napi]
pub fn ina219_power_microwatts(raw: u16, current_lsb_microamps: u32) -> u32 {
    ina219::power_microwatts(raw, current_lsb_microamps)
}

/// Assembles the 16-bit ADS1115 configuration register value.
#[napi]
pub fn ads1115_config_bits(config: Ads1115Config) -> u16 {
    ads1115::Config::from(config).bits()
}

/// Parses a 16-bit ADS1115 configuration register value.
#[napi]
pub fn ads1115_config_from_bits(bits: u16) -> Ads1115Config {
    ads1115::Config::from_bits(bits).into()
}

/// Returns the full-scale range an ADS1115 gain code selects, in microvolts.
#[napi]
pub fn ads1115_full_scale_microvolts(pga: u8) -> u32 {
    ads1115::Pga::from_code(pga).full_scale_microvolts()
}

/// Returns the sample rate an ADS1115 data-rate code selects.
#[napi]
pub fn ads1115_samples_per_second(data_rate: u8) -> u16 {
    ads1115::DataRate::from_code(data_rate).samples_per_second()
}

/// Converts a raw ADS1115 conversion result to nanovolts.
#[napi]
pub fn ads1115_to_nanovolts(pga: u8, raw: i16) -> i64 {
    ads1115::to_nanovolts(ads1115::Pga::from_code(pga), raw)
}

/// Converts a raw ADS1115 conversion result to volts.
#[napi]
pub fn ads1115_to_volts(pga: u8, raw: i16) -> f64 {
    f64::from(ads1115::to_volts(ads1115::Pga::from_code(pga), raw))
}

impl From<Ads1115Config> for ads1115::Config {
    fn from(value: Ads1115Config) -> Self {
        ads1115::Config {
            start_conversion: value.start_conversion,
            mux: ads1115::Mux::from_code(value.mux),
            pga: ads1115::Pga::from_code(value.pga),
            mode: if value.single_shot {
                ads1115::Mode::SingleShot
            } else {
                ads1115::Mode::Continuous
            },
            data_rate: ads1115::DataRate::from_code(value.data_rate),
            comparator_mode: if value.window_comparator {
                ads1115::ComparatorMode::Window
            } else {
                ads1115::ComparatorMode::Traditional
            },
            comparator_polarity: if value.comparator_active_high {
                ads1115::ComparatorPolarity::ActiveHigh
            } else {
                ads1115::ComparatorPolarity::ActiveLow
            },
            comparator_latch: if value.comparator_latching {
                ads1115::ComparatorLatch::Latching
            } else {
                ads1115::ComparatorLatch::NonLatching
            },
            comparator_queue: ads1115::ComparatorQueue::from_code(value.comparator_queue),
        }
    }
}

impl From<ads1115::Config> for Ads1115Config {
    fn from(value: ads1115::Config) -> Self {
        Ads1115Config {
            start_conversion: value.start_conversion,
            mux: value.mux.code(),
            pga: value.pga.code(),
            single_shot: matches!(value.mode, ads1115::Mode::SingleShot),
            data_rate: value.data_rate.code(),
            window_comparator: matches!(value.comparator_mode, ads1115::ComparatorMode::Window),
            comparator_active_high: matches!(
                value.comparator_polarity,
                ads1115::ComparatorPolarity::ActiveHigh
            ),
            comparator_latching: matches!(
                value.comparator_latch,
                ads1115::ComparatorLatch::Latching
            ),
            comparator_queue: value.comparator_queue.code(),
        }
    }
}

/// Maps a bit count onto the resolution it names.
fn resolution(bits: u8) -> napi::Result<ds18b20::Resolution> {
    match bits {
        9 => Ok(ds18b20::Resolution::Bits9),
        10 => Ok(ds18b20::Resolution::Bits10),
        11 => Ok(ds18b20::Resolution::Bits11),
        12 => Ok(ds18b20::Resolution::Bits12),
        _ => Err(napi::Error::from_reason(
            "DS18B20 resolution must be 9, 10, 11, or 12 bits",
        )),
    }
}

/// Reports a buffer of the wrong size as a thrown exception.
fn length_error(what: &str, expected: usize) -> napi::Error {
    napi::Error::from_reason(format!("{what} must be exactly {expected} bytes"))
}

/// Maps a sensor error onto a thrown exception.
fn to_napi(error: SensorError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
