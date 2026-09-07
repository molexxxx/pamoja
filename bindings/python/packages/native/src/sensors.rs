//! Generated Python bindings for the sensor drivers.
//!
//! These mirror the `pamoja-sensors` Rust API: the decode half of eleven common
//! parts, turning the register bytes a bus driver read into the physical reading
//! the datasheet says they mean.
//!
//! A BME280 or BMP280 calibration is read once at start-up and reused for every
//! measurement, so each is a class. Everything else is a plain function over the
//! bytes or the register value a caller already holds.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_sensors::{
    ads1115, bme280, bmp280, ds18b20, hdc1080, ina219, ina226, opt3001, scd4x, sht3x, tmp117,
    SensorError,
};

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

/// A BMP280's per-chip trimming coefficients, as they sit in its registers.
#[gen_stub_pyclass]
#[pyclass]
pub struct Bmp280Coefficients {
    /// The `dig_T1` coefficient.
    #[pyo3(get)]
    dig_t1: u16,
    /// The `dig_T2` coefficient.
    #[pyo3(get)]
    dig_t2: i16,
    /// The `dig_T3` coefficient.
    #[pyo3(get)]
    dig_t3: i16,
    /// The `dig_P1` coefficient.
    #[pyo3(get)]
    dig_p1: u16,
    /// The `dig_P2` coefficient.
    #[pyo3(get)]
    dig_p2: i16,
    /// The `dig_P3` coefficient.
    #[pyo3(get)]
    dig_p3: i16,
    /// The `dig_P4` coefficient.
    #[pyo3(get)]
    dig_p4: i16,
    /// The `dig_P5` coefficient.
    #[pyo3(get)]
    dig_p5: i16,
    /// The `dig_P6` coefficient.
    #[pyo3(get)]
    dig_p6: i16,
    /// The `dig_P7` coefficient.
    #[pyo3(get)]
    dig_p7: i16,
    /// The `dig_P8` coefficient.
    #[pyo3(get)]
    dig_p8: i16,
    /// The `dig_P9` coefficient.
    #[pyo3(get)]
    dig_p9: i16,
}

/// A compensated BMP280 reading.
#[gen_stub_pyclass]
#[pyclass]
pub struct Bmp280Reading {
    /// The temperature in degrees Celsius.
    #[pyo3(get)]
    celsius: f32,
    /// The pressure in pascals.
    #[pyo3(get)]
    pascals: u32,
    /// The pressure in hectopascals, the unit a barometer is usually quoted in.
    #[pyo3(get)]
    hectopascals: f32,
}

/// The uncompensated codes a BMP280 burst read carries.
#[gen_stub_pyclass]
#[pyclass]
pub struct Bmp280RawMeasurement {
    /// The 20-bit pressure code.
    #[pyo3(get)]
    pressure: u32,
    /// The 20-bit temperature code.
    #[pyo3(get)]
    temperature: u32,
    /// Whether pressure oversampling was off, so the code carries no reading.
    #[pyo3(get)]
    pressure_skipped: bool,
    /// Whether temperature oversampling was off, so the code carries no reading.
    #[pyo3(get)]
    temperature_skipped: bool,
}

/// A BMP280 `ctrl_meas` register, field by field.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, PartialEq, Eq)]
pub struct Bmp280CtrlMeas {
    /// The temperature oversampling code, `0..=5`, where `0` skips the measurement.
    #[pyo3(get, set)]
    temperature: u8,
    /// The pressure oversampling code, `0..=5`, where `0` skips the measurement.
    #[pyo3(get, set)]
    pressure: u8,
    /// The power mode code: `0` sleep, `1` forced, `3` normal.
    #[pyo3(get, set)]
    mode: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl Bmp280CtrlMeas {
    /// Builds a control register, defaulting every field to the part's reset state.
    #[new]
    #[pyo3(signature = (temperature = 0, pressure = 0, mode = 0))]
    fn new(temperature: u8, pressure: u8, mode: u8) -> Self {
        Self {
            temperature,
            pressure,
            mode,
        }
    }

    /// Reports whether two control registers select the same settings.
    fn __eq__(&self, other: &Bmp280CtrlMeas) -> bool {
        self == other
    }
}

/// A BMP280 `config` register, field by field.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, PartialEq, Eq)]
pub struct Bmp280Config {
    /// The normal-mode standby code, `0..=7`.
    #[pyo3(get, set)]
    standby: u8,
    /// The IIR filter code, `0..=7`.
    #[pyo3(get, set)]
    filter: u8,
    /// Whether the 3-wire SPI interface is enabled.
    #[pyo3(get, set)]
    spi_3wire: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl Bmp280Config {
    /// Builds a configuration, defaulting every field to the part's reset state.
    #[new]
    #[pyo3(signature = (standby = 0, filter = 0, spi_3wire = false))]
    fn new(standby: u8, filter: u8, spi_3wire: bool) -> Self {
        Self {
            standby,
            filter,
            spi_3wire,
        }
    }

    /// Reports whether two configurations select the same settings.
    fn __eq__(&self, other: &Bmp280Config) -> bool {
        self == other
    }
}

/// A BMP280's factory calibration, read once and reused for every measurement.
#[gen_stub_pyclass]
#[pyclass]
pub struct Bmp280Calibration {
    inner: bmp280::Calibration,
}

#[gen_stub_pymethods]
#[pymethods]
impl Bmp280Calibration {
    /// Builds a calibration from the 24 bytes read out of the device's registers.
    #[new]
    fn new(data: Vec<u8>) -> PyResult<Self> {
        let registers: [u8; bmp280::CALIBRATION_LEN] = data
            .as_slice()
            .try_into()
            .map_err(|_| length_error("calibration", bmp280::CALIBRATION_LEN))?;
        Ok(Self {
            inner: bmp280::Calibration::parse(&registers),
        })
    }

    /// Turns a six-byte burst read into a compensated reading.
    fn compensate(&self, measurement: Vec<u8>) -> PyResult<Bmp280Reading> {
        let registers: [u8; bmp280::DATA_LEN] = measurement
            .as_slice()
            .try_into()
            .map_err(|_| length_error("measurement", bmp280::DATA_LEN))?;
        Ok(self
            .inner
            .compensate(&bmp280::Measurement::parse(&registers))
            .into())
    }

    /// Rebuilds the 24 calibration bytes a device holding these coefficients returns.
    fn to_bytes(&self) -> Vec<u8> {
        self.inner.to_bytes().to_vec()
    }

    /// The trimming coefficients the calibration bytes carried.
    #[getter]
    fn coefficients(&self) -> Bmp280Coefficients {
        self.inner.into()
    }
}

/// Unpacks the six data bytes a BMP280 burst read returns.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_parse_measurement(data: Vec<u8>) -> PyResult<Bmp280RawMeasurement> {
    let registers: [u8; bmp280::DATA_LEN] = data
        .as_slice()
        .try_into()
        .map_err(|_| length_error("measurement", bmp280::DATA_LEN))?;
    Ok(bmp280::Measurement::parse(&registers).into())
}

/// Builds the six data bytes a BMP280 holding these codes would return.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_measurement_bytes(pressure: u32, temperature: u32) -> Vec<u8> {
    let measurement = bmp280::Measurement {
        pressure,
        temperature,
    };
    measurement.to_bytes().to_vec()
}

/// Reports whether a raw BMP280 pressure says the measurement is switched off.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_pressure_skipped(pressure: u32) -> bool {
    bmp280::Measurement {
        pressure,
        temperature: 0,
    }
    .pressure_skipped()
}

/// Reports whether a raw BMP280 temperature says the measurement is switched off.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_temperature_skipped(temperature: u32) -> bool {
    bmp280::Measurement {
        pressure: 0,
        temperature,
    }
    .temperature_skipped()
}

/// Reports whether a BMP280 status byte says a conversion is running.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_measuring(status: u8) -> bool {
    bmp280::measuring(status)
}

/// Reports whether a BMP280 status byte says the calibration image is loading.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_image_updating(status: u8) -> bool {
    bmp280::image_updating(status)
}

/// Packs a BMP280 `ctrl_meas` register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_ctrl_meas_bits(ctrl: Bmp280CtrlMeas) -> u8 {
    bmp280::CtrlMeas::from(ctrl).bits()
}

