//! The C ABI for the sensor drivers.
//!
//! These functions wrap [`pamoja_sensors`] for callers that reach the SDK through
//! the flat C boundary: the decode half of four common parts, turning the register
//! bytes a bus driver read into the physical reading the datasheet says they mean.
//!
//! A reading is a handful of scalars, so it crosses by value as a `#[repr(C)]`
//! struct rather than as a handle. The one exception is a BME280's calibration,
//! which is read once at start-up and then reused for every measurement, so it is
//! a handle the caller keeps.
//!
//! Enumerated settings cross as the code the datasheet prints, because that is
//! what a caller holding a register value already has in front of them.

use pamoja_sensors::{ads1115, bme280, ds18b20, ina219, SensorError};

use crate::{read_bytes, set_last_error, PamojaStatus};

/// The number of calibration bytes a BME280 reports for temperature and pressure.
pub const PAMOJA_BME280_CALIBRATION_TEMP_PRESS_LEN: usize = 26;

/// The number of calibration bytes a BME280 reports for humidity.
pub const PAMOJA_BME280_CALIBRATION_HUMIDITY_LEN: usize = 7;

/// The number of measurement bytes a BME280 burst read returns.
pub const PAMOJA_BME280_MEASUREMENT_LEN: usize = 8;

/// The number of bytes in a DS18B20 scratchpad, the ninth being its CRC.
pub const PAMOJA_DS18B20_SCRATCHPAD_LEN: usize = 9;

/// An opaque handle to a BME280's factory calibration.
///
/// Read the calibration registers once at start-up, build one of these, and reuse
/// it for every measurement. Release it with
/// [`pamoja_bme280_calibration_free`].
pub struct PamojaBme280Calibration {
    calibration: bme280::Calibration,
}

/// A compensated BME280 reading.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaBme280Measurement {
    /// The temperature in degrees Celsius.
    pub celsius: f32,
    /// The pressure in pascals.
    pub pascals: u32,
    /// The pressure in hectopascals, the unit a barometer is usually quoted in.
    pub hectopascals: f32,
    /// The relative humidity as a percentage.
    pub relative_humidity_percent: f32,
}

/// A decoded DS18B20 scratchpad.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaDs18b20Reading {
    /// The raw temperature register, 1/16 degree Celsius per count.
    pub raw_temperature: i16,
    /// The temperature in micro-degrees Celsius, exact in integer arithmetic.
    pub micro_celsius: i32,
    /// The high alarm threshold in whole degrees Celsius.
    pub alarm_high: i8,
    /// The low alarm threshold in whole degrees Celsius.
    pub alarm_low: i8,
    /// The configured resolution, as a number of bits: 9, 10, 11, or 12.
    pub resolution_bits: u8,
}

/// An ADS1115 configuration register, field by field.
///
/// The multi-way settings carry the code the datasheet prints; the single-bit
/// settings are named for the state that bit selects, so there is no code to look
/// up for a flag.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaAds1115Config {
    /// `1` starts a single conversion when written.
    pub start_conversion: u8,
    /// The input multiplexer code, `0..=7`.
    pub mux: u8,
    /// The gain code, `0..=7`, which sets the full-scale range.
    pub pga: u8,
    /// `1` converts once per request and powers down, `0` converts continuously.
    pub single_shot: u8,
    /// The data rate code, `0..=7`.
    pub data_rate: u8,
    /// `1` selects the window comparator, `0` the traditional one.
    pub window_comparator: u8,
    /// `1` makes the ALERT/RDY pin active high.
    pub comparator_active_high: u8,
    /// `1` latches the comparator until the conversion is read.
    pub comparator_latching: u8,
    /// The comparator queue code, `0..=3`, where `3` disables the comparator.
    pub comparator_queue: u8,
}

