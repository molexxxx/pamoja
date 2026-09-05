//! Generated Python bindings for the sensor drivers.
//!
//! These mirror the `pamoja-sensors` Rust API: the decode half of four common
//! parts, turning the register bytes a bus driver read into the physical reading
//! the datasheet says they mean.
//!
//! A BME280's calibration is read once at start-up and reused for every
//! measurement, so it is a class. Everything else is a plain function over the
//! bytes or the register value a caller already holds.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_sensors::{ads1115, bme280, ds18b20, ina219, SensorError};

use crate::PamojaError;

/// A compensated BME280 reading.
#[gen_stub_pyclass]
#[pyclass]
pub struct Bme280Measurement {
    /// The temperature in degrees Celsius.
    #[pyo3(get)]
    celsius: f32,
    /// The pressure in pascals.
    #[pyo3(get)]
    pascals: u32,
    /// The pressure in hectopascals, the unit a barometer is usually quoted in.
    #[pyo3(get)]
    hectopascals: f32,
    /// The relative humidity as a percentage.
    #[pyo3(get)]
    relative_humidity_percent: f32,
}

/// A decoded DS18B20 scratchpad.
#[gen_stub_pyclass]
#[pyclass]
pub struct Ds18b20Reading {
    /// The raw temperature register, 1/16 degree Celsius per count.
    #[pyo3(get)]
    raw_temperature: i16,
    /// The temperature in micro-degrees Celsius, exact in integer arithmetic.
    #[pyo3(get)]
    micro_celsius: i32,
    /// The temperature in degrees Celsius.
    #[pyo3(get)]
    celsius: f32,
    /// The high alarm threshold in whole degrees Celsius.
    #[pyo3(get)]
    alarm_high: i8,
    /// The low alarm threshold in whole degrees Celsius.
    #[pyo3(get)]
    alarm_low: i8,
    /// The configured resolution, as a number of bits: 9, 10, 11, or 12.
    #[pyo3(get)]
    resolution_bits: u8,
}

/// An ADS1115 configuration register, field by field.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Ads1115Config {
    /// Whether writing this starts a single conversion.
    #[pyo3(get, set)]
    start_conversion: bool,
    /// The input multiplexer code, `0..=7`.
    #[pyo3(get, set)]
    mux: u8,
    /// The gain code, `0..=7`, which sets the full-scale range.
    #[pyo3(get, set)]
    pga: u8,
    /// Whether to convert once per request and power down, rather than continuously.
    #[pyo3(get, set)]
    single_shot: bool,
    /// The data rate code, `0..=7`.
    #[pyo3(get, set)]
    data_rate: u8,
    /// Whether to use the window comparator rather than the traditional one.
    #[pyo3(get, set)]
    window_comparator: bool,
    /// Whether the ALERT/RDY pin is active high.
    #[pyo3(get, set)]
    comparator_active_high: bool,
    /// Whether the comparator latches until the conversion is read.
    #[pyo3(get, set)]
    comparator_latching: bool,
    /// The comparator queue code, `0..=3`, where `3` disables the comparator.
    #[pyo3(get, set)]
    comparator_queue: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl Ads1115Config {
    /// Builds a configuration, defaulting every field to the part's reset state.
    #[new]
    #[pyo3(signature = (
        start_conversion = true,
        mux = 0,
        pga = 2,
        single_shot = true,
        data_rate = 4,
        window_comparator = false,
        comparator_active_high = false,
        comparator_latching = false,
        comparator_queue = 3,
    ))]
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    fn new(
        start_conversion: bool,
        mux: u8,
        pga: u8,
        single_shot: bool,
        data_rate: u8,
        window_comparator: bool,
        comparator_active_high: bool,
        comparator_latching: bool,
        comparator_queue: u8,
    ) -> Self {
        Self {
            start_conversion,
            mux,
            pga,
            single_shot,
            data_rate,
            window_comparator,
            comparator_active_high,
            comparator_latching,
            comparator_queue,
        }
    }

    /// Reports whether two configurations select the same settings.
    fn __eq__(&self, other: &Ads1115Config) -> bool {
        ads1115::Config::from(self.clone()).bits() == ads1115::Config::from(other.clone()).bits()
    }
}