/// Parses a BMP280 `ctrl_meas` register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_ctrl_meas_from_bits(bits: u8) -> Bmp280CtrlMeas {
    bmp280::CtrlMeas::from_bits(bits).into()
}

/// Packs a BMP280 `config` register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_config_bits(config: Bmp280Config) -> u8 {
    bmp280::Config::from(config).bits()
}

/// Parses a BMP280 `config` register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_config_from_bits(bits: u8) -> Bmp280Config {
    bmp280::Config::from_bits(bits).into()
}

/// Returns how many samples a BMP280 oversampling code averages.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_oversampling_factor(code: u8) -> u8 {
    bmp280::Oversampling::from_code(code).factor()
}

/// Returns the normal-mode standby period a BMP280 code selects, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_standby_micros(code: u8) -> u32 {
    bmp280::Standby::from_code(code).microseconds()
}

/// A decoded SHT3x temperature and humidity pair.
#[gen_stub_pyclass]
#[pyclass]
pub struct Sht3xMeasurement {
    /// The raw temperature word.
    #[pyo3(get)]
    temperature_raw: u16,
    /// The raw humidity word.
    #[pyo3(get)]
    humidity_raw: u16,
    /// The temperature in milli-degrees Celsius, exact in integer arithmetic.
    #[pyo3(get)]
    milli_celsius: i32,
    /// The temperature in degrees Celsius.
    #[pyo3(get)]
    celsius: f32,
    /// The temperature in milli-degrees Fahrenheit.
    #[pyo3(get)]
    milli_fahrenheit: i32,
    /// The temperature in degrees Fahrenheit.
    #[pyo3(get)]
    fahrenheit: f32,
    /// The relative humidity in milli-percent.
    #[pyo3(get)]
    milli_percent: u32,
    /// The relative humidity as a percentage.
    #[pyo3(get)]
    relative_humidity: f32,
}

/// A decoded SHT3x status register.
#[gen_stub_pyclass]
#[pyclass]
pub struct Sht3xStatus {
    /// The 16-bit status word the flags were read from.
    #[pyo3(get)]
    bits: u16,
    /// Whether at least one alert condition is pending.
    #[pyo3(get)]
    alert_pending: bool,
    /// Whether the on-die heater is running.
    #[pyo3(get)]
    heater_on: bool,
    /// Whether a humidity tracking alert is set.
    #[pyo3(get)]
    humidity_tracking_alert: bool,
    /// Whether a temperature tracking alert is set.
    #[pyo3(get)]
    temperature_tracking_alert: bool,
    /// Whether the part has reset since the flag was last cleared.
    #[pyo3(get)]
    reset_detected: bool,
    /// Whether the last command could not be processed.
    #[pyo3(get)]
    command_failed: bool,
    /// Whether the last write failed its checksum.
    #[pyo3(get)]
    write_checksum_failed: bool,
}

/// Computes the CRC-8 an SHT3x appends to every data word.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_crc(data: Vec<u8>) -> u8 {
    sht3x::crc(&data)
}

/// Reads a CRC-checked three-byte SHT3x word frame.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_word(frame: Vec<u8>) -> PyResult<u16> {
    let frame: [u8; 3] = frame
        .as_slice()
        .try_into()
        .map_err(|_| length_error("word frame", 3))?;
    sht3x::word(&frame).map_err(to_py)
}

/// Builds the three bytes an SHT3x sends for a word: the word then its CRC.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_word_bytes(value: u16) -> Vec<u8> {
    sht3x::word_bytes(value).to_vec()
}

/// Parses and CRC-checks a six-byte SHT3x measurement frame.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_parse_measurement(frame: Vec<u8>) -> PyResult<Sht3xMeasurement> {
    let frame: [u8; 6] = frame
        .as_slice()
        .try_into()
        .map_err(|_| length_error("measurement frame", 6))?;
    Ok(sht3x::Measurement::parse(&frame).map_err(to_py)?.into())
}

/// Builds the six bytes an SHT3x sends for a pair of raw words.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_measurement_bytes(temperature_raw: u16, humidity_raw: u16) -> Vec<u8> {
    let measurement = sht3x::Measurement {
        temperature_raw,
        humidity_raw,
    };
    measurement.to_bytes().to_vec()
}

/// Converts a raw SHT3x temperature word to milli-degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_milli_celsius(raw: u16) -> i32 {
    sht3x::milli_celsius(raw)
}

/// Converts a raw SHT3x temperature word to degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_celsius(raw: u16) -> f32 {
    sht3x::celsius(raw)
}

/// Converts a raw SHT3x temperature word to milli-degrees Fahrenheit.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_milli_fahrenheit(raw: u16) -> i32 {
    sht3x::milli_fahrenheit(raw)
}

/// Converts a raw SHT3x temperature word to degrees Fahrenheit.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_fahrenheit(raw: u16) -> f32 {
    sht3x::fahrenheit(raw)
}

/// Converts a raw SHT3x humidity word to milli-percent.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_milli_percent(raw: u16) -> u32 {
    sht3x::milli_percent(raw)
}

/// Converts a raw SHT3x humidity word to a relative humidity percentage.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_relative_humidity(raw: u16) -> f32 {
    sht3x::relative_humidity(raw)
}

/// Builds the SHT3x temperature word that decodes to a temperature.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_temperature_raw_from_milli_celsius(milli_celsius: i32) -> u16 {
    sht3x::temperature_raw_from_milli_celsius(milli_celsius)
}

/// Builds the SHT3x temperature word that decodes to a temperature in Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_temperature_raw_from_celsius(celsius: f32) -> u16 {
    sht3x::temperature_raw_from_celsius(celsius)
}

/// Builds the SHT3x temperature word that decodes to a temperature in Fahrenheit.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_temperature_raw_from_milli_fahrenheit(milli_fahrenheit: i32) -> u16 {
    sht3x::temperature_raw_from_milli_fahrenheit(milli_fahrenheit)
}

/// Builds the SHT3x humidity word that decodes to a relative humidity.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_humidity_raw_from_milli_percent(milli_percent: u32) -> u16 {
    sht3x::humidity_raw_from_milli_percent(milli_percent)
}

/// Builds the SHT3x humidity word that decodes to a relative humidity percentage.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_humidity_raw_from_relative_humidity(percent: f32) -> u16 {
    sht3x::humidity_raw_from_relative_humidity(percent)
}

/// Parses and CRC-checks a three-byte SHT3x status frame.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_parse_status(frame: Vec<u8>) -> PyResult<Sht3xStatus> {
    let frame: [u8; 3] = frame
        .as_slice()
        .try_into()
        .map_err(|_| length_error("status frame", 3))?;
    Ok(sht3x::Status::parse(&frame).map_err(to_py)?.into())
}

/// Splits an SHT3x status word into its flags.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_status_from_bits(bits: u16) -> Sht3xStatus {
    sht3x::Status::from_bits(bits).into()
}

/// Builds the three bytes an SHT3x sends for a status word, CRC last.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_status_bytes(bits: u16) -> Vec<u8> {
    sht3x::Status::from_bits(bits).to_bytes().to_vec()
}

/// Returns the SHT3x single-shot command for a repeatability and clock mode.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_single_shot(repeatability: &str, clock_stretching: bool) -> PyResult<u16> {
    Ok(sht3x::single_shot(
        read_repeatability(repeatability)?,
        clock_stretching,
    ))
}

/// Returns the SHT3x periodic-mode command for a repeatability and rate.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_periodic(repeatability: &str, rate: &str) -> PyResult<u16> {
    Ok(sht3x::periodic(
        read_repeatability(repeatability)?,
        read_rate(rate)?,
    ))
}

/// Returns how long an SHT3x measurement may take, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_max_measurement_micros(repeatability: &str) -> PyResult<u32> {
    Ok(read_repeatability(repeatability)?.max_measurement_micros())
}

/// Returns how long an SHT3x measurement typically takes, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_typical_measurement_micros(repeatability: &str) -> PyResult<u32> {
    Ok(read_repeatability(repeatability)?.typical_measurement_micros())
}

/// Returns the gap between SHT3x periodic measurements, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_interval_micros(rate: &str) -> PyResult<u32> {
    Ok(read_rate(rate)?.interval_micros())
}