/// Builds a BME280 calibration from the bytes read out of its registers.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_calibration` set to a new handle
/// the caller must release with [`pamoja_bme280_calibration_free`], or
/// [`PamojaStatus::InvalidArgument`] if either buffer is the wrong length.
///
/// # Safety
///
/// `temp_press` must point to at least `temp_press_len` readable bytes and
/// `humidity` to at least `humidity_len`, and `out_calibration` must point to a
/// writable `*mut PamojaBme280Calibration`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_calibration_new(
    temp_press: *const u8,
    temp_press_len: usize,
    humidity: *const u8,
    humidity_len: usize,
    out_calibration: *mut *mut PamojaBme280Calibration,
) -> PamojaStatus {
    if out_calibration.is_null() {
        set_last_error("out_calibration must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_calibration;
    *slot = std::ptr::null_mut();

    let temp_press = match read_bytes(temp_press, temp_press_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let humidity = match read_bytes(humidity, humidity_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };

    let Ok(temp_press) =
        <[u8; PAMOJA_BME280_CALIBRATION_TEMP_PRESS_LEN]>::try_from(&temp_press[..])
    else {
        return wrong_length("temperature and pressure calibration", 26);
    };
    let Ok(humidity) = <[u8; PAMOJA_BME280_CALIBRATION_HUMIDITY_LEN]>::try_from(&humidity[..])
    else {
        return wrong_length("humidity calibration", 7);
    };

    *slot = Box::into_raw(Box::new(PamojaBme280Calibration {
        calibration: bme280::Calibration::from_registers(&temp_press, &humidity),
    }));
    PamojaStatus::Ok
}

/// Turns a BME280 burst read into a compensated reading.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_measurement` filled in, or
/// [`PamojaStatus::InvalidArgument`] if the calibration is null or the
/// measurement is not eight bytes.
///
/// # Safety
///
/// `calibration` must be a live handle from [`pamoja_bme280_calibration_new`],
/// `measurement` must point to at least `measurement_len` readable bytes, and
/// `out_measurement` must point to a writable `PamojaBme280Measurement`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_compensate(
    calibration: *const PamojaBme280Calibration,
    measurement: *const u8,
    measurement_len: usize,
    out_measurement: *mut PamojaBme280Measurement,
) -> PamojaStatus {
    if calibration.is_null() || out_measurement.is_null() {
        set_last_error("calibration and out_measurement must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let measurement = match read_bytes(measurement, measurement_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(registers) = <[u8; PAMOJA_BME280_MEASUREMENT_LEN]>::try_from(&measurement[..]) else {
        return wrong_length("measurement", 8);
    };

    let reading = (*calibration)
        .calibration
        .compensate(&bme280::RawMeasurement::from_registers(&registers));
    *out_measurement = PamojaBme280Measurement {
        celsius: reading.celsius(),
        pascals: reading.pascals(),
        hectopascals: reading.hectopascals(),
        relative_humidity_percent: reading.relative_humidity_percent(),
    };
    PamojaStatus::Ok
}

/// Releases a BME280 calibration handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `calibration` must be a handle from [`pamoja_bme280_calibration_new`] that has
/// not already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_calibration_free(calibration: *mut PamojaBme280Calibration) {
    if !calibration.is_null() {
        drop(Box::from_raw(calibration));
    }
}

/// Parses and CRC-checks a nine-byte DS18B20 scratchpad.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_reading` filled in, or
/// [`PamojaStatus::Codec`] if the CRC does not match, which means the read was
/// corrupted on the bus and should be repeated.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes, and `out_reading`
/// must point to a writable `PamojaDs18b20Reading`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_parse_scratchpad(
    bytes: *const u8,
    bytes_len: usize,
    out_reading: *mut PamojaDs18b20Reading,
) -> PamojaStatus {
    if out_reading.is_null() {
        set_last_error("out_reading must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(scratchpad) = <[u8; PAMOJA_DS18B20_SCRATCHPAD_LEN]>::try_from(&bytes[..]) else {
        return wrong_length("scratchpad", 9);
    };

    match ds18b20::Scratchpad::parse(&scratchpad) {
        Ok(reading) => {
            *out_reading = PamojaDs18b20Reading {
                raw_temperature: reading.raw_temperature(),
                micro_celsius: reading.temperature_micro_celsius(),
                alarm_high: reading.alarm_high(),
                alarm_low: reading.alarm_low(),
                resolution_bits: reading.resolution().bits(),
            };
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Builds the nine bytes a DS18B20 in the given state puts on the bus, CRC last.
///
/// This is the inverse of [`pamoja_ds18b20_parse_scratchpad`], so a node can be
/// written and tested against what a thermometer sends without one attached.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the nine bytes written to `out_bytes`, or
/// [`PamojaStatus::InvalidArgument`] if `bits` is not 9, 10, 11, or 12.
///
/// # Safety
///
/// `out_bytes` must point to at least nine writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_build_scratchpad(
    celsius: f32,
    bits: u8,
    alarm_high: i8,
    alarm_low: i8,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(resolution) = resolution(bits) else {
        return bad_resolution();
    };
    let raw = ds18b20::temperature_from_celsius(celsius, resolution);
    let scratchpad = ds18b20::Scratchpad::new(raw, resolution, alarm_high, alarm_low);
    core::ptr::copy_nonoverlapping(scratchpad.to_bytes().as_ptr(), out_bytes, 9);
    PamojaStatus::Ok
}

/// Computes the Maxim CRC-8 a 1-Wire device checks its own bytes with.
///
/// # Returns
///
/// The checksum over `data`.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes, or be null when
/// `data_len` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_crc8(data: *const u8, data_len: usize) -> u8 {
    match read_bytes(data, data_len) {
        Ok(data) => ds18b20::crc8(&data),
        Err(_) => 0,
    }
}

/// Converts a raw DS18B20 temperature register to micro-degrees Celsius.
///
/// # Returns
///
/// The temperature, exact in integer arithmetic.
#[no_mangle]
pub extern "C" fn pamoja_ds18b20_micro_celsius(raw: i16) -> i32 {
    ds18b20::temperature_to_micro_celsius(raw)
}

/// Converts a raw DS18B20 temperature register to degrees Celsius.
///
/// # Returns
///
/// The temperature.
#[no_mangle]
pub extern "C" fn pamoja_ds18b20_celsius(raw: i16) -> f32 {
    ds18b20::temperature_to_celsius(raw)
}

/// Returns the configuration byte that selects a DS18B20 resolution.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_byte` set, or
/// [`PamojaStatus::InvalidArgument`] if `bits` is not 9, 10, 11, or 12.
///
/// # Safety
///
/// `out_byte` must point to a writable `uint8_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_config_byte(bits: u8, out_byte: *mut u8) -> PamojaStatus {
    if out_byte.is_null() {
        set_last_error("out_byte must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match resolution(bits) {
        Some(resolution) => {
            *out_byte = resolution.config_byte();
            PamojaStatus::Ok
        }
        None => bad_resolution(),
    }
}

/// Returns the resolution a DS18B20 configuration byte selects.
///
/// # Returns
///
/// The number of bits: 9, 10, 11, or 12. Every byte names a resolution, so this
/// never fails.
#[no_mangle]
pub extern "C" fn pamoja_ds18b20_resolution_bits(config_byte: u8) -> u8 {
    ds18b20::Resolution::from_config_byte(config_byte).bits()
}

/// Returns the temperature step a DS18B20 resolution resolves.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_micro_celsius` set, or
/// [`PamojaStatus::InvalidArgument`] if `bits` is not 9, 10, 11, or 12.
///
/// # Safety
///
/// `out_micro_celsius` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_step_micro_celsius(
    bits: u8,
    out_micro_celsius: *mut u32,
) -> PamojaStatus {
    if out_micro_celsius.is_null() {
        set_last_error("out_micro_celsius must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match resolution(bits) {
        Some(resolution) => {
            *out_micro_celsius = resolution.step_micro_celsius();
            PamojaStatus::Ok
        }
        None => bad_resolution(),
    }
}

/// Returns how long a DS18B20 conversion may take at a resolution.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_micros` set to the datasheet's
/// worst case, or [`PamojaStatus::InvalidArgument`] if `bits` is not 9, 10, 11,
/// or 12.
///
/// # Safety
///
/// `out_micros` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_max_conversion_micros(
    bits: u8,
    out_micros: *mut u32,
) -> PamojaStatus {
    if out_micros.is_null() {
        set_last_error("out_micros must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match resolution(bits) {
        Some(resolution) => {
            *out_micros = resolution.max_conversion_micros();
            PamojaStatus::Ok
        }
        None => bad_resolution(),
    }
}

/// Computes the INA219 calibration register for a shunt and current resolution.
///
/// # Returns
///
/// The register value to write.
#[no_mangle]
pub extern "C" fn pamoja_ina219_calibration(
    current_lsb_microamps: u32,
    shunt_milliohms: u32,
) -> u16 {
    ina219::calibration(current_lsb_microamps, shunt_milliohms)
}

/// Returns the smallest current resolution that still covers an expected maximum.
///
/// # Returns
///
/// The current LSB in microamps.
#[no_mangle]
pub extern "C" fn pamoja_ina219_minimum_current_lsb_microamps(max_expected_microamps: u32) -> u32 {
    ina219::minimum_current_lsb_microamps(max_expected_microamps)
}

/// Builds the INA219 shunt-voltage register a monitor reports for a shunt voltage.
///
/// # Returns
///
/// The signed shunt-voltage register, at 10 uV per count.
#[no_mangle]
pub extern "C" fn pamoja_ina219_shunt_register(microvolts: i32) -> i16 {
    ina219::shunt_register(microvolts)
}

/// Builds the INA219 bus-voltage register a monitor reports for a bus voltage.
///
/// # Returns
///
/// The bus-voltage register, with the conversion-ready flag set.
#[no_mangle]
pub extern "C" fn pamoja_ina219_bus_register(millivolts: u32) -> u16 {
    ina219::bus_register(millivolts)
}

/// Builds the INA219 current register a monitor reports for a current.
///
/// # Returns
///
/// The signed current register, or zero if `current_lsb_microamps` is zero.
#[no_mangle]
pub extern "C" fn pamoja_ina219_current_register(
    microamps: i32,
    current_lsb_microamps: u32,
) -> i16 {
    ina219::current_register(microamps, current_lsb_microamps)
}

/// Builds the INA219 power register a monitor reports for a power.
///
/// # Returns
///
/// The power register, or zero if `current_lsb_microamps` is zero.
#[no_mangle]
pub extern "C" fn pamoja_ina219_power_register(microwatts: u32, current_lsb_microamps: u32) -> u16 {
    ina219::power_register(microwatts, current_lsb_microamps)
}

/// Converts a raw INA219 shunt-voltage register to microvolts.
///
/// # Returns
///
/// The shunt voltage.
#[no_mangle]
pub extern "C" fn pamoja_ina219_shunt_microvolts(raw: i16) -> i32 {
    ina219::shunt_microvolts(raw)
}

/// Converts a raw INA219 bus-voltage register to millivolts.
///
/// # Returns
///
/// The bus voltage.
#[no_mangle]
pub extern "C" fn pamoja_ina219_bus_millivolts(raw: u16) -> u32 {
    ina219::bus_millivolts(raw)
}

/// Reports whether an INA219 bus-voltage register says a conversion is ready.
///
/// # Returns
///
/// `true` when the conversion-ready flag is set.
#[no_mangle]
pub extern "C" fn pamoja_ina219_conversion_ready(raw: u16) -> bool {
    ina219::conversion_ready(raw)
}

/// Reports whether an INA219 bus-voltage register flags a math overflow.
///
/// # Returns
///
/// `true` when the current or power reading is meaningless and the calibration
/// needs revisiting.
#[no_mangle]
pub extern "C" fn pamoja_ina219_math_overflow(raw: u16) -> bool {
    ina219::math_overflow(raw)
}

/// Converts a raw INA219 current register to microamps.
///
/// # Returns
///
/// The current, at the resolution the calibration selected.
#[no_mangle]
pub extern "C" fn pamoja_ina219_current_microamps(raw: i16, current_lsb_microamps: u32) -> i32 {
    ina219::current_microamps(raw, current_lsb_microamps)
}

/// Converts a raw INA219 power register to microwatts.
///
/// # Returns
///
/// The power, at the resolution the calibration selected.
#[no_mangle]
pub extern "C" fn pamoja_ina219_power_microwatts(raw: u16, current_lsb_microamps: u32) -> u32 {
    ina219::power_microwatts(raw, current_lsb_microamps)
}

/// Assembles the 16-bit ADS1115 configuration register value.
///
/// # Returns
///
/// The register value to write, most significant bit first.
#[no_mangle]
pub extern "C" fn pamoja_ads1115_config_bits(config: PamojaAds1115Config) -> u16 {
    ads1115::Config::from(config).bits()
}

/// Parses a 16-bit ADS1115 configuration register value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_config` filled in. Every register value
/// decodes, so this fails only on a null pointer.
///
/// # Safety
///
/// `out_config` must point to a writable `PamojaAds1115Config`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ads1115_config_from_bits(
    bits: u16,
    out_config: *mut PamojaAds1115Config,
) -> PamojaStatus {
    if out_config.is_null() {
        set_last_error("out_config must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_config = ads1115::Config::from_bits(bits).into();
    PamojaStatus::Ok
}

/// Returns the full-scale range an ADS1115 gain code selects.
///
/// # Returns
///
/// The full scale in microvolts.
#[no_mangle]
pub extern "C" fn pamoja_ads1115_full_scale_microvolts(pga: u8) -> u32 {
    ads1115::Pga::from_code(pga).full_scale_microvolts()
}

/// Returns the sample rate an ADS1115 data-rate code selects.
///
/// # Returns
///
/// The rate in samples per second.
#[no_mangle]
pub extern "C" fn pamoja_ads1115_samples_per_second(data_rate: u8) -> u16 {
    ads1115::DataRate::from_code(data_rate).samples_per_second()
}

/// Converts a raw ADS1115 conversion result to nanovolts.
///
/// # Returns
///
/// The measured voltage, exact in integer arithmetic at every gain setting.
#[no_mangle]
pub extern "C" fn pamoja_ads1115_to_nanovolts(pga: u8, raw: i16) -> i64 {
    ads1115::to_nanovolts(ads1115::Pga::from_code(pga), raw)
}

/// Converts a raw ADS1115 conversion result to volts.
///
/// # Returns
///
/// The measured voltage.
#[no_mangle]
pub extern "C" fn pamoja_ads1115_to_volts(pga: u8, raw: i16) -> f32 {
    ads1115::to_volts(ads1115::Pga::from_code(pga), raw)
}

impl From<PamojaAds1115Config> for ads1115::Config {
    fn from(value: PamojaAds1115Config) -> Self {
        ads1115::Config {
            start_conversion: value.start_conversion != 0,
            mux: ads1115::Mux::from_code(value.mux),
            pga: ads1115::Pga::from_code(value.pga),
            mode: if value.single_shot != 0 {
                ads1115::Mode::SingleShot
            } else {
                ads1115::Mode::Continuous
            },
            data_rate: ads1115::DataRate::from_code(value.data_rate),
            comparator_mode: if value.window_comparator != 0 {
                ads1115::ComparatorMode::Window
            } else {
                ads1115::ComparatorMode::Traditional
            },
            comparator_polarity: if value.comparator_active_high != 0 {
                ads1115::ComparatorPolarity::ActiveHigh
            } else {
                ads1115::ComparatorPolarity::ActiveLow
            },
            comparator_latch: if value.comparator_latching != 0 {
                ads1115::ComparatorLatch::Latching
            } else {
                ads1115::ComparatorLatch::NonLatching
            },
            comparator_queue: ads1115::ComparatorQueue::from_code(value.comparator_queue),
        }
    }
}

impl From<ads1115::Config> for PamojaAds1115Config {
    fn from(value: ads1115::Config) -> Self {
        PamojaAds1115Config {
            start_conversion: u8::from(value.start_conversion),
            mux: value.mux.code(),
            pga: value.pga.code(),
            single_shot: u8::from(matches!(value.mode, ads1115::Mode::SingleShot)),
            data_rate: value.data_rate.code(),
            window_comparator: u8::from(matches!(
                value.comparator_mode,
                ads1115::ComparatorMode::Window
            )),
            comparator_active_high: u8::from(matches!(
                value.comparator_polarity,
                ads1115::ComparatorPolarity::ActiveHigh
            )),
            comparator_latching: u8::from(matches!(
                value.comparator_latch,
                ads1115::ComparatorLatch::Latching
            )),
            comparator_queue: value.comparator_queue.code(),
        }
    }
}

/// Maps a bit count onto the resolution it names.
fn resolution(bits: u8) -> Option<ds18b20::Resolution> {
    match bits {
        9 => Some(ds18b20::Resolution::Bits9),
        10 => Some(ds18b20::Resolution::Bits10),
        11 => Some(ds18b20::Resolution::Bits11),
        12 => Some(ds18b20::Resolution::Bits12),
        _ => None,
    }
}

/// Records a rejected resolution and reports it as an invalid argument.
fn bad_resolution() -> PamojaStatus {
    set_last_error("DS18B20 resolution must be 9, 10, 11, or 12 bits".to_owned());
    PamojaStatus::InvalidArgument
}

/// Records a buffer of the wrong size and reports it as an invalid argument.
fn wrong_length(what: &str, expected: usize) -> PamojaStatus {
    set_last_error(format!("{what} must be exactly {expected} bytes"));
    PamojaStatus::InvalidArgument
}

/// Records a sensor error and maps it onto its status.
fn failed(error: SensorError) -> PamojaStatus {
    set_last_error(error.to_string());
    PamojaStatus::Codec
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn a_bme280_burst_read_compensates_to_a_reading() {
        // The calibration and measurement bytes from the crate's own datasheet case.
        let temp_press = [0u8; PAMOJA_BME280_CALIBRATION_TEMP_PRESS_LEN];
        let humidity = [0u8; PAMOJA_BME280_CALIBRATION_HUMIDITY_LEN];
        let measurement = [0u8; PAMOJA_BME280_MEASUREMENT_LEN];
        let mut calibration = ptr::null_mut();
        let mut reading = PamojaBme280Measurement {
            celsius: 0.0,
            pascals: 0,
            hectopascals: 0.0,
            relative_humidity_percent: 0.0,
        };

        // Safety: the inputs are valid slices and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_bme280_calibration_new(
                    temp_press.as_ptr(),
                    temp_press.len(),
                    humidity.as_ptr(),
                    humidity.len(),
                    &mut calibration
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_bme280_compensate(
                    calibration,
                    measurement.as_ptr(),
                    measurement.len(),
                    &mut reading
                ),
                PamojaStatus::Ok
            );
            pamoja_bme280_calibration_free(calibration);
        }
        assert!(reading.hectopascals.is_finite());
    }

    #[test]
    fn a_calibration_of_the_wrong_length_is_refused() {
        let short = [0u8; 4];
        let humidity = [0u8; PAMOJA_BME280_CALIBRATION_HUMIDITY_LEN];
        let mut calibration = ptr::null_mut();
        // Safety: the inputs are valid slices and the out-pointer is writable.
        let status = unsafe {
            pamoja_bme280_calibration_new(
                short.as_ptr(),
                short.len(),
                humidity.as_ptr(),
                humidity.len(),
                &mut calibration,
            )
        };
        assert_eq!(status, PamojaStatus::InvalidArgument);
        assert!(calibration.is_null());
    }

    #[test]
    fn a_scratchpad_decodes_and_its_crc_is_checked() {
        // 25.0625 C at 12-bit resolution, with the CRC the device would send.
        let mut bytes = [0x91u8, 0x01, 0x4B, 0x46, 0x7F, 0xFF, 0x0C, 0x10, 0x00];
        // Safety: the input is a valid slice.
        bytes[8] = unsafe { pamoja_ds18b20_crc8(bytes.as_ptr(), 8) };

        let mut reading = PamojaDs18b20Reading {
            raw_temperature: 0,
            micro_celsius: 0,
            alarm_high: 0,
            alarm_low: 0,
            resolution_bits: 0,
        };
        // Safety: the input is a valid slice and the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_ds18b20_parse_scratchpad(bytes.as_ptr(), bytes.len(), &mut reading),
                PamojaStatus::Ok
            );
        }
        assert_eq!(reading.raw_temperature, 0x0191);
        assert_eq!(reading.micro_celsius, 25_062_500);
        assert_eq!(reading.resolution_bits, 12);

        bytes[0] ^= 0xFF;
        // Safety: the input is a valid slice and the out-pointer is writable.
        let status =
            unsafe { pamoja_ds18b20_parse_scratchpad(bytes.as_ptr(), bytes.len(), &mut reading) };
        assert_eq!(
            status,
            PamojaStatus::Codec,
            "a read corrupted on the bus must not be trusted"
        );
    }

    #[test]
    fn a_resolution_outside_the_datasheet_is_refused() {
        let mut byte = 0u8;
        // Safety: the out-pointer is writable.
        let status = unsafe { pamoja_ds18b20_config_byte(8, &mut byte) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
    }

    #[test]
    fn the_resolution_round_trips_through_its_config_byte() {
        for bits in [9u8, 10, 11, 12] {
            let mut byte = 0u8;
            // Safety: the out-pointer is writable.
            unsafe {
                assert_eq!(
                    pamoja_ds18b20_config_byte(bits, &mut byte),
                    PamojaStatus::Ok
                );
            }
            assert_eq!(pamoja_ds18b20_resolution_bits(byte), bits);
        }
    }

    #[test]
    fn an_ina219_reading_converts_at_its_calibrated_resolution() {
        // The datasheet's worked design example: 15 A across a 2 milliohm shunt at
        // 1 mA per count.
        const CURRENT_LSB: u32 = 1_000;
        assert_eq!(pamoja_ina219_calibration(CURRENT_LSB, 2), 0x5000);
        assert_eq!(
            pamoja_ina219_minimum_current_lsb_microamps(15_000_000),
            458,
            "15 A over a 15-bit register, rounded up to the next whole microamp"
        );
        assert_eq!(
            pamoja_ina219_current_microamps(1_000, CURRENT_LSB),
            1_000_000
        );
        // The power LSB is fixed at twenty times the current LSB.
        assert_eq!(pamoja_ina219_power_microwatts(100, CURRENT_LSB), 2_000_000);
        assert!(pamoja_ina219_conversion_ready(0x0002));
        assert!(pamoja_ina219_math_overflow(0x0001));
    }

    #[test]
    fn an_ads1115_config_round_trips_through_its_register() {
        let config = PamojaAds1115Config {
            start_conversion: 1,
            mux: 4,
            pga: 1,
            single_shot: 1,
            data_rate: 4,
            window_comparator: 0,
            comparator_active_high: 0,
            comparator_latching: 0,
            comparator_queue: 3,
        };
        let bits = pamoja_ads1115_config_bits(config);
        let mut back = config;
        // Safety: the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_ads1115_config_from_bits(bits, &mut back),
                PamojaStatus::Ok
            );
        }
        assert_eq!(back, config);
    }

    #[test]
    fn an_ads1115_conversion_scales_to_its_full_range() {
        // Gain code 1 is the plus or minus 4.096 V range.
        assert_eq!(pamoja_ads1115_full_scale_microvolts(1), 4_096_000);
        assert_eq!(pamoja_ads1115_to_nanovolts(1, 32_767), 4_095_875_000);
        assert!((pamoja_ads1115_to_volts(1, 0)).abs() < f32::EPSILON);
    }
}