/// A BME280's factory calibration, read once and reused for every measurement.
#[gen_stub_pyclass]
#[pyclass]
pub struct Bme280Calibration {
    inner: bme280::Calibration,
}

#[gen_stub_pymethods]
#[pymethods]
impl Bme280Calibration {
    /// Builds a calibration from the bytes read out of the device's registers.
    #[new]
    fn new(temp_press: Vec<u8>, humidity: Vec<u8>) -> PyResult<Self> {
        let temp_press: [u8; 26] = temp_press
            .as_slice()
            .try_into()
            .map_err(|_| length_error("temperature and pressure calibration", 26))?;
        let humidity: [u8; 7] = humidity
            .as_slice()
            .try_into()
            .map_err(|_| length_error("humidity calibration", 7))?;
        Ok(Self {
            inner: bme280::Calibration::from_registers(&temp_press, &humidity),
        })
    }

    /// Turns an eight-byte burst read into a compensated reading.
    fn compensate(&self, measurement: Vec<u8>) -> PyResult<Bme280Measurement> {
        let registers: [u8; 8] = measurement
            .as_slice()
            .try_into()
            .map_err(|_| length_error("measurement", 8))?;
        let reading = self
            .inner
            .compensate(&bme280::RawMeasurement::from_registers(&registers));
        Ok(Bme280Measurement {
            celsius: reading.celsius(),
            pascals: reading.pascals(),
            hectopascals: reading.hectopascals(),
            relative_humidity_percent: reading.relative_humidity_percent(),
        })
    }
}

/// Parses and CRC-checks a nine-byte DS18B20 scratchpad.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ds18b20_parse_scratchpad(data: Vec<u8>) -> PyResult<Ds18b20Reading> {
    let scratchpad: [u8; 9] = data
        .as_slice()
        .try_into()
        .map_err(|_| length_error("scratchpad", 9))?;
    let reading = ds18b20::Scratchpad::parse(&scratchpad).map_err(to_py)?;
    Ok(Ds18b20Reading {
        raw_temperature: reading.raw_temperature(),
        micro_celsius: reading.temperature_micro_celsius(),
        celsius: reading.temperature_celsius(),
        alarm_high: reading.alarm_high(),
        alarm_low: reading.alarm_low(),
        resolution_bits: reading.resolution().bits(),
    })
}

/// Builds the nine bytes a DS18B20 in the given state puts on the bus, CRC last.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ds18b20_build_scratchpad(
    celsius: f32,
    bits: u8,
    alarm_high: i8,
    alarm_low: i8,
) -> PyResult<Vec<u8>> {
    let resolution = resolution(bits)?;
    let raw = ds18b20::temperature_from_celsius(celsius, resolution);
    let scratchpad = ds18b20::Scratchpad::new(raw, resolution, alarm_high, alarm_low);
    Ok(scratchpad.to_bytes().to_vec())
}

/// Computes the Maxim CRC-8 a 1-Wire device checks its own bytes with.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ds18b20_crc8(data: Vec<u8>) -> u8 {
    ds18b20::crc8(&data)
}

/// Converts a raw DS18B20 temperature register to micro-degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ds18b20_micro_celsius(raw: i16) -> i32 {
    ds18b20::temperature_to_micro_celsius(raw)
}

/// Converts a raw DS18B20 temperature register to degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ds18b20_celsius(raw: i16) -> f32 {
    ds18b20::temperature_to_celsius(raw)
}

/// Returns the configuration byte that selects a DS18B20 resolution.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ds18b20_config_byte(bits: u8) -> PyResult<u8> {
    Ok(resolution(bits)?.config_byte())
}

/// Returns the resolution a DS18B20 configuration byte selects, in bits.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ds18b20_resolution_bits(config_byte: u8) -> u8 {
    ds18b20::Resolution::from_config_byte(config_byte).bits()
}

/// Returns the temperature step a DS18B20 resolution resolves, in micro-degrees.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ds18b20_step_micro_celsius(bits: u8) -> PyResult<u32> {
    Ok(resolution(bits)?.step_micro_celsius())
}

/// Returns how long a DS18B20 conversion may take at a resolution, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ds18b20_max_conversion_micros(bits: u8) -> PyResult<u32> {
    Ok(resolution(bits)?.max_conversion_micros())
}