/// A decoded SCD4x measurement frame.
#[gen_stub_pyclass]
#[pyclass]
pub struct Scd4xMeasurement {
    /// The carbon dioxide concentration in parts per million.
    #[pyo3(get)]
    co2_ppm: u16,
    /// The raw temperature word.
    #[pyo3(get)]
    temperature_raw: u16,
    /// The raw humidity word.
    #[pyo3(get)]
    humidity_raw: u16,
    /// The temperature in milli-degrees Celsius, exact in integer arithmetic.
    #[pyo3(get)]
    milli_celsius: i32,
    /// The temperature in degrees Celsius.
    #[pyo3(get)]
    celsius: f32,
    /// The relative humidity in milli-percent.
    #[pyo3(get)]
    humidity_milli_percent: u32,
    /// The relative humidity as a percentage.
    #[pyo3(get)]
    relative_humidity_percent: f32,
}

/// Computes the CRC-8 an SCD4x appends to every data word.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_crc(data: Vec<u8>) -> u8 {
    scd4x::crc(&data)
}

/// Reads a CRC-checked three-byte SCD4x word frame.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_word(frame: Vec<u8>) -> PyResult<u16> {
    let frame: [u8; 3] = frame
        .as_slice()
        .try_into()
        .map_err(|_| length_error("word frame", 3))?;
    scd4x::word(&frame).map_err(to_py)
}

/// Builds the three bytes an SCD4x sends for a word: the word then its CRC.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_word_frame(value: u16) -> Vec<u8> {
    scd4x::word_frame(value).to_vec()
}

/// Builds the two bytes that send a bare SCD4x command.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_command_frame(command: u16) -> Vec<u8> {
    scd4x::command_frame(command).to_vec()
}

/// Builds the five bytes that send an SCD4x command with an argument.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_write_frame(command: u16, value: u16) -> Vec<u8> {
    scd4x::write_frame(command, value).to_vec()
}

/// Returns how long an SCD4x command may take, in milliseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_max_duration_ms(command: u16) -> Option<u16> {
    scd4x::max_duration_ms(command)
}

/// Reports whether an SCD4x accepts a command while it is measuring.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_allowed_during_measurement(command: u16) -> bool {
    scd4x::allowed_during_measurement(command)
}

/// Parses and CRC-checks a nine-byte SCD4x measurement frame.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_parse_measurement(frame: Vec<u8>) -> PyResult<Scd4xMeasurement> {
    let frame: [u8; 9] = frame
        .as_slice()
        .try_into()
        .map_err(|_| length_error("measurement frame", 9))?;
    Ok(scd4x::Measurement::parse(&frame).map_err(to_py)?.into())
}

/// Builds the SCD4x measurement a sensor reporting these physical values would send.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_measurement_from_physical(
    co2_ppm: u16,
    milli_celsius: i32,
    humidity_milli_percent: u32,
) -> Scd4xMeasurement {
    scd4x::Measurement::from_physical(co2_ppm, milli_celsius, humidity_milli_percent).into()
}

/// Builds the nine bytes an SCD4x sends for a set of raw words.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_measurement_bytes(co2_ppm: u16, temperature_raw: u16, humidity_raw: u16) -> Vec<u8> {
    let measurement = scd4x::Measurement {
        co2_ppm,
        temperature_raw,
        humidity_raw,
    };
    measurement.to_bytes().to_vec()
}

/// Converts a raw SCD4x temperature word to milli-degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_milli_celsius(raw: u16) -> i32 {
    scd4x::milli_celsius(raw)
}

/// Converts a raw SCD4x temperature word to degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_celsius(raw: u16) -> f32 {
    scd4x::celsius(raw)
}

/// Builds the SCD4x temperature word that decodes to a temperature.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_temperature_raw(milli_celsius: i32) -> u16 {
    scd4x::temperature_raw(milli_celsius)
}

/// Converts a raw SCD4x humidity word to milli-percent.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_humidity_milli_percent(raw: u16) -> u32 {
    scd4x::humidity_milli_percent(raw)
}

/// Converts a raw SCD4x humidity word to a relative humidity percentage.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_relative_humidity_percent(raw: u16) -> f32 {
    scd4x::relative_humidity_percent(raw)
}

/// Builds the SCD4x humidity word that decodes to a relative humidity.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_humidity_raw(milli_percent: u32) -> u16 {
    scd4x::humidity_raw(milli_percent)
}

/// Reports whether an SCD4x data-ready word says a measurement is waiting.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_data_ready(word: u16) -> bool {
    scd4x::data_ready(word)
}

/// Builds the SCD4x word that programs a temperature offset.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_temperature_offset_word(milli_celsius: u32) -> u16 {
    scd4x::temperature_offset_word(milli_celsius)
}

/// Converts an SCD4x temperature-offset word back to milli-degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_temperature_offset_milli_celsius(word: u16) -> u32 {
    scd4x::temperature_offset_milli_celsius(word)
}

/// Builds the SCD4x word that programs an ambient pressure.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_ambient_pressure_word(pascals: u32) -> u16 {
    scd4x::ambient_pressure_word(pascals)
}

/// Converts an SCD4x ambient-pressure word back to pascals.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_ambient_pressure_pascals(word: u16) -> u32 {
    scd4x::ambient_pressure_pascals(word)
}

/// Reads the correction an SCD4x reports after a forced recalibration.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_forced_recalibration_correction_ppm(word: u16) -> Option<i32> {
    scd4x::forced_recalibration_correction_ppm(word)
}

/// Builds the word an SCD4x returns for a forced-recalibration outcome.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (correction_ppm = None))]
pub fn scd4x_forced_recalibration_word(correction_ppm: Option<i32>) -> u16 {
    scd4x::forced_recalibration_word(correction_ppm)
}

/// Reports whether an SCD4x word says automatic self-calibration is on.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_automatic_self_calibration_enabled(word: u16) -> bool {
    scd4x::automatic_self_calibration_enabled(word)
}

/// Builds the SCD4x word that turns automatic self-calibration on or off.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_automatic_self_calibration_word(enabled: bool) -> u16 {
    scd4x::automatic_self_calibration_word(enabled)
}

/// Reports whether an SCD4x self-test word says the part passed.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_self_test_passed(word: u16) -> bool {
    scd4x::self_test_passed(word)
}

/// Reads a CRC-checked nine-byte SCD4x serial-number frame.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_serial_number(frame: Vec<u8>) -> PyResult<u64> {
    let frame: [u8; 9] = frame
        .as_slice()
        .try_into()
        .map_err(|_| length_error("serial number frame", 9))?;
    scd4x::serial_number(&frame).map_err(to_py)
}

/// Builds the nine bytes an SCD4x sends for a serial number.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_serial_number_frame(serial: u64) -> Vec<u8> {
    scd4x::serial_number_frame(serial).to_vec()
}

/// A TMP117 configuration register, field by field.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, PartialEq, Eq)]
pub struct Tmp117Config {
    /// Whether a result went above the high limit.
    #[pyo3(get, set)]
    high_alert: bool,
    /// Whether a result went below the low limit.
    #[pyo3(get, set)]
    low_alert: bool,
    /// Whether a conversion has completed since the register was last read.
    #[pyo3(get, set)]
    data_ready: bool,
    /// Whether an EEPROM write is still in progress.
    #[pyo3(get, set)]
    eeprom_busy: bool,
    /// The conversion-mode code: `0` continuous, `1` shutdown, `3` one-shot.
    #[pyo3(get, set)]
    mode: u8,
    /// The conversion-cycle code, `0..=7`.
    #[pyo3(get, set)]
    cycle: u8,
    /// The averaging code, `0..=3`.
    #[pyo3(get, set)]
    averaging: u8,
    /// Whether the limits act as a therm hysteresis band rather than as alerts.
    #[pyo3(get, set)]
    therm_mode: bool,
    /// Whether the ALERT pin is active high.
    #[pyo3(get, set)]
    alert_active_high: bool,
    /// Whether the ALERT pin reflects data ready rather than the alert flags.
    #[pyo3(get, set)]
    alert_pin_data_ready: bool,
    /// Whether writing this triggers a software reset.
    #[pyo3(get, set)]
    soft_reset: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl Tmp117Config {
    /// Builds a configuration, defaulting every field to the part's reset state.
    #[new]
    #[pyo3(signature = (
        high_alert = false,
        low_alert = false,
        data_ready = false,
        eeprom_busy = false,
        mode = 0,
        cycle = 4,
        averaging = 1,
        therm_mode = false,
        alert_active_high = false,
        alert_pin_data_ready = false,
        soft_reset = false,
    ))]
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    fn new(
        high_alert: bool,
        low_alert: bool,
        data_ready: bool,
        eeprom_busy: bool,
        mode: u8,
        cycle: u8,
        averaging: u8,
        therm_mode: bool,
        alert_active_high: bool,
        alert_pin_data_ready: bool,
        soft_reset: bool,
    ) -> Self {
        Self {
            high_alert,
            low_alert,
            data_ready,
            eeprom_busy,
            mode,
            cycle,
            averaging,
            therm_mode,
            alert_active_high,
            alert_pin_data_ready,
            soft_reset,
        }
    }

    /// Reports whether two configurations select the same settings.
    fn __eq__(&self, other: &Tmp117Config) -> bool {
        self == other
    }
}

/// Converts a raw TMP117 temperature register to nano-degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_nano_celsius(raw: i16) -> i64 {
    tmp117::nano_celsius(raw)
}

/// Converts a raw TMP117 temperature register to micro-degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_micro_celsius(raw: i16) -> i32 {
    tmp117::micro_celsius(raw)
}

/// Converts a raw TMP117 temperature register to degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_celsius(raw: i16) -> f32 {
    tmp117::celsius(raw)
}

/// Builds the TMP117 temperature register that decodes to a temperature.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_raw_from_micro_celsius(micro_celsius: i32) -> i16 {
    tmp117::raw_from_micro_celsius(micro_celsius)
}

/// Builds the TMP117 temperature register that decodes to a temperature in Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_raw_from_celsius(celsius: f32) -> i16 {
    tmp117::raw_from_celsius(celsius)
}

/// Builds the two bytes a TMP117 sends for a temperature register.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_temperature_bytes(raw: i16) -> Vec<u8> {
    tmp117::temperature_bytes(raw).to_vec()
}

/// Reads the two bytes a TMP117 sends for a temperature register.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_temperature_from_bytes(data: Vec<u8>) -> PyResult<i16> {
    let bytes: [u8; 2] = data
        .as_slice()
        .try_into()
        .map_err(|_| length_error("temperature register", 2))?;
    Ok(tmp117::temperature_from_bytes(bytes))
}

/// Reads the device identifier out of a TMP117 device-ID register.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_device_id(raw: u16) -> u16 {
    tmp117::device_id(raw)
}

/// Reads the die revision out of a TMP117 device-ID register.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_revision(raw: u16) -> u8 {
    tmp117::revision(raw)
}

/// Reports whether a TMP117 configuration register flags a high alert.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_high_alert(config: u16) -> bool {
    tmp117::high_alert(config)
}

/// Reports whether a TMP117 configuration register flags a low alert.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_low_alert(config: u16) -> bool {
    tmp117::low_alert(config)
}

/// Reports whether a TMP117 configuration register says a result is ready.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_data_ready(config: u16) -> bool {
    tmp117::data_ready(config)
}

/// Reports whether a TMP117 configuration register says an EEPROM write is running.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_eeprom_busy(config: u16) -> bool {
    tmp117::eeprom_busy(config)
}

/// Reports whether a TMP117 EEPROM unlock register says a write is running.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_eeprom_unlock_busy(unlock: u16) -> bool {
    tmp117::eeprom_unlock_busy(unlock)
}

/// Assembles the 16-bit TMP117 configuration register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_config_bits(config: Tmp117Config) -> u16 {
    tmp117::Configuration::from(config).bits()
}

/// Parses a 16-bit TMP117 configuration register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_config_from_bits(bits: u16) -> Tmp117Config {
    tmp117::Configuration::from_bits(bits).into()
}

/// Returns how many conversions a TMP117 averaging code folds into one result.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_averaging_conversions(code: u8) -> u8 {
    tmp117::Averaging::from_code(code).conversions()
}

/// Returns how long a TMP117 averaging code takes to convert, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_averaging_micros(code: u8) -> u32 {
    tmp117::Averaging::from_code(code).conversion_micros()
}

/// Returns the nominal cycle a TMP117 conversion-cycle code selects, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_cycle_nominal_micros(code: u8) -> u32 {
    tmp117::ConversionCycle::from_code(code).nominal_micros()
}

/// Returns the TMP117 result-update interval for a cycle and averaging code.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_cycle_micros(cycle: u8, averaging: u8) -> u32 {
    tmp117::ConversionCycle::from_code(cycle).cycle_micros(tmp117::Averaging::from_code(averaging))
}

/// A decoded HDC1080 temperature and humidity pair.
#[gen_stub_pyclass]
#[pyclass]
pub struct Hdc1080Measurement {
    /// The raw temperature register.
    #[pyo3(get)]
    temperature_raw: u16,
    /// The raw humidity register.
    #[pyo3(get)]
    humidity_raw: u16,
    /// The temperature in milli-degrees Celsius, exact in integer arithmetic.
    #[pyo3(get)]
    milli_celsius: i32,
    /// The temperature in degrees Celsius.
    #[pyo3(get)]
    celsius: f32,
    /// The relative humidity in milli-percent.
    #[pyo3(get)]
    milli_percent: u32,
    /// The relative humidity as a percentage.
    #[pyo3(get)]
    relative_humidity: f32,
}

/// An HDC1080 configuration register, field by field.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, PartialEq, Eq)]
pub struct Hdc1080Config {
    /// Whether writing this resets the part.
    #[pyo3(get, set)]
    software_reset: bool,
    /// Whether the on-die heater runs during measurements.
    #[pyo3(get, set)]
    heater: bool,
    /// Whether one trigger acquires temperature and humidity in sequence.
    #[pyo3(get, set)]
    sequential: bool,
    /// Whether the supply has dropped below 2.8 V, which the part reports back.
    #[pyo3(get, set)]
    battery_low: bool,
    /// The temperature resolution in bits: 14 or 11.
    #[pyo3(get, set)]
    temperature_resolution_bits: u8,
    /// The humidity resolution in bits: 14, 11, or 8.
    #[pyo3(get, set)]
    humidity_resolution_bits: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl Hdc1080Config {
    /// Builds a configuration, defaulting every field to the part's reset state.
    #[new]
    #[pyo3(signature = (
        software_reset = false,
        heater = false,
        sequential = true,
        battery_low = false,
        temperature_resolution_bits = 14,
        humidity_resolution_bits = 14,
    ))]
    fn new(
        software_reset: bool,
        heater: bool,
        sequential: bool,
        battery_low: bool,
        temperature_resolution_bits: u8,
        humidity_resolution_bits: u8,
    ) -> Self {
        Self {
            software_reset,
            heater,
            sequential,
            battery_low,
            temperature_resolution_bits,
            humidity_resolution_bits,
        }
    }

    /// Reports whether two configurations select the same settings.
    fn __eq__(&self, other: &Hdc1080Config) -> bool {
        self == other
    }
}

/// Converts a raw HDC1080 temperature register to milli-degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_milli_celsius(raw: u16) -> i32 {
    hdc1080::milli_celsius(raw)
}

/// Converts a raw HDC1080 temperature register to degrees Celsius.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_celsius(raw: u16) -> f32 {
    hdc1080::celsius(raw)
}

/// Converts a raw HDC1080 humidity register to milli-percent.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_milli_percent(raw: u16) -> u32 {
    hdc1080::milli_percent(raw)
}

/// Converts a raw HDC1080 humidity register to a relative humidity percentage.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_relative_humidity(raw: u16) -> f32 {
    hdc1080::relative_humidity(raw)
}

/// Builds the HDC1080 temperature register that decodes to a temperature.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_temperature_register(milli_celsius: i32) -> u16 {
    hdc1080::temperature_register(milli_celsius)
}

/// Builds the HDC1080 humidity register that decodes to a relative humidity.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_humidity_register(milli_percent: u32) -> u16 {
    hdc1080::humidity_register(milli_percent)
}