/// Computes the INA219 calibration register for a shunt and current resolution.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_calibration(current_lsb_microamps: u32, shunt_milliohms: u32) -> u16 {
    ina219::calibration(current_lsb_microamps, shunt_milliohms)
}

/// Returns the smallest current resolution that still covers an expected maximum.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_minimum_current_lsb_microamps(max_expected_microamps: u32) -> u32 {
    ina219::minimum_current_lsb_microamps(max_expected_microamps)
}

/// Builds the INA219 shunt-voltage register a monitor reports for a shunt voltage.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_shunt_register(microvolts: i32) -> i16 {
    ina219::shunt_register(microvolts)
}

/// Builds the INA219 bus-voltage register a monitor reports for a bus voltage.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_bus_register(millivolts: u32) -> u16 {
    ina219::bus_register(millivolts)
}

/// Builds the INA219 current register a monitor reports for a current.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_current_register(microamps: i32, current_lsb_microamps: u32) -> i16 {
    ina219::current_register(microamps, current_lsb_microamps)
}

/// Builds the INA219 power register a monitor reports for a power.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_power_register(microwatts: u32, current_lsb_microamps: u32) -> u16 {
    ina219::power_register(microwatts, current_lsb_microamps)
}

/// Converts a raw INA219 shunt-voltage register to microvolts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_shunt_microvolts(raw: i16) -> i32 {
    ina219::shunt_microvolts(raw)
}

/// Converts a raw INA219 bus-voltage register to millivolts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_bus_millivolts(raw: u16) -> u32 {
    ina219::bus_millivolts(raw)
}

/// Reports whether an INA219 bus-voltage register says a conversion is ready.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_conversion_ready(raw: u16) -> bool {
    ina219::conversion_ready(raw)
}

/// Reports whether an INA219 bus-voltage register flags a math overflow.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_math_overflow(raw: u16) -> bool {
    ina219::math_overflow(raw)
}

/// Converts a raw INA219 current register to microamps.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_current_microamps(raw: i16, current_lsb_microamps: u32) -> i32 {
    ina219::current_microamps(raw, current_lsb_microamps)
}

/// Converts a raw INA219 power register to microwatts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_power_microwatts(raw: u16, current_lsb_microamps: u32) -> u32 {
    ina219::power_microwatts(raw, current_lsb_microamps)
}

/// Assembles the 16-bit ADS1115 configuration register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ads1115_config_bits(config: Ads1115Config) -> u16 {
    ads1115::Config::from(config).bits()
}

/// Parses a 16-bit ADS1115 configuration register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ads1115_config_from_bits(bits: u16) -> Ads1115Config {
    ads1115::Config::from_bits(bits).into()
}

/// Returns the full-scale range an ADS1115 gain code selects, in microvolts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ads1115_full_scale_microvolts(pga: u8) -> u32 {
    ads1115::Pga::from_code(pga).full_scale_microvolts()
}

/// Returns the sample rate an ADS1115 data-rate code selects.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ads1115_samples_per_second(data_rate: u8) -> u16 {
    ads1115::DataRate::from_code(data_rate).samples_per_second()
}

/// Converts a raw ADS1115 conversion result to nanovolts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ads1115_to_nanovolts(pga: u8, raw: i16) -> i64 {
    ads1115::to_nanovolts(ads1115::Pga::from_code(pga), raw)
}

/// Converts a raw ADS1115 conversion result to volts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ads1115_to_volts(pga: u8, raw: i16) -> f32 {
    ads1115::to_volts(ads1115::Pga::from_code(pga), raw)
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
fn resolution(bits: u8) -> PyResult<ds18b20::Resolution> {
    match bits {
        9 => Ok(ds18b20::Resolution::Bits9),
        10 => Ok(ds18b20::Resolution::Bits10),
        11 => Ok(ds18b20::Resolution::Bits11),
        12 => Ok(ds18b20::Resolution::Bits12),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "DS18B20 resolution must be 9, 10, 11, or 12 bits",
        )),
    }
}

/// Reports a buffer of the wrong size as a raised exception.
fn length_error(what: &str, expected: usize) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(format!("{what} must be exactly {expected} bytes"))
}

/// Maps a sensor error onto the SDK's Python exception.
fn to_py(error: SensorError) -> PyErr {
    PamojaError::new_err(error.to_string())
}