/// Joins the three HDC1080 serial-ID registers into the 40-bit serial number.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_serial_id(high: u16, mid: u16, low: u16) -> u64 {
    hdc1080::serial_id(high, mid, low)
}

/// Splits a serial number back into the three HDC1080 serial-ID registers.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_serial_id_registers(serial: u64) -> Vec<u16> {
    hdc1080::serial_id_registers(serial).to_vec()
}

/// Parses the four bytes an HDC1080 sequential read returns.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_parse_measurement(data: Vec<u8>) -> PyResult<Hdc1080Measurement> {
    let bytes: [u8; 4] = data
        .as_slice()
        .try_into()
        .map_err(|_| length_error("measurement", 4))?;
    Ok(hdc1080::Measurement::parse(&bytes).into())
}

/// Builds the HDC1080 measurement a sensor reporting these physical values would send.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_measurement_from_physical(
    milli_celsius: i32,
    milli_percent: u32,
) -> Hdc1080Measurement {
    hdc1080::Measurement::from_physical(milli_celsius, milli_percent).into()
}

/// Builds the four bytes an HDC1080 sends for a pair of raw registers.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_measurement_bytes(temperature_raw: u16, humidity_raw: u16) -> Vec<u8> {
    let measurement = hdc1080::Measurement {
        temperature: temperature_raw,
        humidity: humidity_raw,
    };
    measurement.to_bytes().to_vec()
}

/// Parses an HDC1080 configuration register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_config_from_register(raw: u16) -> PyResult<Hdc1080Config> {
    Ok(hdc1080::Configuration::from_register(raw)
        .map_err(to_py)?
        .into())
}

/// Assembles an HDC1080 configuration register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_config_to_register(config: Hdc1080Config) -> PyResult<u16> {
    Ok(hdc1080::Configuration::try_from(config)?.to_register())
}

/// Returns how long to wait after triggering an HDC1080 in this configuration.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_conversion_time_micros(config: Hdc1080Config) -> PyResult<u32> {
    Ok(hdc1080::Configuration::try_from(config)?.conversion_time_micros())
}

/// Returns how long an HDC1080 temperature conversion takes, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_temperature_conversion_micros(bits: u8) -> PyResult<u32> {
    Ok(temperature_resolution(bits)?.conversion_time_micros())
}

/// Returns how long an HDC1080 humidity conversion takes, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_humidity_conversion_micros(bits: u8) -> PyResult<u32> {
    Ok(humidity_resolution(bits)?.conversion_time_micros())
}

/// An OPT3001 configuration register, field by field.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, PartialEq, Eq)]
pub struct Opt3001Config {
    /// The full-scale range number, `0..=11`, or `12` to set the range automatically.
    #[pyo3(get, set)]
    range_number: u8,
    /// Whether a conversion takes 800 ms rather than 100 ms.
    #[pyo3(get, set)]
    long_conversion: bool,
    /// The mode code: `0` shutdown, `1` single shot, `2` continuous.
    #[pyo3(get, set)]
    mode: u8,
    /// Whether the last result overflowed its range.
    #[pyo3(get, set)]
    overflow: bool,
    /// Whether a conversion has completed since the register was last read.
    #[pyo3(get, set)]
    conversion_ready: bool,
    /// Whether the result went above the high limit.
    #[pyo3(get, set)]
    flag_high: bool,
    /// Whether the result went below the low limit.
    #[pyo3(get, set)]
    flag_low: bool,
    /// Whether the INT pin latches until the configuration register is read.
    #[pyo3(get, set)]
    latched_window: bool,
    /// Whether the INT pin is active high.
    #[pyo3(get, set)]
    active_high: bool,
    /// Whether the limit registers carry a mantissa alone, without an exponent.
    #[pyo3(get, set)]
    mask_exponent: bool,
    /// The fault-count code, `0..=3`, for one, two, four, or eight faults.
    #[pyo3(get, set)]
    fault_count: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl Opt3001Config {
    /// Builds a configuration, defaulting every field to the part's reset state.
    #[new]
    #[pyo3(signature = (
        range_number = 12,
        long_conversion = true,
        mode = 0,
        overflow = false,
        conversion_ready = false,
        flag_high = false,
        flag_low = false,
        latched_window = true,
        active_high = false,
        mask_exponent = false,
        fault_count = 0,
    ))]
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    fn new(
        range_number: u8,
        long_conversion: bool,
        mode: u8,
        overflow: bool,
        conversion_ready: bool,
        flag_high: bool,
        flag_low: bool,
        latched_window: bool,
        active_high: bool,
        mask_exponent: bool,
        fault_count: u8,
    ) -> Self {
        Self {
            range_number,
            long_conversion,
            mode,
            overflow,
            conversion_ready,
            flag_high,
            flag_low,
            latched_window,
            active_high,
            mask_exponent,
            fault_count,
        }
    }

    /// Reports whether two configurations select the same settings.
    fn __eq__(&self, other: &Opt3001Config) -> bool {
        self == other
    }
}

/// Returns the illuminance one count carries at an OPT3001 exponent.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_lsb_milli_lux(exponent: u8) -> Option<u32> {
    opt3001::lsb_milli_lux(exponent)
}

/// Returns the full scale an OPT3001 range number covers, in milli-lux.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_full_scale_milli_lux(range_number: u8) -> Option<u32> {
    opt3001::full_scale_milli_lux(range_number)
}

/// Converts a raw OPT3001 result register to milli-lux.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_milli_lux(raw: u16) -> u32 {
    opt3001::milli_lux(raw)
}

/// Converts a raw OPT3001 result register to lux.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_lux(raw: u16) -> f32 {
    opt3001::lux(raw)
}

/// Builds the OPT3001 result register that decodes to an illuminance.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_raw_from_milli_lux(milli_lux: u32) -> u16 {
    opt3001::raw_from_milli_lux(milli_lux)
}

/// Reads the two bytes an OPT3001 sends for a register.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_word_from_bytes(data: Vec<u8>) -> PyResult<u16> {
    let bytes: [u8; 2] = data
        .as_slice()
        .try_into()
        .map_err(|_| length_error("register", 2))?;
    Ok(opt3001::word_from_bytes(bytes))
}

/// Builds the two bytes an OPT3001 sends for a register.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_word_to_bytes(word: u16) -> Vec<u8> {
    opt3001::word_to_bytes(word).to_vec()
}

/// Assembles the 16-bit OPT3001 configuration register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_config_bits(config: Opt3001Config) -> u16 {
    opt3001::Configuration::from(config).bits()
}

/// Parses a 16-bit OPT3001 configuration register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_config_from_bits(bits: u16) -> Opt3001Config {
    opt3001::Configuration::from_bits(bits).into()
}

/// Returns the conversion time an OPT3001 setting selects, in milliseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_conversion_millis(long_conversion: bool) -> u16 {
    conversion_time(long_conversion).millis()
}

/// Returns how many consecutive faults an OPT3001 fault-count code requires.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_fault_count(code: u8) -> u8 {
    opt3001::FaultCount::from_code(code).count()
}

/// Reports whether an OPT3001 range number sets the full scale automatically.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_is_automatic_range(range_number: u8) -> bool {
    range_number == opt3001::RANGE_AUTOMATIC
}

/// A decoded INA226 die-ID register.
#[gen_stub_pyclass]
#[pyclass]
pub struct Ina226DieId {
    /// The 12-bit device identifier.
    #[pyo3(get)]
    device: u16,
    /// The 4-bit die revision.
    #[pyo3(get)]
    revision: u8,
}

/// An INA226 configuration register, field by field.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, PartialEq, Eq)]
pub struct Ina226Config {
    /// Whether writing this resets the part.
    #[pyo3(get, set)]
    reset: bool,
    /// The averaging code, `0..=7`, from 1 to 1024 samples.
    #[pyo3(get, set)]
    averaging: u8,
    /// The bus-voltage conversion-time code, `0..=7`.
    #[pyo3(get, set)]
    bus_conversion_time: u8,
    /// The shunt-voltage conversion-time code, `0..=7`.
    #[pyo3(get, set)]
    shunt_conversion_time: u8,
    /// The operating-mode code, `0..=7`.
    #[pyo3(get, set)]
    mode: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl Ina226Config {
    /// Builds a configuration, defaulting every field to the part's reset state.
    #[new]
    #[pyo3(signature = (
        reset = false,
        averaging = 0,
        bus_conversion_time = 4,
        shunt_conversion_time = 4,
        mode = 7,
    ))]
    fn new(
        reset: bool,
        averaging: u8,
        bus_conversion_time: u8,
        shunt_conversion_time: u8,
        mode: u8,
    ) -> Self {
        Self {
            reset,
            averaging,
            bus_conversion_time,
            shunt_conversion_time,
            mode,
        }
    }

    /// Reports whether two configurations select the same settings.
    fn __eq__(&self, other: &Ina226Config) -> bool {
        self == other
    }
}

/// An INA226 Mask/Enable register, field by field.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, PartialEq, Eq)]
pub struct Ina226MaskEnable {
    /// Alert when the shunt voltage exceeds the limit.
    #[pyo3(get, set)]
    shunt_over_limit: bool,
    /// Alert when the shunt voltage drops below the limit.
    #[pyo3(get, set)]
    shunt_under_limit: bool,
    /// Alert when the bus voltage exceeds the limit.
    #[pyo3(get, set)]
    bus_over_limit: bool,
    /// Alert when the bus voltage drops below the limit.
    #[pyo3(get, set)]
    bus_under_limit: bool,
    /// Alert when the power exceeds the limit.
    #[pyo3(get, set)]
    power_over_limit: bool,
    /// Also alert when a conversion completes.
    #[pyo3(get, set)]
    conversion_ready: bool,
    /// Whether the selected limit function caused the last alert.
    #[pyo3(get, set)]
    alert_function_flag: bool,
    /// Whether every conversion and multiplication has completed.
    #[pyo3(get, set)]
    conversion_ready_flag: bool,
    /// Whether an arithmetic overflow left current and power invalid.
    #[pyo3(get, set)]
    math_overflow: bool,
    /// Whether the alert pin is active high.
    #[pyo3(get, set)]
    alert_active_high: bool,
    /// Whether the alert pin latches until this register is read.
    #[pyo3(get, set)]
    alert_latch: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl Ina226MaskEnable {
    /// Builds a Mask/Enable register, defaulting every field to the reset state.
    #[new]
    #[pyo3(signature = (
        shunt_over_limit = false,
        shunt_under_limit = false,
        bus_over_limit = false,
        bus_under_limit = false,
        power_over_limit = false,
        conversion_ready = false,
        alert_function_flag = false,
        conversion_ready_flag = false,
        math_overflow = false,
        alert_active_high = false,
        alert_latch = false,
    ))]
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    fn new(
        shunt_over_limit: bool,
        shunt_under_limit: bool,
        bus_over_limit: bool,
        bus_under_limit: bool,
        power_over_limit: bool,
        conversion_ready: bool,
        alert_function_flag: bool,
        conversion_ready_flag: bool,
        math_overflow: bool,
        alert_active_high: bool,
        alert_latch: bool,
    ) -> Self {
        Self {
            shunt_over_limit,
            shunt_under_limit,
            bus_over_limit,
            bus_under_limit,
            power_over_limit,
            conversion_ready,
            alert_function_flag,
            conversion_ready_flag,
            math_overflow,
            alert_active_high,
            alert_latch,
        }
    }

    /// Reports whether two registers select the same enables and flags.
    fn __eq__(&self, other: &Ina226MaskEnable) -> bool {
        self == other
    }
}

/// Returns the I2C address an INA226's A1 and A0 pin codes select.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_address(a1: u8, a0: u8) -> PyResult<u8> {
    Ok(ina226::address(address_pin(a1)?, address_pin(a0)?))
}

/// Returns how many samples an INA226 averaging code folds into one result.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_averaging_samples(code: u8) -> u16 {
    averaging(code).samples()
}

/// Returns the conversion time an INA226 code selects, in microseconds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_conversion_micros(code: u8) -> u32 {
    conversion_micros(code).microseconds()
}

/// Reports whether an INA226 mode code converts the shunt voltage.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_measures_shunt(code: u8) -> bool {
    mode(code).measures_shunt()
}

/// Reports whether an INA226 mode code converts the bus voltage.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_measures_bus(code: u8) -> bool {
    mode(code).measures_bus()
}

/// Reports whether an INA226 mode code keeps converting after the first result.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_is_continuous(code: u8) -> bool {
    mode(code).is_continuous()
}

/// Parses an INA226 configuration register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_config_from_register(raw: u16) -> Ina226Config {
    ina226::Configuration::from_register(raw).into()
}

/// Assembles an INA226 configuration register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_config_to_register(config: Ina226Config) -> u16 {
    ina226::Configuration::from(config).to_register()
}

/// Returns how often an INA226 in this configuration updates its results.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_update_micros(config: Ina226Config) -> u32 {
    ina226::Configuration::from(config).update_microseconds()
}

/// Parses an INA226 Mask/Enable register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_mask_enable_from_register(raw: u16) -> Ina226MaskEnable {
    ina226::MaskEnable::from_register(raw).into()
}

/// Assembles an INA226 Mask/Enable register value.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_mask_enable_to_register(mask: Ina226MaskEnable) -> u16 {
    ina226::MaskEnable::from(mask).to_register()
}

/// Returns the alert function an INA226 pin actually responds to.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_active_alert_function(mask: Ina226MaskEnable) -> Option<String> {
    ina226::MaskEnable::from(mask)
        .active_alert_function()
        .map(|function| alert_function_name(function).to_owned())
}

/// Splits an INA226 die-ID register into its device and revision fields.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_die_id(raw: u16) -> Ina226DieId {
    ina226::DieId::from_register(raw).into()
}

/// Checks that a pair of identification registers belongs to an INA226.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_identify(manufacturer_id: u16, die_id: u16) -> PyResult<Ina226DieId> {
    Ok(ina226::identify(manufacturer_id, die_id)
        .map_err(to_py)?
        .into())
}

/// Computes the INA226 calibration register for a shunt and current resolution.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_calibration(current_lsb_microamps: u32, shunt_milliohms: u32) -> u16 {
    ina226::calibration(current_lsb_microamps, shunt_milliohms)
}

/// Returns the smallest current resolution that still covers an expected maximum.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_minimum_current_lsb_microamps(max_expected_microamps: u32) -> u32 {
    ina226::minimum_current_lsb_microamps(max_expected_microamps)
}

/// Converts a raw INA226 shunt-voltage register to nanovolts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_shunt_nanovolts(raw: i16) -> i32 {
    ina226::shunt_nanovolts(raw)
}

/// Converts a raw INA226 shunt-voltage register to millivolts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_shunt_millivolts(raw: i16) -> f32 {
    ina226::shunt_millivolts_f32(raw)
}

/// Converts a raw INA226 bus-voltage register to microvolts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_bus_microvolts(raw: u16) -> u32 {
    ina226::bus_microvolts(raw)
}

/// Converts a raw INA226 bus-voltage register to volts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_bus_volts(raw: u16) -> f32 {
    ina226::bus_volts_f32(raw)
}

/// Converts a raw INA226 current register to microamps.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_current_microamps(raw: i16, current_lsb_microamps: u32) -> i32 {
    ina226::current_microamps(raw, current_lsb_microamps)
}

/// Converts a raw INA226 current register to amps.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_current_amps(raw: i16, current_lsb_microamps: u32) -> f32 {
    ina226::current_amps_f32(raw, current_lsb_microamps)
}

/// Converts a raw INA226 power register to microwatts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_power_microwatts(raw: u16, current_lsb_microamps: u32) -> u32 {
    ina226::power_microwatts(raw, current_lsb_microamps)
}

/// Converts a raw INA226 power register to watts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_power_watts(raw: u16, current_lsb_microamps: u32) -> f32 {
    ina226::power_watts_f32(raw, current_lsb_microamps)
}

/// Builds the INA226 shunt-voltage register a monitor reports for a shunt voltage.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_shunt_register(nanovolts: i32) -> i16 {
    ina226::shunt_register(nanovolts)
}

/// Builds the INA226 bus-voltage register a monitor reports for a bus voltage.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_bus_register(microvolts: u32) -> u16 {
    ina226::bus_register(microvolts)
}

/// Builds the INA226 current register a monitor reports for a current.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_current_register(microamps: i32, current_lsb_microamps: u32) -> i16 {
    ina226::current_register(microamps, current_lsb_microamps)
}

/// Builds the INA226 power register a monitor reports for a power.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_power_register(microwatts: u32, current_lsb_microamps: u32) -> u16 {
    ina226::power_register(microwatts, current_lsb_microamps)
}

/// Computes the INA226 current register the chip derives from a shunt reading.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_current_register_from_shunt(shunt: i16, calibration: u16) -> i16 {
    ina226::current_register_from_shunt(shunt, calibration)
}

/// Computes the INA226 power register the chip derives from a current reading.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_power_register_from_current(current: i16, bus: u16) -> u16 {
    ina226::power_register_from_current(current, bus)
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

impl From<bmp280::Calibration> for Bmp280Coefficients {
    fn from(value: bmp280::Calibration) -> Self {
        Bmp280Coefficients {
            dig_t1: value.dig_t1,
            dig_t2: value.dig_t2,
            dig_t3: value.dig_t3,
            dig_p1: value.dig_p1,
            dig_p2: value.dig_p2,
            dig_p3: value.dig_p3,
            dig_p4: value.dig_p4,
            dig_p5: value.dig_p5,
            dig_p6: value.dig_p6,
            dig_p7: value.dig_p7,
            dig_p8: value.dig_p8,
            dig_p9: value.dig_p9,
        }
    }
}

impl From<bmp280::Reading> for Bmp280Reading {
    fn from(value: bmp280::Reading) -> Self {
        Bmp280Reading {
            celsius: value.celsius(),
            pascals: value.pascals(),
            hectopascals: value.hectopascals(),
        }
    }
}

impl From<bmp280::Measurement> for Bmp280RawMeasurement {
    fn from(value: bmp280::Measurement) -> Self {
        Bmp280RawMeasurement {
            pressure: value.pressure,
            temperature: value.temperature,
            pressure_skipped: value.pressure_skipped(),
            temperature_skipped: value.temperature_skipped(),
        }
    }
}

impl From<Bmp280CtrlMeas> for bmp280::CtrlMeas {
    fn from(value: Bmp280CtrlMeas) -> Self {
        bmp280::CtrlMeas {
            temperature: bmp280::Oversampling::from_code(value.temperature),
            pressure: bmp280::Oversampling::from_code(value.pressure),
            mode: bmp280::Mode::from_code(value.mode),
        }
    }
}

impl From<bmp280::CtrlMeas> for Bmp280CtrlMeas {
    fn from(value: bmp280::CtrlMeas) -> Self {
        Bmp280CtrlMeas {
            temperature: value.temperature.code(),
            pressure: value.pressure.code(),
            mode: value.mode.code(),
        }
    }
}

impl From<Bmp280Config> for bmp280::Config {
    fn from(value: Bmp280Config) -> Self {
        bmp280::Config {
            standby: bmp280::Standby::from_code(value.standby),
            filter: value.filter,
            spi_3wire: value.spi_3wire,
        }
    }
}

impl From<bmp280::Config> for Bmp280Config {
    fn from(value: bmp280::Config) -> Self {
        Bmp280Config {
            standby: value.standby.code(),
            filter: value.filter,
            spi_3wire: value.spi_3wire,
        }
    }
}

impl From<sht3x::Measurement> for Sht3xMeasurement {
    fn from(value: sht3x::Measurement) -> Self {
        Sht3xMeasurement {
            temperature_raw: value.temperature_raw,
            humidity_raw: value.humidity_raw,
            milli_celsius: value.temperature_milli_celsius(),
            celsius: value.temperature_celsius(),
            milli_fahrenheit: value.temperature_milli_fahrenheit(),
            fahrenheit: value.temperature_fahrenheit(),
            milli_percent: value.humidity_milli_percent(),
            relative_humidity: value.relative_humidity(),
        }
    }
}

impl From<sht3x::Status> for Sht3xStatus {
    fn from(value: sht3x::Status) -> Self {
        Sht3xStatus {
            bits: value.bits(),
            alert_pending: value.alert_pending(),
            heater_on: value.heater_on(),
            humidity_tracking_alert: value.humidity_tracking_alert(),
            temperature_tracking_alert: value.temperature_tracking_alert(),
            reset_detected: value.reset_detected(),
            command_failed: value.command_failed(),
            write_checksum_failed: value.write_checksum_failed(),
        }
    }
}

impl From<scd4x::Measurement> for Scd4xMeasurement {
    fn from(value: scd4x::Measurement) -> Self {
        Scd4xMeasurement {
            co2_ppm: value.co2_ppm,
            temperature_raw: value.temperature_raw,
            humidity_raw: value.humidity_raw,
            milli_celsius: value.milli_celsius(),
            celsius: value.celsius(),
            humidity_milli_percent: value.humidity_milli_percent(),
            relative_humidity_percent: value.relative_humidity_percent(),
        }
    }
}

impl From<Tmp117Config> for tmp117::Configuration {
    fn from(value: Tmp117Config) -> Self {
        tmp117::Configuration {
            high_alert: value.high_alert,
            low_alert: value.low_alert,
            data_ready: value.data_ready,
            eeprom_busy: value.eeprom_busy,
            mode: tmp117::ConversionMode::from_code(value.mode),
            cycle: tmp117::ConversionCycle::from_code(value.cycle),
            averaging: tmp117::Averaging::from_code(value.averaging),
            alert_mode: if value.therm_mode {
                tmp117::AlertMode::Therm
            } else {
                tmp117::AlertMode::Alert
            },
            alert_polarity: if value.alert_active_high {
                tmp117::AlertPolarity::ActiveHigh
            } else {
                tmp117::AlertPolarity::ActiveLow
            },
            alert_pin: if value.alert_pin_data_ready {
                tmp117::AlertPin::DataReady
            } else {
                tmp117::AlertPin::AlertFlags
            },
            soft_reset: value.soft_reset,
        }
    }
}

impl From<tmp117::Configuration> for Tmp117Config {
    fn from(value: tmp117::Configuration) -> Self {
        Tmp117Config {
            high_alert: value.high_alert,
            low_alert: value.low_alert,
            data_ready: value.data_ready,
            eeprom_busy: value.eeprom_busy,
            mode: value.mode.code(),
            cycle: value.cycle.code(),
            averaging: value.averaging.code(),
            therm_mode: matches!(value.alert_mode, tmp117::AlertMode::Therm),
            alert_active_high: matches!(value.alert_polarity, tmp117::AlertPolarity::ActiveHigh),
            alert_pin_data_ready: matches!(value.alert_pin, tmp117::AlertPin::DataReady),
            soft_reset: value.soft_reset,
        }
    }
}

impl From<hdc1080::Measurement> for Hdc1080Measurement {
    fn from(value: hdc1080::Measurement) -> Self {
        Hdc1080Measurement {
            temperature_raw: value.temperature,
            humidity_raw: value.humidity,
            milli_celsius: value.milli_celsius(),
            celsius: value.celsius(),
            milli_percent: value.milli_percent(),
            relative_humidity: value.relative_humidity(),
        }
    }
}

impl From<hdc1080::Configuration> for Hdc1080Config {
    fn from(value: hdc1080::Configuration) -> Self {
        Hdc1080Config {
            software_reset: value.software_reset,
            heater: value.heater,
            sequential: matches!(
                value.mode,
                hdc1080::AcquisitionMode::TemperatureThenHumidity
            ),
            battery_low: value.battery_low,
            temperature_resolution_bits: match value.temperature_resolution {
                hdc1080::TemperatureResolution::Bits14 => 14,
                hdc1080::TemperatureResolution::Bits11 => 11,
            },
            humidity_resolution_bits: match value.humidity_resolution {
                hdc1080::HumidityResolution::Bits14 => 14,
                hdc1080::HumidityResolution::Bits11 => 11,
                hdc1080::HumidityResolution::Bits8 => 8,
            },
        }
    }
}

impl TryFrom<Hdc1080Config> for hdc1080::Configuration {
    type Error = PyErr;

    fn try_from(value: Hdc1080Config) -> PyResult<Self> {
        Ok(hdc1080::Configuration {
            software_reset: value.software_reset,
            heater: value.heater,
            mode: if value.sequential {
                hdc1080::AcquisitionMode::TemperatureThenHumidity
            } else {
                hdc1080::AcquisitionMode::Single
            },
            battery_low: value.battery_low,
            temperature_resolution: temperature_resolution(value.temperature_resolution_bits)?,
            humidity_resolution: humidity_resolution(value.humidity_resolution_bits)?,
        })
    }
}

impl From<Opt3001Config> for opt3001::Configuration {
    fn from(value: Opt3001Config) -> Self {
        opt3001::Configuration {
            range_number: value.range_number,
            conversion_time: conversion_time(value.long_conversion),
            mode: opt3001::Mode::from_code(value.mode),
            overflow: value.overflow,
            conversion_ready: value.conversion_ready,
            flag_high: value.flag_high,
            flag_low: value.flag_low,
            latch: if value.latched_window {
                opt3001::Latch::LatchedWindow
            } else {
                opt3001::Latch::TransparentHysteresis
            },
            polarity: if value.active_high {
                opt3001::Polarity::ActiveHigh
            } else {
                opt3001::Polarity::ActiveLow
            },
            mask_exponent: value.mask_exponent,
            fault_count: opt3001::FaultCount::from_code(value.fault_count),
        }
    }
}

impl From<opt3001::Configuration> for Opt3001Config {
    fn from(value: opt3001::Configuration) -> Self {
        Opt3001Config {
            range_number: value.range_number,
            long_conversion: matches!(value.conversion_time, opt3001::ConversionTime::Ms800),
            mode: value.mode.code(),
            overflow: value.overflow,
            conversion_ready: value.conversion_ready,
            flag_high: value.flag_high,
            flag_low: value.flag_low,
            latched_window: matches!(value.latch, opt3001::Latch::LatchedWindow),
            active_high: matches!(value.polarity, opt3001::Polarity::ActiveHigh),
            mask_exponent: value.mask_exponent,
            fault_count: value.fault_count.code(),
        }
    }
}

impl From<Ina226Config> for ina226::Configuration {
    fn from(value: Ina226Config) -> Self {
        ina226::Configuration {
            reset: value.reset,
            averaging: averaging(value.averaging),
            bus_conversion_time: conversion_micros(value.bus_conversion_time),
            shunt_conversion_time: conversion_micros(value.shunt_conversion_time),
            mode: mode(value.mode),
        }
    }
}

impl From<ina226::Configuration> for Ina226Config {
    fn from(value: ina226::Configuration) -> Self {
        Ina226Config {
            reset: value.reset,
            averaging: value.averaging as u8,
            bus_conversion_time: value.bus_conversion_time as u8,
            shunt_conversion_time: value.shunt_conversion_time as u8,
            mode: value.mode as u8,
        }
    }
}

impl From<Ina226MaskEnable> for ina226::MaskEnable {
    fn from(value: Ina226MaskEnable) -> Self {
        ina226::MaskEnable {
            shunt_over_limit: value.shunt_over_limit,
            shunt_under_limit: value.shunt_under_limit,
            bus_over_limit: value.bus_over_limit,
            bus_under_limit: value.bus_under_limit,
            power_over_limit: value.power_over_limit,
            conversion_ready: value.conversion_ready,
            alert_function_flag: value.alert_function_flag,
            conversion_ready_flag: value.conversion_ready_flag,
            math_overflow: value.math_overflow,
            alert_active_high: value.alert_active_high,
            alert_latch: value.alert_latch,
        }
    }
}

impl From<ina226::MaskEnable> for Ina226MaskEnable {
    fn from(value: ina226::MaskEnable) -> Self {
        Ina226MaskEnable {
            shunt_over_limit: value.shunt_over_limit,
            shunt_under_limit: value.shunt_under_limit,
            bus_over_limit: value.bus_over_limit,
            bus_under_limit: value.bus_under_limit,
            power_over_limit: value.power_over_limit,
            conversion_ready: value.conversion_ready,
            alert_function_flag: value.alert_function_flag,
            conversion_ready_flag: value.conversion_ready_flag,
            math_overflow: value.math_overflow,
            alert_active_high: value.alert_active_high,
            alert_latch: value.alert_latch,
        }
    }
}

impl From<ina226::DieId> for Ina226DieId {
    fn from(value: ina226::DieId) -> Self {
        Ina226DieId {
            device: value.device,
            revision: value.revision,
        }
    }
}

/// Reads an SHT3x repeatability back from its name.
fn read_repeatability(repeatability: &str) -> PyResult<sht3x::Repeatability> {
    match repeatability {
        "Low" => Ok(sht3x::Repeatability::Low),
        "Medium" => Ok(sht3x::Repeatability::Medium),
        "High" => Ok(sht3x::Repeatability::High),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "SHT3x repeatability must be Low, Medium, or High",
        )),
    }
}

/// Reads an SHT3x periodic rate back from its name.
fn read_rate(rate: &str) -> PyResult<sht3x::Rate> {
    match rate {
        "HalfMps" => Ok(sht3x::Rate::HalfMps),
        "OneMps" => Ok(sht3x::Rate::OneMps),
        "TwoMps" => Ok(sht3x::Rate::TwoMps),
        "FourMps" => Ok(sht3x::Rate::FourMps),
        "TenMps" => Ok(sht3x::Rate::TenMps),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "SHT3x rate must be HalfMps, OneMps, TwoMps, FourMps, or TenMps",
        )),
    }
}

/// Maps a bit count onto the HDC1080 temperature resolution it names.
fn temperature_resolution(bits: u8) -> PyResult<hdc1080::TemperatureResolution> {
    match bits {
        14 => Ok(hdc1080::TemperatureResolution::Bits14),
        11 => Ok(hdc1080::TemperatureResolution::Bits11),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "HDC1080 temperature resolution must be 14 or 11 bits",
        )),
    }
}

/// Maps a bit count onto the HDC1080 humidity resolution it names.
fn humidity_resolution(bits: u8) -> PyResult<hdc1080::HumidityResolution> {
    match bits {
        14 => Ok(hdc1080::HumidityResolution::Bits14),
        11 => Ok(hdc1080::HumidityResolution::Bits11),
        8 => Ok(hdc1080::HumidityResolution::Bits8),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "HDC1080 humidity resolution must be 14, 11, or 8 bits",
        )),
    }
}

/// Maps the long-conversion flag onto the OPT3001 conversion time it names.
fn conversion_time(long_conversion: bool) -> opt3001::ConversionTime {
    if long_conversion {
        opt3001::ConversionTime::Ms800
    } else {
        opt3001::ConversionTime::Ms100
    }
}

/// Maps an averaging code onto the INA226 setting it names.
fn averaging(code: u8) -> ina226::Averaging {
    ina226::Configuration::from_register(u16::from(code & 0x07) << 9).averaging
}

/// Maps a conversion-time code onto the INA226 setting it names.
fn conversion_micros(code: u8) -> ina226::ConversionTime {
    ina226::Configuration::from_register(u16::from(code & 0x07) << 6).bus_conversion_time
}

/// Maps a mode code onto the INA226 setting it names.
fn mode(code: u8) -> ina226::Mode {
    ina226::Configuration::from_register(u16::from(code & 0x07)).mode
}

/// Maps a pin code onto the level an INA226 address pin is tied to.
fn address_pin(code: u8) -> PyResult<ina226::AddressPin> {
    match code {
        0 => Ok(ina226::AddressPin::Ground),
        1 => Ok(ina226::AddressPin::Supply),
        2 => Ok(ina226::AddressPin::Sda),
        3 => Ok(ina226::AddressPin::Scl),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "an INA226 address pin must be tied to GND, VS, SDA, or SCL: code 0 to 3",
        )),
    }
}

/// Names an INA226 alert function as the string that crosses the boundary.
fn alert_function_name(function: ina226::AlertFunction) -> &'static str {
    match function {
        ina226::AlertFunction::ShuntOverLimit => "ShuntOverLimit",
        ina226::AlertFunction::ShuntUnderLimit => "ShuntUnderLimit",
        ina226::AlertFunction::BusOverLimit => "BusOverLimit",
        ina226::AlertFunction::BusUnderLimit => "BusUnderLimit",
        ina226::AlertFunction::PowerOverLimit => "PowerOverLimit",
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
