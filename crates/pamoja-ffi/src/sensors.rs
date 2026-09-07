//! The C ABI for the sensor drivers.
//!
//! These functions wrap [`pamoja_sensors`] for callers that reach the SDK through
//! the flat C boundary: the decode half of eleven common parts, turning the
//! register bytes a bus driver read into the physical reading the datasheet says
//! they mean, and the inverse builders that produce those bytes again.
//!
//! A reading is a handful of scalars, so it crosses by value as a `#[repr(C)]`
//! struct rather than as a handle. The exceptions are the BME280 and BMP280
//! calibrations, which are read once at start-up and then reused for every
//! measurement, so each is a handle the caller keeps.
//!
//! Enumerated settings cross as the code the datasheet prints, because that is
//! what a caller holding a register value already has in front of them, and
//! booleans cross as a `uint8_t` that is `1` for the state its name describes.
//!
//! A part that carries its own checksum, an undefined register code, or an
//! identification register reports a mismatch as [`PamojaStatus::Codec`], so a
//! caller re-reads rather than trusting the value.

use pamoja_sensors::{
    ads1115, bme280, bmp280, ds18b20, hdc1080, ina219, ina226, opt3001, scd4x, sht3x, tmp117,
    SensorError,
};

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

/// The number of calibration bytes a BMP280 reports.
pub const PAMOJA_BMP280_CALIBRATION_LEN: usize = 24;

/// The number of measurement bytes a BMP280 burst read returns.
pub const PAMOJA_BMP280_DATA_LEN: usize = 6;

/// The address a BMP280 answers on with its SDO pin low.
pub const PAMOJA_BMP280_I2C_ADDRESS_PRIMARY: u8 = 0x76;

/// The address it answers on with SDO high.
pub const PAMOJA_BMP280_I2C_ADDRESS_SECONDARY: u8 = 0x77;

/// The value a BMP280's chip-ID register reads, which confirms the part.
pub const PAMOJA_BMP280_CHIP_ID: u8 = 0x58;

/// The byte written to the reset register to restart a BMP280.
pub const PAMOJA_BMP280_RESET_WORD: u8 = 0xB6;

/// The raw code a BMP280 reports when oversampling is off and nothing was measured.
pub const PAMOJA_BMP280_SKIPPED_OUTPUT: u32 = 0x80000;

/// The first of the 24 BMP280 calibration registers.
pub const PAMOJA_BMP280_REGISTER_CALIBRATION: u8 = 0x88;

/// The BMP280 chip-ID register.
pub const PAMOJA_BMP280_REGISTER_CHIP_ID: u8 = 0xD0;

/// The BMP280 reset register.
pub const PAMOJA_BMP280_REGISTER_RESET: u8 = 0xE0;

/// The BMP280 status register.
pub const PAMOJA_BMP280_REGISTER_STATUS: u8 = 0xF3;

/// The BMP280 `ctrl_meas` register.
pub const PAMOJA_BMP280_REGISTER_CTRL_MEAS: u8 = 0xF4;

/// The BMP280 `config` register.
pub const PAMOJA_BMP280_REGISTER_CONFIG: u8 = 0xF5;

/// The first of the six BMP280 data registers.
pub const PAMOJA_BMP280_REGISTER_DATA: u8 = 0xF7;

// The header generator does not read the crates this one depends on, so these
// carry their value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_BMP280_CALIBRATION_LEN == bmp280::CALIBRATION_LEN);
const _: () = assert!(PAMOJA_BMP280_DATA_LEN == bmp280::DATA_LEN);
const _: () = assert!(PAMOJA_BMP280_I2C_ADDRESS_PRIMARY == bmp280::I2C_ADDRESS_PRIMARY);
const _: () = assert!(PAMOJA_BMP280_I2C_ADDRESS_SECONDARY == bmp280::I2C_ADDRESS_SECONDARY);
const _: () = assert!(PAMOJA_BMP280_CHIP_ID == bmp280::CHIP_ID);
const _: () = assert!(PAMOJA_BMP280_RESET_WORD == bmp280::RESET_WORD);
const _: () = assert!(PAMOJA_BMP280_SKIPPED_OUTPUT == bmp280::SKIPPED_OUTPUT);
const _: () = assert!(PAMOJA_BMP280_REGISTER_CALIBRATION == bmp280::register::CALIBRATION);
const _: () = assert!(PAMOJA_BMP280_REGISTER_CHIP_ID == bmp280::register::CHIP_ID);
const _: () = assert!(PAMOJA_BMP280_REGISTER_RESET == bmp280::register::RESET);
const _: () = assert!(PAMOJA_BMP280_REGISTER_STATUS == bmp280::register::STATUS);
const _: () = assert!(PAMOJA_BMP280_REGISTER_CTRL_MEAS == bmp280::register::CTRL_MEAS);
const _: () = assert!(PAMOJA_BMP280_REGISTER_CONFIG == bmp280::register::CONFIG);
const _: () = assert!(PAMOJA_BMP280_REGISTER_DATA == bmp280::register::DATA);

/// An opaque handle to a BMP280's factory calibration.
///
/// Read the calibration registers once at start-up, build one of these, and reuse
/// it for every measurement. Release it with
/// [`pamoja_bmp280_calibration_free`].
pub struct PamojaBmp280Calibration {
    calibration: bmp280::Calibration,
}

/// A BMP280's per-chip trimming coefficients, as they sit in its registers.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaBmp280Coefficients {
    /// The `dig_T1` coefficient.
    pub dig_t1: u16,
    /// The `dig_T2` coefficient.
    pub dig_t2: i16,
    /// The `dig_T3` coefficient.
    pub dig_t3: i16,
    /// The `dig_P1` coefficient.
    pub dig_p1: u16,
    /// The `dig_P2` coefficient.
    pub dig_p2: i16,
    /// The `dig_P3` coefficient.
    pub dig_p3: i16,
    /// The `dig_P4` coefficient.
    pub dig_p4: i16,
    /// The `dig_P5` coefficient.
    pub dig_p5: i16,
    /// The `dig_P6` coefficient.
    pub dig_p6: i16,
    /// The `dig_P7` coefficient.
    pub dig_p7: i16,
    /// The `dig_P8` coefficient.
    pub dig_p8: i16,
    /// The `dig_P9` coefficient.
    pub dig_p9: i16,
}

/// A compensated BMP280 reading.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaBmp280Reading {
    /// The temperature in degrees Celsius.
    pub celsius: f32,
    /// The pressure in pascals.
    pub pascals: u32,
    /// The pressure in hectopascals, the unit a barometer is usually quoted in.
    pub hectopascals: f32,
}

/// The uncompensated codes a BMP280 burst read carries.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaBmp280Measurement {
    /// The 20-bit pressure code.
    pub pressure: u32,
    /// The 20-bit temperature code.
    pub temperature: u32,
    /// `1` when pressure oversampling was off, so the code carries no reading.
    pub pressure_skipped: u8,
    /// `1` when temperature oversampling was off, so the code carries no reading.
    pub temperature_skipped: u8,
}

/// A BMP280 `ctrl_meas` register, field by field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaBmp280CtrlMeas {
    /// The temperature oversampling code, `0..=5`, where `0` skips the measurement.
    pub temperature: u8,
    /// The pressure oversampling code, `0..=5`, where `0` skips the measurement.
    pub pressure: u8,
    /// The power mode code: `0` sleep, `1` forced, `3` normal.
    pub mode: u8,
}

/// A BMP280 `config` register, field by field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaBmp280Config {
    /// The normal-mode standby code, `0..=7`.
    pub standby: u8,
    /// The IIR filter code, `0..=7`.
    pub filter: u8,
    /// `1` enables the 3-wire SPI interface.
    pub spi_3wire: u8,
}

/// Builds a BMP280 calibration from the 24 bytes read out of its registers.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_calibration` set to a new handle
/// the caller must release with [`pamoja_bmp280_calibration_free`], or
/// [`PamojaStatus::InvalidArgument`] if the buffer is not 24 bytes.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes, and
/// `out_calibration` must point to a writable `*mut PamojaBmp280Calibration`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_calibration_new(
    bytes: *const u8,
    bytes_len: usize,
    out_calibration: *mut *mut PamojaBmp280Calibration,
) -> PamojaStatus {
    if out_calibration.is_null() {
        set_last_error("out_calibration must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_calibration;
    *slot = std::ptr::null_mut();

    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(registers) = <[u8; PAMOJA_BMP280_CALIBRATION_LEN]>::try_from(&bytes[..]) else {
        return wrong_length("calibration", PAMOJA_BMP280_CALIBRATION_LEN);
    };

    *slot = Box::into_raw(Box::new(PamojaBmp280Calibration {
        calibration: bmp280::Calibration::parse(&registers),
    }));
    PamojaStatus::Ok
}

/// Turns a BMP280 burst read into a compensated reading.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_reading` filled in, or
/// [`PamojaStatus::InvalidArgument`] if the calibration is null or the
/// measurement is not six bytes.
///
/// # Safety
///
/// `calibration` must be a live handle from [`pamoja_bmp280_calibration_new`],
/// `measurement` must point to at least `measurement_len` readable bytes, and
/// `out_reading` must point to a writable `PamojaBmp280Reading`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_compensate(
    calibration: *const PamojaBmp280Calibration,
    measurement: *const u8,
    measurement_len: usize,
    out_reading: *mut PamojaBmp280Reading,
) -> PamojaStatus {
    if calibration.is_null() || out_reading.is_null() {
        set_last_error("calibration and out_reading must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let measurement = match read_bytes(measurement, measurement_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(registers) = <[u8; PAMOJA_BMP280_DATA_LEN]>::try_from(&measurement[..]) else {
        return wrong_length("measurement", PAMOJA_BMP280_DATA_LEN);
    };

    let reading = (*calibration)
        .calibration
        .compensate(&bmp280::Measurement::parse(&registers));
    *out_reading = PamojaBmp280Reading {
        celsius: reading.celsius(),
        pascals: reading.pascals(),
        hectopascals: reading.hectopascals(),
    };
    PamojaStatus::Ok
}

/// Rebuilds the 24 calibration bytes a device holding these coefficients returns.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the 24 bytes written to `out_bytes`, or
/// [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `calibration` must be a live handle from [`pamoja_bmp280_calibration_new`],
/// and `out_bytes` must point to at least 24 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_calibration_to_bytes(
    calibration: *const PamojaBmp280Calibration,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if calibration.is_null() || out_bytes.is_null() {
        set_last_error("calibration and out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = (*calibration).calibration.to_bytes();
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_BMP280_CALIBRATION_LEN);
    PamojaStatus::Ok
}

/// Reads out the trimming coefficients a BMP280 calibration carries.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_coefficients` filled in, or
/// [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `calibration` must be a live handle from [`pamoja_bmp280_calibration_new`],
/// and `out_coefficients` must point to a writable `PamojaBmp280Coefficients`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_calibration_coefficients(
    calibration: *const PamojaBmp280Calibration,
    out_coefficients: *mut PamojaBmp280Coefficients,
) -> PamojaStatus {
    if calibration.is_null() || out_coefficients.is_null() {
        set_last_error("calibration and out_coefficients must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_coefficients = (*calibration).calibration.into();
    PamojaStatus::Ok
}

/// Releases a BMP280 calibration handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `calibration` must be a handle from [`pamoja_bmp280_calibration_new`] that has
/// not already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_calibration_free(calibration: *mut PamojaBmp280Calibration) {
    if !calibration.is_null() {
        drop(Box::from_raw(calibration));
    }
}

/// Unpacks the six data bytes a BMP280 burst read returns.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_measurement` filled in, or
/// [`PamojaStatus::InvalidArgument`] if the buffer is not six bytes.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes, and `out_measurement`
/// must point to a writable `PamojaBmp280Measurement`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_parse_measurement(
    data: *const u8,
    data_len: usize,
    out_measurement: *mut PamojaBmp280Measurement,
) -> PamojaStatus {
    if out_measurement.is_null() {
        set_last_error("out_measurement must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let data = match read_bytes(data, data_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(registers) = <[u8; PAMOJA_BMP280_DATA_LEN]>::try_from(&data[..]) else {
        return wrong_length("measurement", PAMOJA_BMP280_DATA_LEN);
    };
    *out_measurement = bmp280::Measurement::parse(&registers).into();
    PamojaStatus::Ok
}

/// Builds the six data bytes a BMP280 holding these codes would return.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the six bytes written to `out_bytes`, or
/// [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least six writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_measurement_bytes(
    pressure: u32,
    temperature: u32,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let measurement = bmp280::Measurement {
        pressure,
        temperature,
    };
    let bytes = measurement.to_bytes();
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_BMP280_DATA_LEN);
    PamojaStatus::Ok
}

/// Reports whether a BMP280 status byte says a conversion is running.
///
/// # Returns
///
/// `true` while the part is measuring.
#[no_mangle]
pub extern "C" fn pamoja_bmp280_measuring(status: u8) -> bool {
    bmp280::measuring(status)
}

/// Reports whether a BMP280 status byte says the calibration image is loading.
///
/// # Returns
///
/// `true` while the coefficients are being copied out of non-volatile memory.
#[no_mangle]
pub extern "C" fn pamoja_bmp280_image_updating(status: u8) -> bool {
    bmp280::image_updating(status)
}

/// Assembles a BMP280 `ctrl_meas` register value.
///
/// # Returns
///
/// The register value to write.
#[no_mangle]
pub extern "C" fn pamoja_bmp280_ctrl_meas_bits(config: PamojaBmp280CtrlMeas) -> u8 {
    bmp280::CtrlMeas::from(config).bits()
}

/// Parses a BMP280 `ctrl_meas` register value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_config` filled in. Every register value
/// decodes, so this fails only on a null pointer.
///
/// # Safety
///
/// `out_config` must point to a writable `PamojaBmp280CtrlMeas`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_ctrl_meas_from_bits(
    bits: u8,
    out_config: *mut PamojaBmp280CtrlMeas,
) -> PamojaStatus {
    if out_config.is_null() {
        set_last_error("out_config must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_config = bmp280::CtrlMeas::from_bits(bits).into();
    PamojaStatus::Ok
}

/// Assembles a BMP280 `config` register value.
///
/// # Returns
///
/// The register value to write.
#[no_mangle]
pub extern "C" fn pamoja_bmp280_config_bits(config: PamojaBmp280Config) -> u8 {
    bmp280::Config::from(config).bits()
}

/// Parses a BMP280 `config` register value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_config` filled in. Every register value
/// decodes, so this fails only on a null pointer.
///
/// # Safety
///
/// `out_config` must point to a writable `PamojaBmp280Config`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_config_from_bits(
    bits: u8,
    out_config: *mut PamojaBmp280Config,
) -> PamojaStatus {
    if out_config.is_null() {
        set_last_error("out_config must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_config = bmp280::Config::from_bits(bits).into();
    PamojaStatus::Ok
}

/// Returns how many samples a BMP280 oversampling code averages.
///
/// # Returns
///
/// The oversampling factor, `1` to `16`.
#[no_mangle]
pub extern "C" fn pamoja_bmp280_oversampling_factor(code: u8) -> u8 {
    bmp280::Oversampling::from_code(code).factor()
}

/// Returns the normal-mode standby period a BMP280 code selects.
///
/// # Returns
///
/// The period in microseconds.
#[no_mangle]
pub extern "C" fn pamoja_bmp280_standby_micros(code: u8) -> u32 {
    bmp280::Standby::from_code(code).microseconds()
}

/// The number of bytes in an SHT3x measurement frame, each word followed by its CRC.
pub const PAMOJA_SHT3X_MEASUREMENT_LEN: usize = 6;

/// The number of bytes in an SHT3x word frame: the word then its CRC.
pub const PAMOJA_SHT3X_WORD_LEN: usize = 3;

/// The address an SHT3x answers on with its ADDR pin low.
pub const PAMOJA_SHT3X_I2C_ADDRESS_A: u8 = 0x44;

/// The address it answers on with ADDR high.
pub const PAMOJA_SHT3X_I2C_ADDRESS_B: u8 = 0x45;

/// The gap an SHT3x needs between two commands, in microseconds.
pub const PAMOJA_SHT3X_MIN_COMMAND_GAP_MICROS: u32 = 1_000;

/// The status word an SHT3x reads after a reset.
pub const PAMOJA_SHT3X_STATUS_DEFAULT: u16 = 0x8010;

/// One high-repeatability measurement, holding the bus until it is ready.
pub const PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_HIGH_STRETCH: u16 = 0x2C06;

/// One medium-repeatability measurement, holding the bus.
pub const PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_MEDIUM_STRETCH: u16 = 0x2C0D;

/// One low-repeatability measurement, holding the bus.
pub const PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_LOW_STRETCH: u16 = 0x2C10;

/// One high-repeatability measurement, released and fetched later.
pub const PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_HIGH: u16 = 0x2400;

/// One medium-repeatability measurement, released and fetched later.
pub const PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_MEDIUM: u16 = 0x240B;

/// One low-repeatability measurement, released and fetched later.
pub const PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_LOW: u16 = 0x2416;

/// A measurement every two seconds at high repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_HALF_MPS_HIGH: u16 = 0x2032;

/// A measurement every two seconds at medium repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_HALF_MPS_MEDIUM: u16 = 0x2024;

/// A measurement every two seconds at low repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_HALF_MPS_LOW: u16 = 0x202F;

/// One measurement a second at high repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_ONE_MPS_HIGH: u16 = 0x2130;

/// One measurement a second at medium repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_ONE_MPS_MEDIUM: u16 = 0x2126;

/// One measurement a second at low repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_ONE_MPS_LOW: u16 = 0x212D;

/// Two measurements a second at high repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_TWO_MPS_HIGH: u16 = 0x2236;

/// Two measurements a second at medium repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_TWO_MPS_MEDIUM: u16 = 0x2220;

/// Two measurements a second at low repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_TWO_MPS_LOW: u16 = 0x222B;

/// Four measurements a second at high repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_FOUR_MPS_HIGH: u16 = 0x2334;

/// Four measurements a second at medium repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_FOUR_MPS_MEDIUM: u16 = 0x2322;

/// Four measurements a second at low repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_FOUR_MPS_LOW: u16 = 0x2329;

/// Ten measurements a second at high repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_TEN_MPS_HIGH: u16 = 0x2737;

/// Ten measurements a second at medium repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_TEN_MPS_MEDIUM: u16 = 0x2721;

/// Ten measurements a second at low repeatability.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_TEN_MPS_LOW: u16 = 0x272A;

/// Accelerated response time: four measurements a second with a faster filter.
pub const PAMOJA_SHT3X_COMMAND_PERIODIC_ART: u16 = 0x2B32;

/// Fetches the last result of a periodic measurement.
pub const PAMOJA_SHT3X_COMMAND_FETCH_DATA: u16 = 0xE000;

/// Leaves periodic mode so another command can be accepted.
pub const PAMOJA_SHT3X_COMMAND_BREAK: u16 = 0x3093;

/// Restarts the part as if it had been power-cycled.
pub const PAMOJA_SHT3X_COMMAND_SOFT_RESET: u16 = 0x30A2;

/// The general-call reset, addressed to 0x00.
pub const PAMOJA_SHT3X_COMMAND_GENERAL_CALL_RESET: u16 = 0x0006;

/// Turns the on-die heater on.
pub const PAMOJA_SHT3X_COMMAND_HEATER_ENABLE: u16 = 0x306D;

/// Turns the on-die heater off.
pub const PAMOJA_SHT3X_COMMAND_HEATER_DISABLE: u16 = 0x3066;

/// Reads the status register.
pub const PAMOJA_SHT3X_COMMAND_READ_STATUS: u16 = 0xF32D;

/// Clears the latched flags in the status register.
pub const PAMOJA_SHT3X_COMMAND_CLEAR_STATUS: u16 = 0x3041;

// The header generator does not read the crates this one depends on, so these
// carry their value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_SHT3X_I2C_ADDRESS_A == sht3x::I2C_ADDRESS_A);
const _: () = assert!(PAMOJA_SHT3X_I2C_ADDRESS_B == sht3x::I2C_ADDRESS_B);
const _: () = assert!(PAMOJA_SHT3X_MIN_COMMAND_GAP_MICROS == sht3x::MIN_COMMAND_GAP_MICROS);
const _: () = assert!(PAMOJA_SHT3X_STATUS_DEFAULT == sht3x::Status::DEFAULT);
const _: () = assert!(
    PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_HIGH_STRETCH == sht3x::command::SINGLE_SHOT_HIGH_STRETCH
);
const _: () = assert!(
    PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_MEDIUM_STRETCH == sht3x::command::SINGLE_SHOT_MEDIUM_STRETCH
);
const _: () = assert!(
    PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_LOW_STRETCH == sht3x::command::SINGLE_SHOT_LOW_STRETCH
);
const _: () = assert!(PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_HIGH == sht3x::command::SINGLE_SHOT_HIGH);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_MEDIUM == sht3x::command::SINGLE_SHOT_MEDIUM);
const _: () = assert!(PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_LOW == sht3x::command::SINGLE_SHOT_LOW);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_HALF_MPS_HIGH == sht3x::command::PERIODIC_0_5_MPS_HIGH);
const _: () = assert!(
    PAMOJA_SHT3X_COMMAND_PERIODIC_HALF_MPS_MEDIUM == sht3x::command::PERIODIC_0_5_MPS_MEDIUM
);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_HALF_MPS_LOW == sht3x::command::PERIODIC_0_5_MPS_LOW);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_ONE_MPS_HIGH == sht3x::command::PERIODIC_1_MPS_HIGH);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_ONE_MPS_MEDIUM == sht3x::command::PERIODIC_1_MPS_MEDIUM);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_ONE_MPS_LOW == sht3x::command::PERIODIC_1_MPS_LOW);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_TWO_MPS_HIGH == sht3x::command::PERIODIC_2_MPS_HIGH);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_TWO_MPS_MEDIUM == sht3x::command::PERIODIC_2_MPS_MEDIUM);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_TWO_MPS_LOW == sht3x::command::PERIODIC_2_MPS_LOW);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_FOUR_MPS_HIGH == sht3x::command::PERIODIC_4_MPS_HIGH);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_FOUR_MPS_MEDIUM == sht3x::command::PERIODIC_4_MPS_MEDIUM);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_FOUR_MPS_LOW == sht3x::command::PERIODIC_4_MPS_LOW);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_TEN_MPS_HIGH == sht3x::command::PERIODIC_10_MPS_HIGH);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_TEN_MPS_MEDIUM == sht3x::command::PERIODIC_10_MPS_MEDIUM);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_TEN_MPS_LOW == sht3x::command::PERIODIC_10_MPS_LOW);
const _: () = assert!(PAMOJA_SHT3X_COMMAND_PERIODIC_ART == sht3x::command::PERIODIC_ART);
const _: () = assert!(PAMOJA_SHT3X_COMMAND_FETCH_DATA == sht3x::command::FETCH_DATA);
const _: () = assert!(PAMOJA_SHT3X_COMMAND_BREAK == sht3x::command::BREAK);
const _: () = assert!(PAMOJA_SHT3X_COMMAND_SOFT_RESET == sht3x::command::SOFT_RESET);
const _: () =
    assert!(PAMOJA_SHT3X_COMMAND_GENERAL_CALL_RESET == sht3x::command::GENERAL_CALL_RESET);
const _: () = assert!(PAMOJA_SHT3X_COMMAND_HEATER_ENABLE == sht3x::command::HEATER_ENABLE);
const _: () = assert!(PAMOJA_SHT3X_COMMAND_HEATER_DISABLE == sht3x::command::HEATER_DISABLE);
const _: () = assert!(PAMOJA_SHT3X_COMMAND_READ_STATUS == sht3x::command::READ_STATUS);
const _: () = assert!(PAMOJA_SHT3X_COMMAND_CLEAR_STATUS == sht3x::command::CLEAR_STATUS);

/// A decoded SHT3x temperature and humidity pair.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaSht3xMeasurement {
    /// The raw temperature word.
    pub temperature_raw: u16,
    /// The raw humidity word.
    pub humidity_raw: u16,
    /// The temperature in milli-degrees Celsius, exact in integer arithmetic.
    pub milli_celsius: i32,
    /// The temperature in degrees Celsius.
    pub celsius: f32,
    /// The temperature in milli-degrees Fahrenheit.
    pub milli_fahrenheit: i32,
    /// The temperature in degrees Fahrenheit.
    pub fahrenheit: f32,
    /// The relative humidity in milli-percent.
    pub milli_percent: u32,
    /// The relative humidity as a percentage.
    pub relative_humidity: f32,
}

/// A decoded SHT3x status register.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSht3xStatus {
    /// The 16-bit status word the flags were read from.
    pub bits: u16,
    /// `1` when at least one alert condition is pending.
    pub alert_pending: u8,
    /// `1` while the on-die heater is running.
    pub heater_on: u8,
    /// `1` when a humidity tracking alert is set.
    pub humidity_tracking_alert: u8,
    /// `1` when a temperature tracking alert is set.
    pub temperature_tracking_alert: u8,
    /// `1` when the part has reset since the flag was last cleared.
    pub reset_detected: u8,
    /// `1` when the last command could not be processed.
    pub command_failed: u8,
    /// `1` when the last write failed its checksum.
    pub write_checksum_failed: u8,
}

/// Computes the CRC-8 an SHT3x appends to every data word.
///
/// # Returns
///
/// The checksum over `data`, or 0 if the pointer is null with a non-zero length.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes, or be null when
/// `data_len` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_crc(data: *const u8, data_len: usize) -> u8 {
    match read_bytes(data, data_len) {
        Ok(data) => sht3x::crc(&data),
        Err(_) => 0,
    }
}

/// Reads a CRC-checked three-byte SHT3x word frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_word` set, or
/// [`PamojaStatus::Codec`] if the CRC does not match.
///
/// # Safety
///
/// `frame` must point to at least `frame_len` readable bytes, and `out_word` must
/// point to a writable `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_word(
    frame: *const u8,
    frame_len: usize,
    out_word: *mut u16,
) -> PamojaStatus {
    if out_word.is_null() {
        set_last_error("out_word must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let frame = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(frame) = <[u8; PAMOJA_SHT3X_WORD_LEN]>::try_from(&frame[..]) else {
        return wrong_length("word frame", PAMOJA_SHT3X_WORD_LEN);
    };
    match sht3x::word(&frame) {
        Ok(word) => {
            *out_word = word;
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Builds the three bytes an SHT3x sends for a word: the word then its CRC.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the three bytes written to `out_bytes`,
/// or [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least three writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_word_bytes(value: u16, out_bytes: *mut u8) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = sht3x::word_bytes(value);
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_SHT3X_WORD_LEN);
    PamojaStatus::Ok
}

/// Parses and CRC-checks a six-byte SHT3x measurement frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_measurement` filled in, or
/// [`PamojaStatus::Codec`] if either word fails its checksum, which means the read
/// was corrupted on the bus and should be repeated.
///
/// # Safety
///
/// `frame` must point to at least `frame_len` readable bytes, and
/// `out_measurement` must point to a writable `PamojaSht3xMeasurement`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_parse_measurement(
    frame: *const u8,
    frame_len: usize,
    out_measurement: *mut PamojaSht3xMeasurement,
) -> PamojaStatus {
    if out_measurement.is_null() {
        set_last_error("out_measurement must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let frame = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(frame) = <[u8; PAMOJA_SHT3X_MEASUREMENT_LEN]>::try_from(&frame[..]) else {
        return wrong_length("measurement frame", PAMOJA_SHT3X_MEASUREMENT_LEN);
    };
    match sht3x::Measurement::parse(&frame) {
        Ok(measurement) => {
            *out_measurement = measurement.into();
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Builds the six bytes an SHT3x sends for a pair of raw words.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the six bytes written to `out_bytes`, or
/// [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least six writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_measurement_bytes(
    temperature_raw: u16,
    humidity_raw: u16,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let measurement = sht3x::Measurement {
        temperature_raw,
        humidity_raw,
    };
    let bytes = measurement.to_bytes();
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_SHT3X_MEASUREMENT_LEN);
    PamojaStatus::Ok
}

/// Converts a raw SHT3x temperature word to milli-degrees Celsius.
///
/// # Returns
///
/// The temperature, exact in integer arithmetic.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_milli_celsius(raw: u16) -> i32 {
    sht3x::milli_celsius(raw)
}

/// Converts a raw SHT3x temperature word to degrees Celsius.
///
/// # Returns
///
/// The temperature.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_celsius(raw: u16) -> f32 {
    sht3x::celsius(raw)
}

/// Converts a raw SHT3x temperature word to milli-degrees Fahrenheit.
///
/// # Returns
///
/// The temperature, exact in integer arithmetic.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_milli_fahrenheit(raw: u16) -> i32 {
    sht3x::milli_fahrenheit(raw)
}

/// Converts a raw SHT3x temperature word to degrees Fahrenheit.
///
/// # Returns
///
/// The temperature.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_fahrenheit(raw: u16) -> f32 {
    sht3x::fahrenheit(raw)
}

/// Converts a raw SHT3x humidity word to milli-percent.
///
/// # Returns
///
/// The relative humidity, exact in integer arithmetic.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_milli_percent(raw: u16) -> u32 {
    sht3x::milli_percent(raw)
}

/// Converts a raw SHT3x humidity word to a relative humidity percentage.
///
/// # Returns
///
/// The relative humidity.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_relative_humidity(raw: u16) -> f32 {
    sht3x::relative_humidity(raw)
}

/// Builds the SHT3x temperature word that decodes to a temperature.
///
/// # Returns
///
/// The raw word, saturating at the ends of the part's range.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_temperature_raw_from_milli_celsius(milli_celsius: i32) -> u16 {
    sht3x::temperature_raw_from_milli_celsius(milli_celsius)
}

/// Builds the SHT3x temperature word that decodes to a temperature in Celsius.
///
/// # Returns
///
/// The raw word, saturating at the ends of the part's range.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_temperature_raw_from_celsius(celsius: f32) -> u16 {
    sht3x::temperature_raw_from_celsius(celsius)
}

/// Builds the SHT3x temperature word that decodes to a temperature in Fahrenheit.
///
/// # Returns
///
/// The raw word, saturating at the ends of the part's range.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_temperature_raw_from_milli_fahrenheit(milli_fahrenheit: i32) -> u16 {
    sht3x::temperature_raw_from_milli_fahrenheit(milli_fahrenheit)
}

/// Builds the SHT3x humidity word that decodes to a relative humidity.
///
/// # Returns
///
/// The raw word, saturating at full scale.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_humidity_raw_from_milli_percent(milli_percent: u32) -> u16 {
    sht3x::humidity_raw_from_milli_percent(milli_percent)
}

/// Builds the SHT3x humidity word that decodes to a relative humidity percentage.
///
/// # Returns
///
/// The raw word, saturating at full scale.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_humidity_raw_from_relative_humidity(percent: f32) -> u16 {
    sht3x::humidity_raw_from_relative_humidity(percent)
}

/// Parses and CRC-checks a three-byte SHT3x status frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_status` filled in, or
/// [`PamojaStatus::Codec`] if the CRC does not match.
///
/// # Safety
///
/// `frame` must point to at least `frame_len` readable bytes, and `out_status`
/// must point to a writable `PamojaSht3xStatus`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_parse_status(
    frame: *const u8,
    frame_len: usize,
    out_status: *mut PamojaSht3xStatus,
) -> PamojaStatus {
    if out_status.is_null() {
        set_last_error("out_status must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let frame = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(frame) = <[u8; PAMOJA_SHT3X_WORD_LEN]>::try_from(&frame[..]) else {
        return wrong_length("status frame", PAMOJA_SHT3X_WORD_LEN);
    };
    match sht3x::Status::parse(&frame) {
        Ok(status) => {
            *out_status = status.into();
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Splits an SHT3x status word into its flags.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_status` filled in. Every word decodes, so
/// this fails only on a null pointer.
///
/// # Safety
///
/// `out_status` must point to a writable `PamojaSht3xStatus`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_status_from_bits(
    bits: u16,
    out_status: *mut PamojaSht3xStatus,
) -> PamojaStatus {
    if out_status.is_null() {
        set_last_error("out_status must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_status = sht3x::Status::from_bits(bits).into();
    PamojaStatus::Ok
}

/// Builds the three bytes an SHT3x sends for a status word, CRC last.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the three bytes written to `out_bytes`,
/// or [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least three writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_status_bytes(bits: u16, out_bytes: *mut u8) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = sht3x::Status::from_bits(bits).to_bytes();
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_SHT3X_WORD_LEN);
    PamojaStatus::Ok
}

/// Returns the SHT3x single-shot command for a repeatability and clock mode.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_command` set, or
/// [`PamojaStatus::InvalidArgument`] if `repeatability` is not 0, 1, or 2.
///
/// # Safety
///
/// `out_command` must point to a writable `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_single_shot(
    repeatability: u8,
    clock_stretching: bool,
    out_command: *mut u16,
) -> PamojaStatus {
    if out_command.is_null() {
        set_last_error("out_command must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(repeatability) = repeatability_from_code(repeatability) else {
        return bad_repeatability();
    };
    *out_command = sht3x::single_shot(repeatability, clock_stretching);
    PamojaStatus::Ok
}

/// Returns the SHT3x periodic-mode command for a repeatability and rate.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_command` set, or
/// [`PamojaStatus::InvalidArgument`] if either code is outside its range.
///
/// # Safety
///
/// `out_command` must point to a writable `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_periodic(
    repeatability: u8,
    rate: u8,
    out_command: *mut u16,
) -> PamojaStatus {
    if out_command.is_null() {
        set_last_error("out_command must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(repeatability) = repeatability_from_code(repeatability) else {
        return bad_repeatability();
    };
    let Some(rate) = rate_from_code(rate) else {
        return bad_rate();
    };
    *out_command = sht3x::periodic(repeatability, rate);
    PamojaStatus::Ok
}

/// Returns how long an SHT3x measurement may take at a repeatability.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_micros` set to the datasheet's
/// worst case, or [`PamojaStatus::InvalidArgument`] if the code is out of range.
///
/// # Safety
///
/// `out_micros` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_max_measurement_micros(
    repeatability: u8,
    out_micros: *mut u32,
) -> PamojaStatus {
    if out_micros.is_null() {
        set_last_error("out_micros must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match repeatability_from_code(repeatability) {
        Some(repeatability) => {
            *out_micros = repeatability.max_measurement_micros();
            PamojaStatus::Ok
        }
        None => bad_repeatability(),
    }
}

/// Returns how long an SHT3x measurement typically takes at a repeatability.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_micros` set, or
/// [`PamojaStatus::InvalidArgument`] if the code is out of range.
///
/// # Safety
///
/// `out_micros` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_typical_measurement_micros(
    repeatability: u8,
    out_micros: *mut u32,
) -> PamojaStatus {
    if out_micros.is_null() {
        set_last_error("out_micros must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match repeatability_from_code(repeatability) {
        Some(repeatability) => {
            *out_micros = repeatability.typical_measurement_micros();
            PamojaStatus::Ok
        }
        None => bad_repeatability(),
    }
}

/// Returns the gap between SHT3x periodic measurements at a rate.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_micros` set, or
/// [`PamojaStatus::InvalidArgument`] if the code is out of range.
///
/// # Safety
///
/// `out_micros` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_interval_micros(
    rate: u8,
    out_micros: *mut u32,
) -> PamojaStatus {
    if out_micros.is_null() {
        set_last_error("out_micros must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match rate_from_code(rate) {
        Some(rate) => {
            *out_micros = rate.interval_micros();
            PamojaStatus::Ok
        }
        None => bad_rate(),
    }
}

/// The number of bytes in an SCD4x measurement frame, three words with their CRCs.
pub const PAMOJA_SCD4X_MEASUREMENT_LEN: usize = 9;

/// The number of bytes in an SCD4x word frame: the word then its CRC.
pub const PAMOJA_SCD4X_WORD_LEN: usize = 3;

/// The number of bytes in an SCD4x command frame.
pub const PAMOJA_SCD4X_COMMAND_LEN: usize = 2;

/// The number of bytes in an SCD4x write frame: a command, a word, and its CRC.
pub const PAMOJA_SCD4X_WRITE_LEN: usize = 5;

/// The single address an SCD4x answers on.
pub const PAMOJA_SCD4X_I2C_ADDRESS: u8 = 0x62;

/// The highest carbon dioxide concentration an SCD4x reports, in parts per million.
pub const PAMOJA_SCD4X_CO2_MAX_PPM: u16 = 40_000;

/// The temperature offset an SCD4x holds after a factory reset.
pub const PAMOJA_SCD4X_DEFAULT_TEMPERATURE_OFFSET_MILLI_CELSIUS: u32 = 4_000;

/// How often an SCD4x in periodic mode produces a result, in milliseconds.
pub const PAMOJA_SCD4X_PERIODIC_MEASUREMENT_INTERVAL_MS: u32 = 5_000;

/// How often it produces a result in low-power periodic mode, in milliseconds.
pub const PAMOJA_SCD4X_LOW_POWER_PERIODIC_MEASUREMENT_INTERVAL_MS: u32 = 30_000;

/// How long an SCD4x takes to become responsive after power-up, in milliseconds.
pub const PAMOJA_SCD4X_POWER_UP_TIME_MS: u32 = 1_000;

/// The word a forced recalibration returns when it did not take.
pub const PAMOJA_SCD4X_FORCED_RECALIBRATION_FAILED: u16 = 0xffff;

/// Starts periodic measurements at one result every five seconds.
pub const PAMOJA_SCD4X_COMMAND_START_PERIODIC_MEASUREMENT: u16 = 0x21b1;

/// Reads the latest carbon dioxide, temperature, and humidity words.
pub const PAMOJA_SCD4X_COMMAND_READ_MEASUREMENT: u16 = 0xec05;

/// Stops periodic measurements so other commands are accepted again.
pub const PAMOJA_SCD4X_COMMAND_STOP_PERIODIC_MEASUREMENT: u16 = 0x3f86;

/// Writes the temperature offset the part subtracts from its own reading.
pub const PAMOJA_SCD4X_COMMAND_SET_TEMPERATURE_OFFSET: u16 = 0x241d;

/// Reads the temperature offset back.
pub const PAMOJA_SCD4X_COMMAND_GET_TEMPERATURE_OFFSET: u16 = 0x2318;

/// Writes the altitude the part compensates its pressure for, in metres.
pub const PAMOJA_SCD4X_COMMAND_SET_SENSOR_ALTITUDE: u16 = 0x2427;

/// Reads the configured altitude back.
pub const PAMOJA_SCD4X_COMMAND_GET_SENSOR_ALTITUDE: u16 = 0x2322;

/// Writes the ambient pressure, which may be sent during a measurement.
pub const PAMOJA_SCD4X_COMMAND_SET_AMBIENT_PRESSURE: u16 = 0xe000;

/// Recalibrates against a known concentration and returns the correction applied.
pub const PAMOJA_SCD4X_COMMAND_PERFORM_FORCED_RECALIBRATION: u16 = 0x362f;

/// Turns automatic self-calibration on or off.
pub const PAMOJA_SCD4X_COMMAND_SET_AUTOMATIC_SELF_CALIBRATION_ENABLED: u16 = 0x2416;

/// Reads whether automatic self-calibration is on.
pub const PAMOJA_SCD4X_COMMAND_GET_AUTOMATIC_SELF_CALIBRATION_ENABLED: u16 = 0x2313;

/// Starts low-power periodic measurements, one result every thirty seconds.
pub const PAMOJA_SCD4X_COMMAND_START_LOW_POWER_PERIODIC_MEASUREMENT: u16 = 0x21ac;

/// Reads whether a fresh result is waiting.
pub const PAMOJA_SCD4X_COMMAND_GET_DATA_READY_STATUS: u16 = 0xe4b8;

/// Stores the current settings in non-volatile memory.
pub const PAMOJA_SCD4X_COMMAND_PERSIST_SETTINGS: u16 = 0x3615;

/// Reads the 48-bit serial number, three words with their CRCs.
pub const PAMOJA_SCD4X_COMMAND_GET_SERIAL_NUMBER: u16 = 0x3682;

/// Runs the on-board self test, which takes ten seconds.
pub const PAMOJA_SCD4X_COMMAND_PERFORM_SELF_TEST: u16 = 0x3639;

/// Restores the factory settings, discarding the stored calibration.
pub const PAMOJA_SCD4X_COMMAND_PERFORM_FACTORY_RESET: u16 = 0x3632;

/// Reloads the stored settings without a power cycle.
pub const PAMOJA_SCD4X_COMMAND_REINIT: u16 = 0x3646;

/// Takes one measurement on demand, an SCD41 command.
pub const PAMOJA_SCD4X_COMMAND_MEASURE_SINGLE_SHOT: u16 = 0x219d;

/// Takes one humidity and temperature measurement without the photoacoustic cell.
pub const PAMOJA_SCD4X_COMMAND_MEASURE_SINGLE_SHOT_RHT_ONLY: u16 = 0x2196;

/// Puts an SCD41 into its lowest-power state.
pub const PAMOJA_SCD4X_COMMAND_POWER_DOWN: u16 = 0x36e0;

/// Brings an SCD41 back out of power-down.
pub const PAMOJA_SCD4X_COMMAND_WAKE_UP: u16 = 0x36f6;

// The header generator does not read the crates this one depends on, so these
// carry their value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_SCD4X_I2C_ADDRESS == scd4x::I2C_ADDRESS);
const _: () = assert!(PAMOJA_SCD4X_CO2_MAX_PPM == scd4x::CO2_MAX_PPM);
const _: () = assert!(
    PAMOJA_SCD4X_DEFAULT_TEMPERATURE_OFFSET_MILLI_CELSIUS
        == scd4x::DEFAULT_TEMPERATURE_OFFSET_MILLI_CELSIUS
);
const _: () = assert!(
    PAMOJA_SCD4X_PERIODIC_MEASUREMENT_INTERVAL_MS == scd4x::PERIODIC_MEASUREMENT_INTERVAL_MS
);
const _: () = assert!(
    PAMOJA_SCD4X_LOW_POWER_PERIODIC_MEASUREMENT_INTERVAL_MS
        == scd4x::LOW_POWER_PERIODIC_MEASUREMENT_INTERVAL_MS
);
const _: () = assert!(PAMOJA_SCD4X_POWER_UP_TIME_MS == scd4x::POWER_UP_TIME_MS);
const _: () =
    assert!(PAMOJA_SCD4X_FORCED_RECALIBRATION_FAILED == scd4x::FORCED_RECALIBRATION_FAILED);
const _: () = assert!(
    PAMOJA_SCD4X_COMMAND_START_PERIODIC_MEASUREMENT == scd4x::command::START_PERIODIC_MEASUREMENT
);
const _: () = assert!(PAMOJA_SCD4X_COMMAND_READ_MEASUREMENT == scd4x::command::READ_MEASUREMENT);
const _: () = assert!(
    PAMOJA_SCD4X_COMMAND_STOP_PERIODIC_MEASUREMENT == scd4x::command::STOP_PERIODIC_MEASUREMENT
);
const _: () =
    assert!(PAMOJA_SCD4X_COMMAND_SET_TEMPERATURE_OFFSET == scd4x::command::SET_TEMPERATURE_OFFSET);
const _: () =
    assert!(PAMOJA_SCD4X_COMMAND_GET_TEMPERATURE_OFFSET == scd4x::command::GET_TEMPERATURE_OFFSET);
const _: () =
    assert!(PAMOJA_SCD4X_COMMAND_SET_SENSOR_ALTITUDE == scd4x::command::SET_SENSOR_ALTITUDE);
const _: () =
    assert!(PAMOJA_SCD4X_COMMAND_GET_SENSOR_ALTITUDE == scd4x::command::GET_SENSOR_ALTITUDE);
const _: () =
    assert!(PAMOJA_SCD4X_COMMAND_SET_AMBIENT_PRESSURE == scd4x::command::SET_AMBIENT_PRESSURE);
const _: () = assert!(
    PAMOJA_SCD4X_COMMAND_PERFORM_FORCED_RECALIBRATION
        == scd4x::command::PERFORM_FORCED_RECALIBRATION
);
const _: () = assert!(
    PAMOJA_SCD4X_COMMAND_SET_AUTOMATIC_SELF_CALIBRATION_ENABLED
        == scd4x::command::SET_AUTOMATIC_SELF_CALIBRATION_ENABLED
);
const _: () = assert!(
    PAMOJA_SCD4X_COMMAND_GET_AUTOMATIC_SELF_CALIBRATION_ENABLED
        == scd4x::command::GET_AUTOMATIC_SELF_CALIBRATION_ENABLED
);
const _: () = assert!(
    PAMOJA_SCD4X_COMMAND_START_LOW_POWER_PERIODIC_MEASUREMENT
        == scd4x::command::START_LOW_POWER_PERIODIC_MEASUREMENT
);
const _: () =
    assert!(PAMOJA_SCD4X_COMMAND_GET_DATA_READY_STATUS == scd4x::command::GET_DATA_READY_STATUS);
const _: () = assert!(PAMOJA_SCD4X_COMMAND_PERSIST_SETTINGS == scd4x::command::PERSIST_SETTINGS);
const _: () = assert!(PAMOJA_SCD4X_COMMAND_GET_SERIAL_NUMBER == scd4x::command::GET_SERIAL_NUMBER);
const _: () = assert!(PAMOJA_SCD4X_COMMAND_PERFORM_SELF_TEST == scd4x::command::PERFORM_SELF_TEST);
const _: () =
    assert!(PAMOJA_SCD4X_COMMAND_PERFORM_FACTORY_RESET == scd4x::command::PERFORM_FACTORY_RESET);
const _: () = assert!(PAMOJA_SCD4X_COMMAND_REINIT == scd4x::command::REINIT);
const _: () =
    assert!(PAMOJA_SCD4X_COMMAND_MEASURE_SINGLE_SHOT == scd4x::command::MEASURE_SINGLE_SHOT);
const _: () = assert!(
    PAMOJA_SCD4X_COMMAND_MEASURE_SINGLE_SHOT_RHT_ONLY
        == scd4x::command::MEASURE_SINGLE_SHOT_RHT_ONLY
);
const _: () = assert!(PAMOJA_SCD4X_COMMAND_POWER_DOWN == scd4x::command::POWER_DOWN);
const _: () = assert!(PAMOJA_SCD4X_COMMAND_WAKE_UP == scd4x::command::WAKE_UP);

/// A decoded SCD4x measurement frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaScd4xMeasurement {
    /// The carbon dioxide concentration in parts per million.
    pub co2_ppm: u16,
    /// The raw temperature word.
    pub temperature_raw: u16,
    /// The raw humidity word.
    pub humidity_raw: u16,
    /// The temperature in milli-degrees Celsius, exact in integer arithmetic.
    pub milli_celsius: i32,
    /// The temperature in degrees Celsius.
    pub celsius: f32,
    /// The relative humidity in milli-percent.
    pub humidity_milli_percent: u32,
    /// The relative humidity as a percentage.
    pub relative_humidity_percent: f32,
}

/// Computes the CRC-8 an SCD4x appends to every data word.
///
/// # Returns
///
/// The checksum over `data`, or 0 if the pointer is null with a non-zero length.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes, or be null when
/// `data_len` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_crc(data: *const u8, data_len: usize) -> u8 {
    match read_bytes(data, data_len) {
        Ok(data) => scd4x::crc(&data),
        Err(_) => 0,
    }
}

/// Reads a CRC-checked three-byte SCD4x word frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_word` set, or
/// [`PamojaStatus::Codec`] if the CRC does not match.
///
/// # Safety
///
/// `frame` must point to at least `frame_len` readable bytes, and `out_word` must
/// point to a writable `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_word(
    frame: *const u8,
    frame_len: usize,
    out_word: *mut u16,
) -> PamojaStatus {
    if out_word.is_null() {
        set_last_error("out_word must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let frame = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(frame) = <[u8; PAMOJA_SCD4X_WORD_LEN]>::try_from(&frame[..]) else {
        return wrong_length("word frame", PAMOJA_SCD4X_WORD_LEN);
    };
    match scd4x::word(&frame) {
        Ok(word) => {
            *out_word = word;
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Builds the three bytes an SCD4x sends for a word: the word then its CRC.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the three bytes written to `out_bytes`,
/// or [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least three writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_word_frame(value: u16, out_bytes: *mut u8) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = scd4x::word_frame(value);
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_SCD4X_WORD_LEN);
    PamojaStatus::Ok
}

/// Builds the two bytes that address an SCD4x command, most significant first.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the two bytes written to `out_bytes`, or
/// [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least two writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_command_frame(
    command: u16,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = scd4x::command_frame(command);
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_SCD4X_COMMAND_LEN);
    PamojaStatus::Ok
}

/// Builds the five bytes that write a word to an SCD4x: command, word, CRC.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the five bytes written to `out_bytes`,
/// or [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least five writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_write_frame(
    command: u16,
    value: u16,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = scd4x::write_frame(command, value);
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_SCD4X_WRITE_LEN);
    PamojaStatus::Ok
}

/// Returns how long an SCD4x command may take before its result can be read.
///
/// # Returns
///
/// `true` when the command has a documented execution time, with `*out_millis`
/// set to it; `false` when it completes as soon as it is acknowledged.
///
/// # Safety
///
/// `out_millis` must point to a writable `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_max_duration_ms(command: u16, out_millis: *mut u16) -> bool {
    match scd4x::max_duration_ms(command) {
        Some(millis) if !out_millis.is_null() => {
            *out_millis = millis;
            true
        }
        _ => false,
    }
}

/// Reports whether an SCD4x accepts a command while it is measuring.
///
/// # Returns
///
/// `true` when the command may be sent without stopping periodic measurements.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_allowed_during_measurement(command: u16) -> bool {
    scd4x::allowed_during_measurement(command)
}

/// Parses and CRC-checks a nine-byte SCD4x measurement frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_measurement` filled in, or
/// [`PamojaStatus::Codec`] if any word fails its checksum, which means the read
/// was corrupted on the bus and should be repeated.
///
/// # Safety
///
/// `frame` must point to at least `frame_len` readable bytes, and
/// `out_measurement` must point to a writable `PamojaScd4xMeasurement`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_parse_measurement(
    frame: *const u8,
    frame_len: usize,
    out_measurement: *mut PamojaScd4xMeasurement,
) -> PamojaStatus {
    if out_measurement.is_null() {
        set_last_error("out_measurement must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let frame = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(frame) = <[u8; PAMOJA_SCD4X_MEASUREMENT_LEN]>::try_from(&frame[..]) else {
        return wrong_length("measurement frame", PAMOJA_SCD4X_MEASUREMENT_LEN);
    };
    match scd4x::Measurement::parse(&frame) {
        Ok(measurement) => {
            *out_measurement = measurement.into();
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Builds the SCD4x measurement a sensor reporting these physical values would send.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_measurement` filled in, or
/// [`PamojaStatus::InvalidArgument`] if the pointer is null.
///
/// # Safety
///
/// `out_measurement` must point to a writable `PamojaScd4xMeasurement`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_measurement_from_physical(
    co2_ppm: u16,
    milli_celsius: i32,
    humidity_milli_percent: u32,
    out_measurement: *mut PamojaScd4xMeasurement,
) -> PamojaStatus {
    if out_measurement.is_null() {
        set_last_error("out_measurement must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let measurement =
        scd4x::Measurement::from_physical(co2_ppm, milli_celsius, humidity_milli_percent);
    *out_measurement = measurement.into();
    PamojaStatus::Ok
}

/// Builds the nine bytes an SCD4x sends for a set of raw words, each CRC included.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the nine bytes written to `out_bytes`,
/// or [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least nine writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_measurement_bytes(
    co2_ppm: u16,
    temperature_raw: u16,
    humidity_raw: u16,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let measurement = scd4x::Measurement {
        co2_ppm,
        temperature_raw,
        humidity_raw,
    };
    let bytes = measurement.to_bytes();
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_SCD4X_MEASUREMENT_LEN);
    PamojaStatus::Ok
}

/// Converts a raw SCD4x temperature word to milli-degrees Celsius.
///
/// # Returns
///
/// The temperature, exact in integer arithmetic.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_milli_celsius(raw: u16) -> i32 {
    scd4x::milli_celsius(raw)
}

/// Converts a raw SCD4x temperature word to degrees Celsius.
///
/// # Returns
///
/// The temperature.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_celsius(raw: u16) -> f32 {
    scd4x::celsius(raw)
}

/// Builds the SCD4x temperature word that decodes to a temperature.
///
/// # Returns
///
/// The raw word, saturating at the ends of the part's range.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_temperature_raw(milli_celsius: i32) -> u16 {
    scd4x::temperature_raw(milli_celsius)
}

/// Converts a raw SCD4x humidity word to milli-percent.
///
/// # Returns
///
/// The relative humidity, exact in integer arithmetic.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_humidity_milli_percent(raw: u16) -> u32 {
    scd4x::humidity_milli_percent(raw)
}

/// Converts a raw SCD4x humidity word to a relative humidity percentage.
///
/// # Returns
///
/// The relative humidity.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_relative_humidity_percent(raw: u16) -> f32 {
    scd4x::relative_humidity_percent(raw)
}

/// Builds the SCD4x humidity word that decodes to a relative humidity.
///
/// # Returns
///
/// The raw word, saturating at full scale.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_humidity_raw(milli_percent: u32) -> u16 {
    scd4x::humidity_raw(milli_percent)
}

/// Reports whether an SCD4x data-ready word says a fresh result is waiting.
///
/// # Returns
///
/// `true` when any of the low eleven bits is set.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_data_ready(word: u16) -> bool {
    scd4x::data_ready(word)
}

/// Builds the SCD4x temperature-offset word for an offset.
///
/// # Returns
///
/// The word to write, which scales by 2^16 rather than by the 2^16 - 1 the
/// measurement words use.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_temperature_offset_word(milli_celsius: u32) -> u16 {
    scd4x::temperature_offset_word(milli_celsius)
}

/// Reads an SCD4x temperature-offset word back as an offset.
///
/// # Returns
///
/// The offset in milli-degrees Celsius.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_temperature_offset_milli_celsius(word: u16) -> u32 {
    scd4x::temperature_offset_milli_celsius(word)
}

/// Builds the SCD4x ambient-pressure word for a pressure.
///
/// # Returns
///
/// The word to write, at 100 pascals per count.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_ambient_pressure_word(pascals: u32) -> u16 {
    scd4x::ambient_pressure_word(pascals)
}

/// Reads an SCD4x ambient-pressure word back as a pressure.
///
/// # Returns
///
/// The pressure in pascals.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_ambient_pressure_pascals(word: u16) -> u32 {
    scd4x::ambient_pressure_pascals(word)
}

/// Reads the correction a forced recalibration applied.
///
/// # Returns
///
/// `true` when the recalibration took, with `*out_ppm` set to the correction in
/// parts per million; `false` when the part reported that it failed.
///
/// # Safety
///
/// `out_ppm` must point to a writable `int32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_forced_recalibration_correction_ppm(
    word: u16,
    out_ppm: *mut i32,
) -> bool {
    match scd4x::forced_recalibration_correction_ppm(word) {
        Some(ppm) if !out_ppm.is_null() => {
            *out_ppm = ppm;
            true
        }
        _ => false,
    }
}

/// Builds the word an SCD4x returns for a forced-recalibration outcome.
///
/// # Returns
///
/// The word, which is the failure sentinel when `succeeded` is `false`.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_forced_recalibration_word(
    succeeded: bool,
    correction_ppm: i32,
) -> u16 {
    scd4x::forced_recalibration_word(succeeded.then_some(correction_ppm))
}

/// Reports whether an SCD4x word says automatic self-calibration is on.
///
/// # Returns
///
/// `true` when the part recalibrates itself against clean air.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_automatic_self_calibration_enabled(word: u16) -> bool {
    scd4x::automatic_self_calibration_enabled(word)
}

/// Builds the SCD4x word that turns automatic self-calibration on or off.
///
/// # Returns
///
/// The word to write.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_automatic_self_calibration_word(enabled: bool) -> u16 {
    scd4x::automatic_self_calibration_word(enabled)
}

/// Reports whether an SCD4x self-test word says the part is healthy.
///
/// # Returns
///
/// `true` when the self test found no malfunction.
#[no_mangle]
pub extern "C" fn pamoja_scd4x_self_test_passed(word: u16) -> bool {
    scd4x::self_test_passed(word)
}

/// Reads the 48-bit serial number out of a nine-byte SCD4x frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_serial` set, or
/// [`PamojaStatus::Codec`] if any word fails its checksum.
///
/// # Safety
///
/// `frame` must point to at least `frame_len` readable bytes, and `out_serial`
/// must point to a writable `uint64_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_serial_number(
    frame: *const u8,
    frame_len: usize,
    out_serial: *mut u64,
) -> PamojaStatus {
    if out_serial.is_null() {
        set_last_error("out_serial must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let frame = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(frame) = <[u8; PAMOJA_SCD4X_MEASUREMENT_LEN]>::try_from(&frame[..]) else {
        return wrong_length("serial number frame", PAMOJA_SCD4X_MEASUREMENT_LEN);
    };
    match scd4x::serial_number(&frame) {
        Ok(serial) => {
            *out_serial = serial;
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Builds the nine bytes an SCD4x sends for a serial number, each CRC included.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the nine bytes written to `out_bytes`,
/// or [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least nine writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_serial_number_frame(
    serial: u64,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = scd4x::serial_number_frame(serial);
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_SCD4X_MEASUREMENT_LEN);
    PamojaStatus::Ok
}

/// The number of bytes in a TMP117 register read.
pub const PAMOJA_TMP117_REGISTER_LEN: usize = 2;

/// The value a TMP117's device-ID register reads, which confirms the part.
pub const PAMOJA_TMP117_DEVICE_ID: u16 = 0x0117;

/// The value its configuration register reads after a reset.
pub const PAMOJA_TMP117_CONFIG_RESET: u16 = 0x0220;

/// The value its high-limit register reads after a reset.
pub const PAMOJA_TMP117_HIGH_LIMIT_RESET: u16 = 0x6000;

/// The value its low-limit register reads after a reset.
pub const PAMOJA_TMP117_LOW_LIMIT_RESET: u16 = 0x8000;

/// The value its result register reads before the first conversion completes.
pub const PAMOJA_TMP117_TEMP_RESULT_RESET: u16 = 0x8000;

/// The byte a general-call reset sends to address 0x00.
pub const PAMOJA_TMP117_GENERAL_CALL_RESET: u8 = 0x06;

/// The word written to the EEPROM unlock register to allow a write.
pub const PAMOJA_TMP117_EEPROM_UNLOCK: u16 = 0x8000;

/// The address a TMP117 answers on with ADD0 tied to GND.
pub const PAMOJA_TMP117_ADDRESS_ADD0_GND: u8 = 0x48;

/// The address it answers on with ADD0 tied to V+.
pub const PAMOJA_TMP117_ADDRESS_ADD0_VPLUS: u8 = 0x49;

/// The address it answers on with ADD0 tied to SDA.
pub const PAMOJA_TMP117_ADDRESS_ADD0_SDA: u8 = 0x4A;

/// The address it answers on with ADD0 tied to SCL.
pub const PAMOJA_TMP117_ADDRESS_ADD0_SCL: u8 = 0x4B;

/// The TMP117 temperature result register.
pub const PAMOJA_TMP117_REGISTER_TEMP_RESULT: u8 = 0x00;

/// The TMP117 configuration register.
pub const PAMOJA_TMP117_REGISTER_CONFIGURATION: u8 = 0x01;

/// The TMP117 high-limit register.
pub const PAMOJA_TMP117_REGISTER_THIGH_LIMIT: u8 = 0x02;

/// The TMP117 low-limit register.
pub const PAMOJA_TMP117_REGISTER_TLOW_LIMIT: u8 = 0x03;

/// The TMP117 EEPROM unlock register.
pub const PAMOJA_TMP117_REGISTER_EEPROM_UL: u8 = 0x04;

/// The first TMP117 general-purpose EEPROM register.
pub const PAMOJA_TMP117_REGISTER_EEPROM1: u8 = 0x05;

/// The second TMP117 general-purpose EEPROM register.
pub const PAMOJA_TMP117_REGISTER_EEPROM2: u8 = 0x06;

/// The TMP117 temperature offset register.
pub const PAMOJA_TMP117_REGISTER_TEMP_OFFSET: u8 = 0x07;

/// The third TMP117 general-purpose EEPROM register.
pub const PAMOJA_TMP117_REGISTER_EEPROM3: u8 = 0x08;

/// The TMP117 device-ID register.
pub const PAMOJA_TMP117_REGISTER_DEVICE_ID: u8 = 0x0F;

// The header generator does not read the crates this one depends on, so these
// carry their value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_TMP117_DEVICE_ID == tmp117::DEVICE_ID);
const _: () = assert!(PAMOJA_TMP117_CONFIG_RESET == tmp117::CONFIG_RESET);
const _: () = assert!(PAMOJA_TMP117_HIGH_LIMIT_RESET == tmp117::HIGH_LIMIT_RESET);
const _: () = assert!(PAMOJA_TMP117_LOW_LIMIT_RESET == tmp117::LOW_LIMIT_RESET);
const _: () = assert!(PAMOJA_TMP117_TEMP_RESULT_RESET == tmp117::TEMP_RESULT_RESET);
const _: () = assert!(PAMOJA_TMP117_GENERAL_CALL_RESET == tmp117::GENERAL_CALL_RESET);
const _: () = assert!(PAMOJA_TMP117_EEPROM_UNLOCK == tmp117::EEPROM_UNLOCK);
const _: () = assert!(PAMOJA_TMP117_ADDRESS_ADD0_GND == tmp117::address::ADD0_GND);
const _: () = assert!(PAMOJA_TMP117_ADDRESS_ADD0_VPLUS == tmp117::address::ADD0_VPLUS);
const _: () = assert!(PAMOJA_TMP117_ADDRESS_ADD0_SDA == tmp117::address::ADD0_SDA);
const _: () = assert!(PAMOJA_TMP117_ADDRESS_ADD0_SCL == tmp117::address::ADD0_SCL);
const _: () = assert!(PAMOJA_TMP117_REGISTER_TEMP_RESULT == tmp117::register::TEMP_RESULT);
const _: () = assert!(PAMOJA_TMP117_REGISTER_CONFIGURATION == tmp117::register::CONFIGURATION);
const _: () = assert!(PAMOJA_TMP117_REGISTER_THIGH_LIMIT == tmp117::register::THIGH_LIMIT);
const _: () = assert!(PAMOJA_TMP117_REGISTER_TLOW_LIMIT == tmp117::register::TLOW_LIMIT);
const _: () = assert!(PAMOJA_TMP117_REGISTER_EEPROM_UL == tmp117::register::EEPROM_UL);
const _: () = assert!(PAMOJA_TMP117_REGISTER_EEPROM1 == tmp117::register::EEPROM1);
const _: () = assert!(PAMOJA_TMP117_REGISTER_EEPROM2 == tmp117::register::EEPROM2);
const _: () = assert!(PAMOJA_TMP117_REGISTER_TEMP_OFFSET == tmp117::register::TEMP_OFFSET);
const _: () = assert!(PAMOJA_TMP117_REGISTER_EEPROM3 == tmp117::register::EEPROM3);
const _: () = assert!(PAMOJA_TMP117_REGISTER_DEVICE_ID == tmp117::register::DEVICE_ID);

/// A TMP117 configuration register, field by field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaTmp117Config {
    /// `1` when a result went above the high limit.
    pub high_alert: u8,
    /// `1` when a result went below the low limit.
    pub low_alert: u8,
    /// `1` when a conversion has completed since the register was last read.
    pub data_ready: u8,
    /// `1` while an EEPROM write is still in progress.
    pub eeprom_busy: u8,
    /// The conversion-mode code: `0` continuous, `1` shutdown, `3` one-shot.
    pub mode: u8,
    /// The conversion-cycle code, `0..=7`.
    pub cycle: u8,
    /// The averaging code, `0..=3`.
    pub averaging: u8,
    /// `1` makes the limits a therm hysteresis band rather than alerts.
    pub therm_mode: u8,
    /// `1` makes the ALERT pin active high.
    pub alert_active_high: u8,
    /// `1` makes the ALERT pin reflect data ready rather than the alert flags.
    pub alert_pin_data_ready: u8,
    /// `1` triggers a software reset when this register is written.
    pub soft_reset: u8,
}

/// Converts a raw TMP117 temperature register to nano-degrees Celsius.
///
/// # Returns
///
/// The temperature, exact in integer arithmetic at the part's 7.8125 m°C step.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_nano_celsius(raw: i16) -> i64 {
    tmp117::nano_celsius(raw)
}

/// Converts a raw TMP117 temperature register to micro-degrees Celsius.
///
/// # Returns
///
/// The temperature, truncated toward zero at the last digit.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_micro_celsius(raw: i16) -> i32 {
    tmp117::micro_celsius(raw)
}

/// Converts a raw TMP117 temperature register to degrees Celsius.
///
/// # Returns
///
/// The temperature.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_celsius(raw: i16) -> f32 {
    tmp117::celsius(raw)
}

/// Builds the TMP117 temperature register that decodes to a temperature.
///
/// # Returns
///
/// The nearest register value, saturating at the ends of the part's range.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_raw_from_micro_celsius(micro_celsius: i32) -> i16 {
    tmp117::raw_from_micro_celsius(micro_celsius)
}

/// Builds the TMP117 temperature register that decodes to a temperature in Celsius.
///
/// # Returns
///
/// The nearest register value, saturating at the ends of the part's range.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_raw_from_celsius(celsius: f32) -> i16 {
    tmp117::raw_from_celsius(celsius)
}

/// Builds the two bytes a TMP117 sends for a temperature register.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the two bytes written to `out_bytes`, or
/// [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least two writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_tmp117_temperature_bytes(
    raw: i16,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = tmp117::temperature_bytes(raw);
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_TMP117_REGISTER_LEN);
    PamojaStatus::Ok
}

/// Reads the two bytes a TMP117 sends for a temperature register.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_raw` set, or
/// [`PamojaStatus::InvalidArgument`] if the buffer is not two bytes.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes, and `out_raw` must
/// point to a writable `int16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_tmp117_temperature_from_bytes(
    bytes: *const u8,
    bytes_len: usize,
    out_raw: *mut i16,
) -> PamojaStatus {
    if out_raw.is_null() {
        set_last_error("out_raw must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(bytes) = <[u8; PAMOJA_TMP117_REGISTER_LEN]>::try_from(&bytes[..]) else {
        return wrong_length("temperature register", PAMOJA_TMP117_REGISTER_LEN);
    };
    *out_raw = tmp117::temperature_from_bytes(bytes);
    PamojaStatus::Ok
}

/// Reads the device identifier out of a TMP117 device-ID register.
///
/// # Returns
///
/// The low twelve bits, which are 0x117 for a TMP117.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_device_id(raw: u16) -> u16 {
    tmp117::device_id(raw)
}

/// Reads the die revision out of a TMP117 device-ID register.
///
/// # Returns
///
/// The high four bits.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_revision(raw: u16) -> u8 {
    tmp117::revision(raw)
}

/// Reports whether a TMP117 configuration register flags a high alert.
///
/// # Returns
///
/// `true` when a result went above the high limit.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_high_alert(config: u16) -> bool {
    tmp117::high_alert(config)
}

/// Reports whether a TMP117 configuration register flags a low alert.
///
/// # Returns
///
/// `true` when a result went below the low limit.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_low_alert(config: u16) -> bool {
    tmp117::low_alert(config)
}

/// Reports whether a TMP117 configuration register says a result is ready.
///
/// # Returns
///
/// `true` when a conversion completed since the register was last read.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_data_ready(config: u16) -> bool {
    tmp117::data_ready(config)
}

/// Reports whether a TMP117 configuration register says an EEPROM write is running.
///
/// # Returns
///
/// `true` while the write is in progress.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_eeprom_busy(config: u16) -> bool {
    tmp117::eeprom_busy(config)
}

/// Reports whether a TMP117 EEPROM unlock register says a write is running.
///
/// # Returns
///
/// `true` while the write is in progress.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_eeprom_unlock_busy(unlock: u16) -> bool {
    tmp117::eeprom_unlock_busy(unlock)
}

/// Assembles the 16-bit TMP117 configuration register value.
///
/// # Returns
///
/// The register value to write.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_config_bits(config: PamojaTmp117Config) -> u16 {
    tmp117::Configuration::from(config).bits()
}

/// Parses a 16-bit TMP117 configuration register value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_config` filled in. Every register value
/// decodes, so this fails only on a null pointer.
///
/// # Safety
///
/// `out_config` must point to a writable `PamojaTmp117Config`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_tmp117_config_from_bits(
    bits: u16,
    out_config: *mut PamojaTmp117Config,
) -> PamojaStatus {
    if out_config.is_null() {
        set_last_error("out_config must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_config = tmp117::Configuration::from_bits(bits).into();
    PamojaStatus::Ok
}

/// Returns how many conversions a TMP117 averaging code folds into one result.
///
/// # Returns
///
/// The conversion count: 1, 8, 32, or 64.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_averaging_conversions(code: u8) -> u8 {
    tmp117::Averaging::from_code(code).conversions()
}

/// Returns how long a TMP117 averaging code takes to convert.
///
/// # Returns
///
/// The conversion time in microseconds.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_averaging_micros(code: u8) -> u32 {
    tmp117::Averaging::from_code(code).conversion_micros()
}

/// Returns the nominal cycle a TMP117 conversion-cycle code selects.
///
/// # Returns
///
/// The cycle in microseconds, before the averaging setting extends it.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_cycle_nominal_micros(code: u8) -> u32 {
    tmp117::ConversionCycle::from_code(code).nominal_micros()
}

/// Returns how often a TMP117 updates its result for a cycle and averaging code.
///
/// # Returns
///
/// The longer of the nominal cycle and the time the averaging takes.
#[no_mangle]
pub extern "C" fn pamoja_tmp117_cycle_micros(cycle: u8, averaging: u8) -> u32 {
    let averaging = tmp117::Averaging::from_code(averaging);
    tmp117::ConversionCycle::from_code(cycle).cycle_micros(averaging)
}

/// The number of bytes an HDC1080 sequential read returns.
pub const PAMOJA_HDC1080_MEASUREMENT_LEN: usize = 4;

/// The number of serial-ID registers an HDC1080 carries.
pub const PAMOJA_HDC1080_SERIAL_ID_REGISTERS: usize = 3;

/// The single address an HDC1080 answers on.
pub const PAMOJA_HDC1080_I2C_ADDRESS: u8 = 0x40;

/// The value its manufacturer-ID register reads: TI.
pub const PAMOJA_HDC1080_MANUFACTURER_ID: u16 = 0x5449;

/// The value its device-ID register reads, which confirms the part.
pub const PAMOJA_HDC1080_DEVICE_ID: u16 = 0x1050;

/// The value its configuration register reads after a reset.
pub const PAMOJA_HDC1080_CONFIGURATION_RESET: u16 = 0x1000;

/// The HDC1080 temperature register.
pub const PAMOJA_HDC1080_REGISTER_TEMPERATURE: u8 = 0x00;

/// The HDC1080 humidity register.
pub const PAMOJA_HDC1080_REGISTER_HUMIDITY: u8 = 0x01;

/// The HDC1080 configuration register.
pub const PAMOJA_HDC1080_REGISTER_CONFIGURATION: u8 = 0x02;

/// The high word of the HDC1080 serial ID.
pub const PAMOJA_HDC1080_REGISTER_SERIAL_ID_HIGH: u8 = 0xFB;

/// The middle word of the HDC1080 serial ID.
pub const PAMOJA_HDC1080_REGISTER_SERIAL_ID_MID: u8 = 0xFC;

/// The low word of the HDC1080 serial ID.
pub const PAMOJA_HDC1080_REGISTER_SERIAL_ID_LOW: u8 = 0xFD;

/// The HDC1080 manufacturer-ID register.
pub const PAMOJA_HDC1080_REGISTER_MANUFACTURER_ID: u8 = 0xFE;

/// The HDC1080 device-ID register.
pub const PAMOJA_HDC1080_REGISTER_DEVICE_ID: u8 = 0xFF;

// The header generator does not read the crates this one depends on, so these
// carry their value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_HDC1080_I2C_ADDRESS == hdc1080::I2C_ADDRESS);
const _: () = assert!(PAMOJA_HDC1080_MANUFACTURER_ID == hdc1080::MANUFACTURER_ID);
const _: () = assert!(PAMOJA_HDC1080_DEVICE_ID == hdc1080::DEVICE_ID);
const _: () = assert!(PAMOJA_HDC1080_CONFIGURATION_RESET == hdc1080::CONFIGURATION_RESET);
const _: () = assert!(PAMOJA_HDC1080_REGISTER_TEMPERATURE == hdc1080::register::TEMPERATURE);
const _: () = assert!(PAMOJA_HDC1080_REGISTER_HUMIDITY == hdc1080::register::HUMIDITY);
const _: () = assert!(PAMOJA_HDC1080_REGISTER_CONFIGURATION == hdc1080::register::CONFIGURATION);
const _: () = assert!(PAMOJA_HDC1080_REGISTER_SERIAL_ID_HIGH == hdc1080::register::SERIAL_ID_HIGH);
const _: () = assert!(PAMOJA_HDC1080_REGISTER_SERIAL_ID_MID == hdc1080::register::SERIAL_ID_MID);
const _: () = assert!(PAMOJA_HDC1080_REGISTER_SERIAL_ID_LOW == hdc1080::register::SERIAL_ID_LOW);
const _: () =
    assert!(PAMOJA_HDC1080_REGISTER_MANUFACTURER_ID == hdc1080::register::MANUFACTURER_ID);
const _: () = assert!(PAMOJA_HDC1080_REGISTER_DEVICE_ID == hdc1080::register::DEVICE_ID);

/// A decoded HDC1080 temperature and humidity pair.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaHdc1080Measurement {
    /// The raw temperature register.
    pub temperature_raw: u16,
    /// The raw humidity register.
    pub humidity_raw: u16,
    /// The temperature in milli-degrees Celsius, exact in integer arithmetic.
    pub milli_celsius: i32,
    /// The temperature in degrees Celsius.
    pub celsius: f32,
    /// The relative humidity in milli-percent.
    pub milli_percent: u32,
    /// The relative humidity as a percentage.
    pub relative_humidity: f32,
}

/// An HDC1080 configuration register, field by field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaHdc1080Config {
    /// `1` resets the part when this register is written.
    pub software_reset: u8,
    /// `1` runs the on-die heater during measurements.
    pub heater: u8,
    /// `1` acquires temperature and humidity from one trigger.
    pub sequential: u8,
    /// `1` when the supply has dropped below 2.8 V, which the part reports back.
    pub battery_low: u8,
    /// The temperature resolution in bits: 14 or 11.
    pub temperature_resolution_bits: u8,
    /// The humidity resolution in bits: 14, 11, or 8.
    pub humidity_resolution_bits: u8,
}

/// Converts a raw HDC1080 temperature register to milli-degrees Celsius.
///
/// # Returns
///
/// The temperature, exact in integer arithmetic.
#[no_mangle]
pub extern "C" fn pamoja_hdc1080_milli_celsius(raw: u16) -> i32 {
    hdc1080::milli_celsius(raw)
}

/// Converts a raw HDC1080 temperature register to degrees Celsius.
///
/// # Returns
///
/// The temperature.
#[no_mangle]
pub extern "C" fn pamoja_hdc1080_celsius(raw: u16) -> f32 {
    hdc1080::celsius(raw)
}

/// Converts a raw HDC1080 humidity register to milli-percent.
///
/// # Returns
///
/// The relative humidity, exact in integer arithmetic.
#[no_mangle]
pub extern "C" fn pamoja_hdc1080_milli_percent(raw: u16) -> u32 {
    hdc1080::milli_percent(raw)
}

/// Converts a raw HDC1080 humidity register to a relative humidity percentage.
///
/// # Returns
///
/// The relative humidity.
#[no_mangle]
pub extern "C" fn pamoja_hdc1080_relative_humidity(raw: u16) -> f32 {
    hdc1080::relative_humidity(raw)
}

/// Builds the HDC1080 temperature register that decodes to a temperature.
///
/// # Returns
///
/// The 14-bit code in bits 15:2, clamped to the part's range.
#[no_mangle]
pub extern "C" fn pamoja_hdc1080_temperature_register(milli_celsius: i32) -> u16 {
    hdc1080::temperature_register(milli_celsius)
}

/// Builds the HDC1080 humidity register that decodes to a relative humidity.
///
/// # Returns
///
/// The 14-bit code in bits 15:2, clamped to full scale.
#[no_mangle]
pub extern "C" fn pamoja_hdc1080_humidity_register(milli_percent: u32) -> u16 {
    hdc1080::humidity_register(milli_percent)
}

/// Joins the three HDC1080 serial-ID registers into the 40-bit serial number.
///
/// # Returns
///
/// The serial number.
#[no_mangle]
pub extern "C" fn pamoja_hdc1080_serial_id(high: u16, mid: u16, low: u16) -> u64 {
    hdc1080::serial_id(high, mid, low)
}

/// Splits a serial number back into the three HDC1080 serial-ID registers.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the three registers written to
/// `out_registers` high word first, or [`PamojaStatus::InvalidArgument`] if the
/// pointer is null.
///
/// # Safety
///
/// `out_registers` must point to at least three writable `uint16_t` values.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_serial_id_registers(
    serial: u64,
    out_registers: *mut u16,
) -> PamojaStatus {
    if out_registers.is_null() {
        set_last_error("out_registers must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let registers = hdc1080::serial_id_registers(serial);
    core::ptr::copy_nonoverlapping(
        registers.as_ptr(),
        out_registers,
        PAMOJA_HDC1080_SERIAL_ID_REGISTERS,
    );
    PamojaStatus::Ok
}

/// Parses the four bytes an HDC1080 sequential read returns.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_measurement` filled in, or
/// [`PamojaStatus::InvalidArgument`] if the buffer is not four bytes. The part
/// sends no checksum, so a well-sized read always decodes.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes, and
/// `out_measurement` must point to a writable `PamojaHdc1080Measurement`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_parse_measurement(
    bytes: *const u8,
    bytes_len: usize,
    out_measurement: *mut PamojaHdc1080Measurement,
) -> PamojaStatus {
    if out_measurement.is_null() {
        set_last_error("out_measurement must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(bytes) = <[u8; PAMOJA_HDC1080_MEASUREMENT_LEN]>::try_from(&bytes[..]) else {
        return wrong_length("measurement", PAMOJA_HDC1080_MEASUREMENT_LEN);
    };
    *out_measurement = hdc1080::Measurement::parse(&bytes).into();
    PamojaStatus::Ok
}

/// Builds the HDC1080 measurement a sensor reporting these physical values would send.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_measurement` filled in, or
/// [`PamojaStatus::InvalidArgument`] if the pointer is null.
///
/// # Safety
///
/// `out_measurement` must point to a writable `PamojaHdc1080Measurement`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_measurement_from_physical(
    milli_celsius: i32,
    milli_percent: u32,
    out_measurement: *mut PamojaHdc1080Measurement,
) -> PamojaStatus {
    if out_measurement.is_null() {
        set_last_error("out_measurement must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let measurement = hdc1080::Measurement::from_physical(milli_celsius, milli_percent);
    *out_measurement = measurement.into();
    PamojaStatus::Ok
}

/// Builds the four bytes an HDC1080 sends for a pair of raw registers.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the four bytes written to `out_bytes`,
/// or [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least four writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_measurement_bytes(
    temperature_raw: u16,
    humidity_raw: u16,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let measurement = hdc1080::Measurement {
        temperature: temperature_raw,
        humidity: humidity_raw,
    };
    let bytes = measurement.to_bytes();
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_HDC1080_MEASUREMENT_LEN);
    PamojaStatus::Ok
}

/// Parses an HDC1080 configuration register value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_config` filled in, or
/// [`PamojaStatus::Codec`] if the humidity-resolution field carries the code the
/// datasheet leaves undefined, which means the value did not come from a working
/// part.
///
/// # Safety
///
/// `out_config` must point to a writable `PamojaHdc1080Config`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_config_from_register(
    raw: u16,
    out_config: *mut PamojaHdc1080Config,
) -> PamojaStatus {
    if out_config.is_null() {
        set_last_error("out_config must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match hdc1080::Configuration::from_register(raw) {
        Ok(config) => {
            *out_config = config.into();
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Assembles an HDC1080 configuration register value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_register` set, or
/// [`PamojaStatus::InvalidArgument`] if either resolution is not one the part
/// offers.
///
/// # Safety
///
/// `out_register` must point to a writable `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_config_to_register(
    config: PamojaHdc1080Config,
    out_register: *mut u16,
) -> PamojaStatus {
    if out_register.is_null() {
        set_last_error("out_register must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match hdc1080::Configuration::try_from(config) {
        Ok(config) => {
            *out_register = config.to_register();
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Returns how long to wait after triggering an HDC1080 in a configuration.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_micros` set, or
/// [`PamojaStatus::InvalidArgument`] if either resolution is not one the part
/// offers.
///
/// # Safety
///
/// `out_micros` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_conversion_time_micros(
    config: PamojaHdc1080Config,
    out_micros: *mut u32,
) -> PamojaStatus {
    if out_micros.is_null() {
        set_last_error("out_micros must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match hdc1080::Configuration::try_from(config) {
        Ok(config) => {
            *out_micros = config.conversion_time_micros();
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Returns how long an HDC1080 temperature conversion takes at a resolution.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_micros` set, or
/// [`PamojaStatus::InvalidArgument`] if `bits` is not 14 or 11.
///
/// # Safety
///
/// `out_micros` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_temperature_conversion_micros(
    bits: u8,
    out_micros: *mut u32,
) -> PamojaStatus {
    if out_micros.is_null() {
        set_last_error("out_micros must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match temperature_resolution(bits) {
        Some(resolution) => {
            *out_micros = resolution.conversion_time_micros();
            PamojaStatus::Ok
        }
        None => bad_temperature_resolution(),
    }
}

/// Returns how long an HDC1080 humidity conversion takes at a resolution.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_micros` set, or
/// [`PamojaStatus::InvalidArgument`] if `bits` is not 14, 11, or 8.
///
/// # Safety
///
/// `out_micros` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_humidity_conversion_micros(
    bits: u8,
    out_micros: *mut u32,
) -> PamojaStatus {
    if out_micros.is_null() {
        set_last_error("out_micros must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match humidity_resolution(bits) {
        Some(resolution) => {
            *out_micros = resolution.conversion_time_micros();
            PamojaStatus::Ok
        }
        None => bad_humidity_resolution(),
    }
}

/// The number of bytes in an OPT3001 register read.
pub const PAMOJA_OPT3001_REGISTER_LEN: usize = 2;

/// The address an OPT3001 answers on with its ADDR pin tied to GND.
pub const PAMOJA_OPT3001_I2C_ADDRESS_GND: u8 = 0x44;

/// The address it answers on with ADDR tied to VDD.
pub const PAMOJA_OPT3001_I2C_ADDRESS_VDD: u8 = 0x45;

/// The address it answers on with ADDR tied to SDA.
pub const PAMOJA_OPT3001_I2C_ADDRESS_SDA: u8 = 0x46;

/// The address it answers on with ADDR tied to SCL.
pub const PAMOJA_OPT3001_I2C_ADDRESS_SCL: u8 = 0x47;

/// The value its manufacturer-ID register reads: TI.
pub const PAMOJA_OPT3001_MANUFACTURER_ID: u16 = 0x5449;

/// The value its device-ID register reads, which confirms the part.
pub const PAMOJA_OPT3001_DEVICE_ID: u16 = 0x3001;

/// The value its configuration register reads after a reset.
pub const PAMOJA_OPT3001_CONFIGURATION_RESET: u16 = 0xC810;

/// The value its low-limit register reads after a reset.
pub const PAMOJA_OPT3001_LOW_LIMIT_RESET: u16 = 0x0000;

/// The value its high-limit register reads after a reset.
pub const PAMOJA_OPT3001_HIGH_LIMIT_RESET: u16 = 0xBFFF;

/// The low-limit value that turns the INT pin into an end-of-conversion signal.
pub const PAMOJA_OPT3001_LOW_LIMIT_END_OF_CONVERSION: u16 = 0xC000;

/// The range number that lets the part choose its own full scale.
pub const PAMOJA_OPT3001_RANGE_AUTOMATIC: u8 = 0b1100;

/// The highest fixed range number the part defines.
pub const PAMOJA_OPT3001_RANGE_MAX: u8 = 11;

/// The OPT3001 result register.
pub const PAMOJA_OPT3001_REGISTER_RESULT: u8 = 0x00;

/// The OPT3001 configuration register.
pub const PAMOJA_OPT3001_REGISTER_CONFIGURATION: u8 = 0x01;

/// The OPT3001 low-limit register.
pub const PAMOJA_OPT3001_REGISTER_LOW_LIMIT: u8 = 0x02;

/// The OPT3001 high-limit register.
pub const PAMOJA_OPT3001_REGISTER_HIGH_LIMIT: u8 = 0x03;

/// The OPT3001 manufacturer-ID register.
pub const PAMOJA_OPT3001_REGISTER_MANUFACTURER_ID: u8 = 0x7E;

/// The OPT3001 device-ID register.
pub const PAMOJA_OPT3001_REGISTER_DEVICE_ID: u8 = 0x7F;

// The header generator does not read the crates this one depends on, so these
// carry their value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_OPT3001_I2C_ADDRESS_GND == opt3001::I2C_ADDRESS_GND);
const _: () = assert!(PAMOJA_OPT3001_I2C_ADDRESS_VDD == opt3001::I2C_ADDRESS_VDD);
const _: () = assert!(PAMOJA_OPT3001_I2C_ADDRESS_SDA == opt3001::I2C_ADDRESS_SDA);
const _: () = assert!(PAMOJA_OPT3001_I2C_ADDRESS_SCL == opt3001::I2C_ADDRESS_SCL);
const _: () = assert!(PAMOJA_OPT3001_MANUFACTURER_ID == opt3001::MANUFACTURER_ID);
const _: () = assert!(PAMOJA_OPT3001_DEVICE_ID == opt3001::DEVICE_ID);
const _: () = assert!(PAMOJA_OPT3001_CONFIGURATION_RESET == opt3001::CONFIGURATION_RESET);
const _: () = assert!(PAMOJA_OPT3001_LOW_LIMIT_RESET == opt3001::LOW_LIMIT_RESET);
const _: () = assert!(PAMOJA_OPT3001_HIGH_LIMIT_RESET == opt3001::HIGH_LIMIT_RESET);
const _: () =
    assert!(PAMOJA_OPT3001_LOW_LIMIT_END_OF_CONVERSION == opt3001::LOW_LIMIT_END_OF_CONVERSION);
const _: () = assert!(PAMOJA_OPT3001_RANGE_AUTOMATIC == opt3001::RANGE_AUTOMATIC);
const _: () = assert!(PAMOJA_OPT3001_RANGE_MAX == opt3001::RANGE_MAX);
const _: () = assert!(PAMOJA_OPT3001_REGISTER_RESULT == opt3001::register::RESULT);
const _: () = assert!(PAMOJA_OPT3001_REGISTER_CONFIGURATION == opt3001::register::CONFIGURATION);
const _: () = assert!(PAMOJA_OPT3001_REGISTER_LOW_LIMIT == opt3001::register::LOW_LIMIT);
const _: () = assert!(PAMOJA_OPT3001_REGISTER_HIGH_LIMIT == opt3001::register::HIGH_LIMIT);
const _: () =
    assert!(PAMOJA_OPT3001_REGISTER_MANUFACTURER_ID == opt3001::register::MANUFACTURER_ID);
const _: () = assert!(PAMOJA_OPT3001_REGISTER_DEVICE_ID == opt3001::register::DEVICE_ID);

/// An OPT3001 configuration register, field by field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaOpt3001Config {
    /// The full-scale range number, `0..=11`, or `12` to set the range automatically.
    pub range_number: u8,
    /// `1` makes a conversion take 800 ms rather than 100 ms.
    pub long_conversion: u8,
    /// The mode code: `0` shutdown, `1` single shot, `2` continuous.
    pub mode: u8,
    /// `1` when the last result overflowed its range.
    pub overflow: u8,
    /// `1` when a conversion has completed since the register was last read.
    pub conversion_ready: u8,
    /// `1` when the result went above the high limit.
    pub flag_high: u8,
    /// `1` when the result went below the low limit.
    pub flag_low: u8,
    /// `1` latches the INT pin until the configuration register is read.
    pub latched_window: u8,
    /// `1` makes the INT pin active high.
    pub active_high: u8,
    /// `1` makes the limit registers carry a mantissa alone, without an exponent.
    pub mask_exponent: u8,
    /// The fault-count code, `0..=3`, for one, two, four, or eight faults.
    pub fault_count: u8,
}

/// Returns the illuminance one count carries at an OPT3001 exponent.
///
/// # Returns
///
/// `true` when the exponent is one the part defines, with `*out_milli_lux` set to
/// the step; `false` for a reserved exponent.
///
/// # Safety
///
/// `out_milli_lux` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_lsb_milli_lux(
    exponent: u8,
    out_milli_lux: *mut u32,
) -> bool {
    match opt3001::lsb_milli_lux(exponent) {
        Some(step) if !out_milli_lux.is_null() => {
            *out_milli_lux = step;
            true
        }
        _ => false,
    }
}

/// Returns the full scale an OPT3001 range number covers.
///
/// # Returns
///
/// `true` when the range is one the part defines, with `*out_milli_lux` set to
/// the full scale; `false` for a reserved range number, which has none.
///
/// # Safety
///
/// `out_milli_lux` must point to a writable `uint32_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_full_scale_milli_lux(
    range_number: u8,
    out_milli_lux: *mut u32,
) -> bool {
    match opt3001::full_scale_milli_lux(range_number) {
        Some(full_scale) if !out_milli_lux.is_null() => {
            *out_milli_lux = full_scale;
            true
        }
        _ => false,
    }
}

/// Converts a raw OPT3001 result register to milli-lux.
///
/// # Returns
///
/// The illuminance, exact in integer arithmetic.
#[no_mangle]
pub extern "C" fn pamoja_opt3001_milli_lux(raw: u16) -> u32 {
    opt3001::milli_lux(raw)
}

/// Converts a raw OPT3001 result register to lux.
///
/// # Returns
///
/// The illuminance.
#[no_mangle]
pub extern "C" fn pamoja_opt3001_lux(raw: u16) -> f32 {
    opt3001::lux(raw)
}

/// Builds the OPT3001 result register that decodes to an illuminance.
///
/// # Returns
///
/// The register value, using the smallest exponent that fits and saturating at
/// full scale.
#[no_mangle]
pub extern "C" fn pamoja_opt3001_raw_from_milli_lux(milli_lux: u32) -> u16 {
    opt3001::raw_from_milli_lux(milli_lux)
}

/// Reads the two bytes an OPT3001 sends for a register, most significant first.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_word` set, or
/// [`PamojaStatus::InvalidArgument`] if the buffer is not two bytes.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes, and `out_word` must
/// point to a writable `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_word_from_bytes(
    bytes: *const u8,
    bytes_len: usize,
    out_word: *mut u16,
) -> PamojaStatus {
    if out_word.is_null() {
        set_last_error("out_word must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(bytes) = <[u8; PAMOJA_OPT3001_REGISTER_LEN]>::try_from(&bytes[..]) else {
        return wrong_length("register", PAMOJA_OPT3001_REGISTER_LEN);
    };
    *out_word = opt3001::word_from_bytes(bytes);
    PamojaStatus::Ok
}

/// Builds the two bytes an OPT3001 sends for a register, most significant first.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the two bytes written to `out_bytes`, or
/// [`PamojaStatus::InvalidArgument`] if `out_bytes` is null.
///
/// # Safety
///
/// `out_bytes` must point to at least two writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_word_to_bytes(
    word: u16,
    out_bytes: *mut u8,
) -> PamojaStatus {
    if out_bytes.is_null() {
        set_last_error("out_bytes must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = opt3001::word_to_bytes(word);
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_bytes, PAMOJA_OPT3001_REGISTER_LEN);
    PamojaStatus::Ok
}

/// Assembles the 16-bit OPT3001 configuration register value.
///
/// # Returns
///
/// The register value to write, with the read-only status bits written as zero.
#[no_mangle]
pub extern "C" fn pamoja_opt3001_config_bits(config: PamojaOpt3001Config) -> u16 {
    opt3001::Configuration::from(config).bits()
}

/// Parses a 16-bit OPT3001 configuration register value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_config` filled in. Every register value
/// decodes, so this fails only on a null pointer.
///
/// # Safety
///
/// `out_config` must point to a writable `PamojaOpt3001Config`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_config_from_bits(
    bits: u16,
    out_config: *mut PamojaOpt3001Config,
) -> PamojaStatus {
    if out_config.is_null() {
        set_last_error("out_config must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_config = opt3001::Configuration::from_bits(bits).into();
    PamojaStatus::Ok
}

/// Returns the conversion time an OPT3001 setting selects.
///
/// # Returns
///
/// The time in milliseconds: 800 for the long conversion, 100 otherwise.
#[no_mangle]
pub extern "C" fn pamoja_opt3001_conversion_millis(long_conversion: bool) -> u16 {
    conversion_time(long_conversion).millis()
}

/// Returns how many consecutive faults an OPT3001 fault-count code requires.
///
/// # Returns
///
/// The fault count: 1, 2, 4, or 8.
#[no_mangle]
pub extern "C" fn pamoja_opt3001_fault_count(code: u8) -> u8 {
    opt3001::FaultCount::from_code(code).count()
}

/// Reports whether an OPT3001 range number sets the full scale automatically.
///
/// # Returns
///
/// `true` for the automatic range number, which has no fixed full scale.
#[no_mangle]
pub extern "C" fn pamoja_opt3001_is_automatic_range(range_number: u8) -> bool {
    range_number == opt3001::RANGE_AUTOMATIC
}

/// The address an INA226 answers on with both address pins tied to GND.
pub const PAMOJA_INA226_BASE_ADDRESS: u8 = 0x40;

/// The value its manufacturer-ID register reads: TI.
pub const PAMOJA_INA226_MANUFACTURER_ID: u16 = 0x5449;

/// The device identifier its die-ID register carries.
pub const PAMOJA_INA226_DEVICE_ID: u16 = 0x226;

/// The value its configuration register reads after a reset.
pub const PAMOJA_INA226_CONFIG_RESET: u16 = 0x4127;

/// The shunt-voltage resolution, in nanovolts per count.
pub const PAMOJA_INA226_SHUNT_LSB_NANOVOLTS: i32 = 2_500;

/// The bus-voltage resolution, in microvolts per count.
pub const PAMOJA_INA226_BUS_LSB_MICROVOLTS: u32 = 1_250;

/// How many times the power resolution is the current resolution.
pub const PAMOJA_INA226_POWER_LSB_RATIO: u32 = 25;

/// The INA226 configuration register.
pub const PAMOJA_INA226_REGISTER_CONFIGURATION: u8 = 0x00;

/// The INA226 shunt-voltage register.
pub const PAMOJA_INA226_REGISTER_SHUNT_VOLTAGE: u8 = 0x01;

/// The INA226 bus-voltage register.
pub const PAMOJA_INA226_REGISTER_BUS_VOLTAGE: u8 = 0x02;

/// The INA226 power register.
pub const PAMOJA_INA226_REGISTER_POWER: u8 = 0x03;

/// The INA226 current register.
pub const PAMOJA_INA226_REGISTER_CURRENT: u8 = 0x04;

/// The INA226 calibration register.
pub const PAMOJA_INA226_REGISTER_CALIBRATION: u8 = 0x05;

/// The INA226 Mask/Enable register.
pub const PAMOJA_INA226_REGISTER_MASK_ENABLE: u8 = 0x06;

/// The INA226 alert-limit register.
pub const PAMOJA_INA226_REGISTER_ALERT_LIMIT: u8 = 0x07;

/// The INA226 manufacturer-ID register.
pub const PAMOJA_INA226_REGISTER_MANUFACTURER_ID: u8 = 0xFE;

/// The INA226 die-ID register.
pub const PAMOJA_INA226_REGISTER_DIE_ID: u8 = 0xFF;

// The header generator does not read the crates this one depends on, so these
// carry their value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_INA226_BASE_ADDRESS == ina226::BASE_ADDRESS);
const _: () = assert!(PAMOJA_INA226_MANUFACTURER_ID == ina226::MANUFACTURER_ID);
const _: () = assert!(PAMOJA_INA226_DEVICE_ID == ina226::DEVICE_ID);
const _: () = assert!(PAMOJA_INA226_CONFIG_RESET == ina226::CONFIG_RESET);
const _: () = assert!(PAMOJA_INA226_SHUNT_LSB_NANOVOLTS == ina226::SHUNT_LSB_NANOVOLTS);
const _: () = assert!(PAMOJA_INA226_BUS_LSB_MICROVOLTS == ina226::BUS_LSB_MICROVOLTS);
const _: () = assert!(PAMOJA_INA226_POWER_LSB_RATIO == ina226::POWER_LSB_RATIO);
const _: () = assert!(PAMOJA_INA226_REGISTER_CONFIGURATION == ina226::register::CONFIGURATION);
const _: () = assert!(PAMOJA_INA226_REGISTER_SHUNT_VOLTAGE == ina226::register::SHUNT_VOLTAGE);
const _: () = assert!(PAMOJA_INA226_REGISTER_BUS_VOLTAGE == ina226::register::BUS_VOLTAGE);
const _: () = assert!(PAMOJA_INA226_REGISTER_POWER == ina226::register::POWER);
const _: () = assert!(PAMOJA_INA226_REGISTER_CURRENT == ina226::register::CURRENT);
const _: () = assert!(PAMOJA_INA226_REGISTER_CALIBRATION == ina226::register::CALIBRATION);
const _: () = assert!(PAMOJA_INA226_REGISTER_MASK_ENABLE == ina226::register::MASK_ENABLE);
const _: () = assert!(PAMOJA_INA226_REGISTER_ALERT_LIMIT == ina226::register::ALERT_LIMIT);
const _: () = assert!(PAMOJA_INA226_REGISTER_MANUFACTURER_ID == ina226::register::MANUFACTURER_ID);
const _: () = assert!(PAMOJA_INA226_REGISTER_DIE_ID == ina226::register::DIE_ID);

/// The limit comparison an INA226 alert pin responds to.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaIna226AlertFunction {
    /// Shunt voltage above the alert limit.
    ShuntOverLimit = 0,
    /// Shunt voltage below the alert limit.
    ShuntUnderLimit = 1,
    /// Bus voltage above the alert limit.
    BusOverLimit = 2,
    /// Bus voltage below the alert limit.
    BusUnderLimit = 3,
    /// Power above the alert limit.
    PowerOverLimit = 4,
}

/// An INA226 configuration register, field by field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaIna226Config {
    /// `1` resets the part when this register is written.
    pub reset: u8,
    /// The averaging code, `0..=7`, from 1 to 1024 samples.
    pub averaging: u8,
    /// The bus-voltage conversion-time code, `0..=7`.
    pub bus_conversion_time: u8,
    /// The shunt-voltage conversion-time code, `0..=7`.
    pub shunt_conversion_time: u8,
    /// The operating-mode code, `0..=7`.
    pub mode: u8,
}

/// An INA226 Mask/Enable register, field by field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaIna226MaskEnable {
    /// `1` alerts when the shunt voltage exceeds the limit.
    pub shunt_over_limit: u8,
    /// `1` alerts when the shunt voltage drops below the limit.
    pub shunt_under_limit: u8,
    /// `1` alerts when the bus voltage exceeds the limit.
    pub bus_over_limit: u8,
    /// `1` alerts when the bus voltage drops below the limit.
    pub bus_under_limit: u8,
    /// `1` alerts when the power exceeds the limit.
    pub power_over_limit: u8,
    /// `1` also alerts when a conversion completes.
    pub conversion_ready: u8,
    /// `1` when the selected limit function caused the last alert.
    pub alert_function_flag: u8,
    /// `1` when every conversion and multiplication has completed.
    pub conversion_ready_flag: u8,
    /// `1` when an arithmetic overflow left current and power invalid.
    pub math_overflow: u8,
    /// `1` makes the alert pin active high.
    pub alert_active_high: u8,
    /// `1` latches the alert pin until this register is read.
    pub alert_latch: u8,
}

/// A decoded INA226 die-ID register.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaIna226DieId {
    /// The 12-bit device identifier.
    pub device: u16,
    /// The 4-bit die revision.
    pub revision: u8,
}

/// Returns the I2C address an INA226's A1 and A0 pin codes select.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_address` set to a 7-bit address in
/// `0x40..=0x4F`, or [`PamojaStatus::InvalidArgument`] if either code is above 3.
///
/// # Safety
///
/// `out_address` must point to a writable `uint8_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_address(
    a1: u8,
    a0: u8,
    out_address: *mut u8,
) -> PamojaStatus {
    if out_address.is_null() {
        set_last_error("out_address must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let (Some(a1), Some(a0)) = (address_pin(a1), address_pin(a0)) else {
        set_last_error(
            "an INA226 address pin must be tied to GND, VS, SDA, or SCL: code 0 to 3".to_owned(),
        );
        return PamojaStatus::InvalidArgument;
    };
    *out_address = ina226::address(a1, a0);
    PamojaStatus::Ok
}

/// Returns how many samples an INA226 averaging code folds into one result.
///
/// # Returns
///
/// The sample count, from 1 to 1024.
#[no_mangle]
pub extern "C" fn pamoja_ina226_averaging_samples(code: u8) -> u16 {
    averaging(code).samples()
}

/// Returns the conversion time an INA226 code selects.
///
/// # Returns
///
/// The time in microseconds.
#[no_mangle]
pub extern "C" fn pamoja_ina226_conversion_micros(code: u8) -> u32 {
    conversion_time_setting(code).microseconds()
}

/// Reports whether an INA226 mode code converts the shunt voltage.
///
/// # Returns
///
/// `true` when the shunt is measured in that mode.
#[no_mangle]
pub extern "C" fn pamoja_ina226_measures_shunt(code: u8) -> bool {
    mode(code).measures_shunt()
}

/// Reports whether an INA226 mode code converts the bus voltage.
///
/// # Returns
///
/// `true` when the bus is measured in that mode.
#[no_mangle]
pub extern "C" fn pamoja_ina226_measures_bus(code: u8) -> bool {
    mode(code).measures_bus()
}

/// Reports whether an INA226 mode code keeps converting after the first result.
///
/// # Returns
///
/// `true` for the continuous modes.
#[no_mangle]
pub extern "C" fn pamoja_ina226_is_continuous(code: u8) -> bool {
    mode(code).is_continuous()
}

/// Parses an INA226 configuration register value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_config` filled in. Every register value
/// decodes, so this fails only on a null pointer.
///
/// # Safety
///
/// `out_config` must point to a writable `PamojaIna226Config`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_config_from_register(
    raw: u16,
    out_config: *mut PamojaIna226Config,
) -> PamojaStatus {
    if out_config.is_null() {
        set_last_error("out_config must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_config = ina226::Configuration::from_register(raw).into();
    PamojaStatus::Ok
}

/// Assembles an INA226 configuration register value.
///
/// # Returns
///
/// The register value to write.
#[no_mangle]
pub extern "C" fn pamoja_ina226_config_to_register(config: PamojaIna226Config) -> u16 {
    ina226::Configuration::from(config).to_register()
}

/// Returns how often an INA226 in a configuration updates its results.
///
/// # Returns
///
/// The interval in microseconds: every conversion the mode takes, averaged.
#[no_mangle]
pub extern "C" fn pamoja_ina226_update_micros(config: PamojaIna226Config) -> u32 {
    ina226::Configuration::from(config).update_microseconds()
}

/// Parses an INA226 Mask/Enable register value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_mask` filled in. Every register value
/// decodes, so this fails only on a null pointer.
///
/// # Safety
///
/// `out_mask` must point to a writable `PamojaIna226MaskEnable`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_mask_enable_from_register(
    raw: u16,
    out_mask: *mut PamojaIna226MaskEnable,
) -> PamojaStatus {
    if out_mask.is_null() {
        set_last_error("out_mask must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_mask = ina226::MaskEnable::from_register(raw).into();
    PamojaStatus::Ok
}

/// Assembles an INA226 Mask/Enable register value.
///
/// # Returns
///
/// The register value to write, with the read-only flags written as zero.
#[no_mangle]
pub extern "C" fn pamoja_ina226_mask_enable_to_register(mask: PamojaIna226MaskEnable) -> u16 {
    ina226::MaskEnable::from(mask).to_register()
}

/// Returns the alert function an INA226 pin actually responds to.
///
/// # Returns
///
/// `true` when one limit function is selected, with `*out_function` set to it;
/// `false` when none is, so the pin only ever signals a completed conversion.
///
/// # Safety
///
/// `out_function` must point to a writable `PamojaIna226AlertFunction`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_active_alert_function(
    mask: PamojaIna226MaskEnable,
    out_function: *mut PamojaIna226AlertFunction,
) -> bool {
    match ina226::MaskEnable::from(mask).active_alert_function() {
        Some(function) if !out_function.is_null() => {
            *out_function = function.into();
            true
        }
        _ => false,
    }
}

/// Splits an INA226 die-ID register into its device and revision fields.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with `*out_die_id` filled in. Every register value
/// decodes, so this fails only on a null pointer.
///
/// # Safety
///
/// `out_die_id` must point to a writable `PamojaIna226DieId`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_die_id(
    raw: u16,
    out_die_id: *mut PamojaIna226DieId,
) -> PamojaStatus {
    if out_die_id.is_null() {
        set_last_error("out_die_id must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_die_id = ina226::DieId::from_register(raw).into();
    PamojaStatus::Ok
}

/// Checks that a pair of identification registers belongs to an INA226.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_die_id` filled in, or
/// [`PamojaStatus::Codec`] if either register carries something other than the
/// values the datasheet fixes, which means a different part, or nothing at all,
/// answered at that address.
///
/// # Safety
///
/// `out_die_id` must point to a writable `PamojaIna226DieId`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_identify(
    manufacturer_id: u16,
    die_id: u16,
    out_die_id: *mut PamojaIna226DieId,
) -> PamojaStatus {
    if out_die_id.is_null() {
        set_last_error("out_die_id must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match ina226::identify(manufacturer_id, die_id) {
        Ok(identified) => {
            *out_die_id = identified.into();
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Computes the INA226 calibration register for a shunt and current resolution.
///
/// # Returns
///
/// The register value to write.
#[no_mangle]
pub extern "C" fn pamoja_ina226_calibration(
    current_lsb_microamps: u32,
    shunt_milliohms: u32,
) -> u16 {
    ina226::calibration(current_lsb_microamps, shunt_milliohms)
}

/// Returns the smallest current resolution that still covers an expected maximum.
///
/// # Returns
///
/// The current LSB in microamps.
#[no_mangle]
pub extern "C" fn pamoja_ina226_minimum_current_lsb_microamps(max_expected_microamps: u32) -> u32 {
    ina226::minimum_current_lsb_microamps(max_expected_microamps)
}

/// Converts a raw INA226 shunt-voltage register to nanovolts.
///
/// # Returns
///
/// The shunt voltage, exact in integer arithmetic at 2.5 uV per count.
#[no_mangle]
pub extern "C" fn pamoja_ina226_shunt_nanovolts(raw: i16) -> i32 {
    ina226::shunt_nanovolts(raw)
}

/// Converts a raw INA226 shunt-voltage register to millivolts.
///
/// # Returns
///
/// The shunt voltage.
#[no_mangle]
pub extern "C" fn pamoja_ina226_shunt_millivolts(raw: i16) -> f32 {
    ina226::shunt_millivolts_f32(raw)
}

/// Converts a raw INA226 bus-voltage register to microvolts.
///
/// # Returns
///
/// The bus voltage, exact in integer arithmetic at 1.25 mV per count.
#[no_mangle]
pub extern "C" fn pamoja_ina226_bus_microvolts(raw: u16) -> u32 {
    ina226::bus_microvolts(raw)
}

/// Converts a raw INA226 bus-voltage register to volts.
///
/// # Returns
///
/// The bus voltage.
#[no_mangle]
pub extern "C" fn pamoja_ina226_bus_volts(raw: u16) -> f32 {
    ina226::bus_volts_f32(raw)
}

/// Converts a raw INA226 current register to microamps.
///
/// # Returns
///
/// The current, at the resolution the calibration selected.
#[no_mangle]
pub extern "C" fn pamoja_ina226_current_microamps(raw: i16, current_lsb_microamps: u32) -> i32 {
    ina226::current_microamps(raw, current_lsb_microamps)
}

/// Converts a raw INA226 current register to amps.
///
/// # Returns
///
/// The current.
#[no_mangle]
pub extern "C" fn pamoja_ina226_current_amps(raw: i16, current_lsb_microamps: u32) -> f32 {
    ina226::current_amps_f32(raw, current_lsb_microamps)
}

/// Converts a raw INA226 power register to microwatts.
///
/// # Returns
///
/// The power. The power LSB is fixed at twenty-five times the current LSB.
#[no_mangle]
pub extern "C" fn pamoja_ina226_power_microwatts(raw: u16, current_lsb_microamps: u32) -> u32 {
    ina226::power_microwatts(raw, current_lsb_microamps)
}

/// Converts a raw INA226 power register to watts.
///
/// # Returns
///
/// The power.
#[no_mangle]
pub extern "C" fn pamoja_ina226_power_watts(raw: u16, current_lsb_microamps: u32) -> f32 {
    ina226::power_watts_f32(raw, current_lsb_microamps)
}

/// Builds the INA226 shunt-voltage register a monitor reports for a shunt voltage.
///
/// # Returns
///
/// The signed register value, at 2.5 uV per count.
#[no_mangle]
pub extern "C" fn pamoja_ina226_shunt_register(nanovolts: i32) -> i16 {
    ina226::shunt_register(nanovolts)
}

/// Builds the INA226 bus-voltage register a monitor reports for a bus voltage.
///
/// # Returns
///
/// The register value, at 1.25 mV per count.
#[no_mangle]
pub extern "C" fn pamoja_ina226_bus_register(microvolts: u32) -> u16 {
    ina226::bus_register(microvolts)
}

/// Builds the INA226 current register a monitor reports for a current.
///
/// # Returns
///
/// The signed register value, or zero if `current_lsb_microamps` is zero.
#[no_mangle]
pub extern "C" fn pamoja_ina226_current_register(
    microamps: i32,
    current_lsb_microamps: u32,
) -> i16 {
    ina226::current_register(microamps, current_lsb_microamps)
}

/// Builds the INA226 power register a monitor reports for a power.
///
/// # Returns
///
/// The register value, or zero if `current_lsb_microamps` is zero.
#[no_mangle]
pub extern "C" fn pamoja_ina226_power_register(microwatts: u32, current_lsb_microamps: u32) -> u16 {
    ina226::power_register(microwatts, current_lsb_microamps)
}

/// Computes the INA226 current register the chip derives from a shunt reading.
///
/// # Returns
///
/// The signed current register the part's own arithmetic produces.
#[no_mangle]
pub extern "C" fn pamoja_ina226_current_register_from_shunt(shunt: i16, calibration: u16) -> i16 {
    ina226::current_register_from_shunt(shunt, calibration)
}

/// Computes the INA226 power register the chip derives from a current reading.
///
/// # Returns
///
/// The power register the part's own arithmetic produces.
#[no_mangle]
pub extern "C" fn pamoja_ina226_power_register_from_current(current: i16, bus: u16) -> u16 {
    ina226::power_register_from_current(current, bus)
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

impl From<bmp280::Calibration> for PamojaBmp280Coefficients {
    fn from(value: bmp280::Calibration) -> Self {
        PamojaBmp280Coefficients {
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

impl From<bmp280::Measurement> for PamojaBmp280Measurement {
    fn from(value: bmp280::Measurement) -> Self {
        PamojaBmp280Measurement {
            pressure: value.pressure,
            temperature: value.temperature,
            pressure_skipped: u8::from(value.pressure_skipped()),
            temperature_skipped: u8::from(value.temperature_skipped()),
        }
    }
}

impl From<PamojaBmp280CtrlMeas> for bmp280::CtrlMeas {
    fn from(value: PamojaBmp280CtrlMeas) -> Self {
        bmp280::CtrlMeas {
            temperature: bmp280::Oversampling::from_code(value.temperature),
            pressure: bmp280::Oversampling::from_code(value.pressure),
            mode: bmp280::Mode::from_code(value.mode),
        }
    }
}

impl From<bmp280::CtrlMeas> for PamojaBmp280CtrlMeas {
    fn from(value: bmp280::CtrlMeas) -> Self {
        PamojaBmp280CtrlMeas {
            temperature: value.temperature.code(),
            pressure: value.pressure.code(),
            mode: value.mode.code(),
        }
    }
}

impl From<PamojaBmp280Config> for bmp280::Config {
    fn from(value: PamojaBmp280Config) -> Self {
        bmp280::Config {
            standby: bmp280::Standby::from_code(value.standby),
            filter: value.filter,
            spi_3wire: value.spi_3wire != 0,
        }
    }
}

impl From<bmp280::Config> for PamojaBmp280Config {
    fn from(value: bmp280::Config) -> Self {
        PamojaBmp280Config {
            standby: value.standby.code(),
            filter: value.filter,
            spi_3wire: u8::from(value.spi_3wire),
        }
    }
}

impl From<sht3x::Measurement> for PamojaSht3xMeasurement {
    fn from(value: sht3x::Measurement) -> Self {
        PamojaSht3xMeasurement {
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

impl From<sht3x::Status> for PamojaSht3xStatus {
    fn from(value: sht3x::Status) -> Self {
        PamojaSht3xStatus {
            bits: value.bits(),
            alert_pending: u8::from(value.alert_pending()),
            heater_on: u8::from(value.heater_on()),
            humidity_tracking_alert: u8::from(value.humidity_tracking_alert()),
            temperature_tracking_alert: u8::from(value.temperature_tracking_alert()),
            reset_detected: u8::from(value.reset_detected()),
            command_failed: u8::from(value.command_failed()),
            write_checksum_failed: u8::from(value.write_checksum_failed()),
        }
    }
}

impl From<scd4x::Measurement> for PamojaScd4xMeasurement {
    fn from(value: scd4x::Measurement) -> Self {
        PamojaScd4xMeasurement {
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

impl From<PamojaTmp117Config> for tmp117::Configuration {
    fn from(value: PamojaTmp117Config) -> Self {
        tmp117::Configuration {
            high_alert: value.high_alert != 0,
            low_alert: value.low_alert != 0,
            data_ready: value.data_ready != 0,
            eeprom_busy: value.eeprom_busy != 0,
            mode: tmp117::ConversionMode::from_code(value.mode),
            cycle: tmp117::ConversionCycle::from_code(value.cycle),
            averaging: tmp117::Averaging::from_code(value.averaging),
            alert_mode: if value.therm_mode != 0 {
                tmp117::AlertMode::Therm
            } else {
                tmp117::AlertMode::Alert
            },
            alert_polarity: if value.alert_active_high != 0 {
                tmp117::AlertPolarity::ActiveHigh
            } else {
                tmp117::AlertPolarity::ActiveLow
            },
            alert_pin: if value.alert_pin_data_ready != 0 {
                tmp117::AlertPin::DataReady
            } else {
                tmp117::AlertPin::AlertFlags
            },
            soft_reset: value.soft_reset != 0,
        }
    }
}

impl From<tmp117::Configuration> for PamojaTmp117Config {
    fn from(value: tmp117::Configuration) -> Self {
        PamojaTmp117Config {
            high_alert: u8::from(value.high_alert),
            low_alert: u8::from(value.low_alert),
            data_ready: u8::from(value.data_ready),
            eeprom_busy: u8::from(value.eeprom_busy),
            mode: value.mode.code(),
            cycle: value.cycle.code(),
            averaging: value.averaging.code(),
            therm_mode: u8::from(matches!(value.alert_mode, tmp117::AlertMode::Therm)),
            alert_active_high: u8::from(matches!(
                value.alert_polarity,
                tmp117::AlertPolarity::ActiveHigh
            )),
            alert_pin_data_ready: u8::from(matches!(value.alert_pin, tmp117::AlertPin::DataReady)),
            soft_reset: u8::from(value.soft_reset),
        }
    }
}

impl From<hdc1080::Measurement> for PamojaHdc1080Measurement {
    fn from(value: hdc1080::Measurement) -> Self {
        PamojaHdc1080Measurement {
            temperature_raw: value.temperature,
            humidity_raw: value.humidity,
            milli_celsius: value.milli_celsius(),
            celsius: value.celsius(),
            milli_percent: value.milli_percent(),
            relative_humidity: value.relative_humidity(),
        }
    }
}

impl From<hdc1080::Configuration> for PamojaHdc1080Config {
    fn from(value: hdc1080::Configuration) -> Self {
        PamojaHdc1080Config {
            software_reset: u8::from(value.software_reset),
            heater: u8::from(value.heater),
            sequential: u8::from(matches!(
                value.mode,
                hdc1080::AcquisitionMode::TemperatureThenHumidity
            )),
            battery_low: u8::from(value.battery_low),
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

impl TryFrom<PamojaHdc1080Config> for hdc1080::Configuration {
    type Error = PamojaStatus;

    fn try_from(value: PamojaHdc1080Config) -> Result<Self, PamojaStatus> {
        let Some(temperature_resolution) =
            temperature_resolution(value.temperature_resolution_bits)
        else {
            return Err(bad_temperature_resolution());
        };
        let Some(humidity_resolution) = humidity_resolution(value.humidity_resolution_bits) else {
            return Err(bad_humidity_resolution());
        };
        Ok(hdc1080::Configuration {
            software_reset: value.software_reset != 0,
            heater: value.heater != 0,
            mode: if value.sequential != 0 {
                hdc1080::AcquisitionMode::TemperatureThenHumidity
            } else {
                hdc1080::AcquisitionMode::Single
            },
            battery_low: value.battery_low != 0,
            temperature_resolution,
            humidity_resolution,
        })
    }
}

impl From<PamojaOpt3001Config> for opt3001::Configuration {
    fn from(value: PamojaOpt3001Config) -> Self {
        opt3001::Configuration {
            range_number: value.range_number,
            conversion_time: conversion_time(value.long_conversion != 0),
            mode: opt3001::Mode::from_code(value.mode),
            overflow: value.overflow != 0,
            conversion_ready: value.conversion_ready != 0,
            flag_high: value.flag_high != 0,
            flag_low: value.flag_low != 0,
            latch: if value.latched_window != 0 {
                opt3001::Latch::LatchedWindow
            } else {
                opt3001::Latch::TransparentHysteresis
            },
            polarity: if value.active_high != 0 {
                opt3001::Polarity::ActiveHigh
            } else {
                opt3001::Polarity::ActiveLow
            },
            mask_exponent: value.mask_exponent != 0,
            fault_count: opt3001::FaultCount::from_code(value.fault_count),
        }
    }
}

impl From<opt3001::Configuration> for PamojaOpt3001Config {
    fn from(value: opt3001::Configuration) -> Self {
        PamojaOpt3001Config {
            range_number: value.range_number,
            long_conversion: u8::from(matches!(
                value.conversion_time,
                opt3001::ConversionTime::Ms800
            )),
            mode: value.mode.code(),
            overflow: u8::from(value.overflow),
            conversion_ready: u8::from(value.conversion_ready),
            flag_high: u8::from(value.flag_high),
            flag_low: u8::from(value.flag_low),
            latched_window: u8::from(matches!(value.latch, opt3001::Latch::LatchedWindow)),
            active_high: u8::from(matches!(value.polarity, opt3001::Polarity::ActiveHigh)),
            mask_exponent: u8::from(value.mask_exponent),
            fault_count: value.fault_count.code(),
        }
    }
}

impl From<PamojaIna226Config> for ina226::Configuration {
    fn from(value: PamojaIna226Config) -> Self {
        ina226::Configuration {
            reset: value.reset != 0,
            averaging: averaging(value.averaging),
            bus_conversion_time: conversion_time_setting(value.bus_conversion_time),
            shunt_conversion_time: conversion_time_setting(value.shunt_conversion_time),
            mode: mode(value.mode),
        }
    }
}

impl From<ina226::Configuration> for PamojaIna226Config {
    fn from(value: ina226::Configuration) -> Self {
        PamojaIna226Config {
            reset: u8::from(value.reset),
            averaging: value.averaging as u8,
            bus_conversion_time: value.bus_conversion_time as u8,
            shunt_conversion_time: value.shunt_conversion_time as u8,
            mode: value.mode as u8,
        }
    }
}

impl From<PamojaIna226MaskEnable> for ina226::MaskEnable {
    fn from(value: PamojaIna226MaskEnable) -> Self {
        ina226::MaskEnable {
            shunt_over_limit: value.shunt_over_limit != 0,
            shunt_under_limit: value.shunt_under_limit != 0,
            bus_over_limit: value.bus_over_limit != 0,
            bus_under_limit: value.bus_under_limit != 0,
            power_over_limit: value.power_over_limit != 0,
            conversion_ready: value.conversion_ready != 0,
            alert_function_flag: value.alert_function_flag != 0,
            conversion_ready_flag: value.conversion_ready_flag != 0,
            math_overflow: value.math_overflow != 0,
            alert_active_high: value.alert_active_high != 0,
            alert_latch: value.alert_latch != 0,
        }
    }
}

impl From<ina226::MaskEnable> for PamojaIna226MaskEnable {
    fn from(value: ina226::MaskEnable) -> Self {
        PamojaIna226MaskEnable {
            shunt_over_limit: u8::from(value.shunt_over_limit),
            shunt_under_limit: u8::from(value.shunt_under_limit),
            bus_over_limit: u8::from(value.bus_over_limit),
            bus_under_limit: u8::from(value.bus_under_limit),
            power_over_limit: u8::from(value.power_over_limit),
            conversion_ready: u8::from(value.conversion_ready),
            alert_function_flag: u8::from(value.alert_function_flag),
            conversion_ready_flag: u8::from(value.conversion_ready_flag),
            math_overflow: u8::from(value.math_overflow),
            alert_active_high: u8::from(value.alert_active_high),
            alert_latch: u8::from(value.alert_latch),
        }
    }
}

impl From<ina226::AlertFunction> for PamojaIna226AlertFunction {
    fn from(value: ina226::AlertFunction) -> Self {
        match value {
            ina226::AlertFunction::ShuntOverLimit => Self::ShuntOverLimit,
            ina226::AlertFunction::ShuntUnderLimit => Self::ShuntUnderLimit,
            ina226::AlertFunction::BusOverLimit => Self::BusOverLimit,
            ina226::AlertFunction::BusUnderLimit => Self::BusUnderLimit,
            ina226::AlertFunction::PowerOverLimit => Self::PowerOverLimit,
        }
    }
}

impl From<ina226::DieId> for PamojaIna226DieId {
    fn from(value: ina226::DieId) -> Self {
        PamojaIna226DieId {
            device: value.device,
            revision: value.revision,
        }
    }
}

/// Maps a code onto the SHT3x repeatability it names.
fn repeatability_from_code(code: u8) -> Option<sht3x::Repeatability> {
    match code {
        0 => Some(sht3x::Repeatability::Low),
        1 => Some(sht3x::Repeatability::Medium),
        2 => Some(sht3x::Repeatability::High),
        _ => None,
    }
}

/// Records a rejected repeatability and reports it as an invalid argument.
fn bad_repeatability() -> PamojaStatus {
    set_last_error("SHT3x repeatability must be 0 low, 1 medium, or 2 high".to_owned());
    PamojaStatus::InvalidArgument
}

/// Maps a code onto the SHT3x periodic-mode rate it names.
fn rate_from_code(code: u8) -> Option<sht3x::Rate> {
    match code {
        0 => Some(sht3x::Rate::HalfMps),
        1 => Some(sht3x::Rate::OneMps),
        2 => Some(sht3x::Rate::TwoMps),
        3 => Some(sht3x::Rate::FourMps),
        4 => Some(sht3x::Rate::TenMps),
        _ => None,
    }
}

/// Records a rejected rate and reports it as an invalid argument.
fn bad_rate() -> PamojaStatus {
    set_last_error("SHT3x rate must be 0 for 0.5, 1, 2, 3, or 4 for 10 per second".to_owned());
    PamojaStatus::InvalidArgument
}

/// Maps a bit count onto the HDC1080 temperature resolution it names.
fn temperature_resolution(bits: u8) -> Option<hdc1080::TemperatureResolution> {
    match bits {
        14 => Some(hdc1080::TemperatureResolution::Bits14),
        11 => Some(hdc1080::TemperatureResolution::Bits11),
        _ => None,
    }
}

/// Records a rejected temperature resolution and reports it as an invalid argument.
fn bad_temperature_resolution() -> PamojaStatus {
    set_last_error("HDC1080 temperature resolution must be 14 or 11 bits".to_owned());
    PamojaStatus::InvalidArgument
}

/// Maps a bit count onto the HDC1080 humidity resolution it names.
fn humidity_resolution(bits: u8) -> Option<hdc1080::HumidityResolution> {
    match bits {
        14 => Some(hdc1080::HumidityResolution::Bits14),
        11 => Some(hdc1080::HumidityResolution::Bits11),
        8 => Some(hdc1080::HumidityResolution::Bits8),
        _ => None,
    }
}

/// Records a rejected humidity resolution and reports it as an invalid argument.
fn bad_humidity_resolution() -> PamojaStatus {
    set_last_error("HDC1080 humidity resolution must be 14, 11, or 8 bits".to_owned());
    PamojaStatus::InvalidArgument
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
fn conversion_time_setting(code: u8) -> ina226::ConversionTime {
    ina226::Configuration::from_register(u16::from(code & 0x07) << 6).bus_conversion_time
}

/// Maps a mode code onto the INA226 setting it names.
fn mode(code: u8) -> ina226::Mode {
    ina226::Configuration::from_register(u16::from(code & 0x07)).mode
}

/// Maps a pin code onto the level an INA226 address pin is tied to.
fn address_pin(code: u8) -> Option<ina226::AddressPin> {
    match code {
        0 => Some(ina226::AddressPin::Ground),
        1 => Some(ina226::AddressPin::Supply),
        2 => Some(ina226::AddressPin::Sda),
        3 => Some(ina226::AddressPin::Scl),
        _ => None,
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

    #[test]
    fn a_bmp280_burst_read_compensates_and_its_coefficients_round_trip() {
        // The calibration block and burst read from the crate's own datasheet case.
        let calibration_bytes = [
            0x70, 0x6B, 0x43, 0x67, 0x18, 0xFC, 0x7D, 0x8E, 0x43, 0xD6, 0xD0, 0x0B, 0x27, 0x0B,
            0x8C, 0x00, 0xF9, 0xFF, 0x8C, 0x3C, 0xF8, 0xC6, 0x70, 0x17,
        ];
        let measurement = [0x65u8, 0x5A, 0xC0, 0x7E, 0xED, 0x00];
        let mut calibration = ptr::null_mut();
        let mut reading = PamojaBmp280Reading {
            celsius: 0.0,
            pascals: 0,
            hectopascals: 0.0,
        };
        let mut coefficients = [0u8; PAMOJA_BMP280_CALIBRATION_LEN];

        // Safety: the inputs are valid slices and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_bmp280_calibration_new(
                    calibration_bytes.as_ptr(),
                    calibration_bytes.len(),
                    &mut calibration
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_bmp280_compensate(
                    calibration,
                    measurement.as_ptr(),
                    measurement.len(),
                    &mut reading
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_bmp280_calibration_to_bytes(calibration, coefficients.as_mut_ptr()),
                PamojaStatus::Ok
            );
            pamoja_bmp280_calibration_free(calibration);
        }
        assert_eq!(reading.pascals, 100_653);
        assert!((reading.celsius - 25.08).abs() < 1e-2);
        assert_eq!(
            coefficients, calibration_bytes,
            "the coefficients round-trip through their registers"
        );
    }

    #[test]
    fn a_bmp280_control_register_round_trips() {
        let ctrl = PamojaBmp280CtrlMeas {
            temperature: 2,
            pressure: 5,
            mode: 3,
        };
        let bits = pamoja_bmp280_ctrl_meas_bits(ctrl);
        let mut back = ctrl;
        // Safety: the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_bmp280_ctrl_meas_from_bits(bits, &mut back),
                PamojaStatus::Ok
            );
        }
        assert_eq!(back, ctrl);
        assert_eq!(pamoja_bmp280_oversampling_factor(5), 16);
        assert!(pamoja_bmp280_measuring(0x08));
    }

    #[test]
    fn an_sht3x_frame_decodes_and_its_crc_is_checked() {
        // Two fifths of full scale is 25 C, three fifths is 60 percent.
        let frame = [0x66u8, 0x66, 0x93, 0x99, 0x99, 0xBE];
        let mut measurement = PamojaSht3xMeasurement {
            temperature_raw: 0,
            humidity_raw: 0,
            milli_celsius: 0,
            celsius: 0.0,
            milli_fahrenheit: 0,
            fahrenheit: 0.0,
            milli_percent: 0,
            relative_humidity: 0.0,
        };
        // Safety: the input is a valid slice and the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_sht3x_parse_measurement(frame.as_ptr(), frame.len(), &mut measurement),
                PamojaStatus::Ok
            );
        }
        assert_eq!(measurement.milli_celsius, 25_000);
        assert_eq!(measurement.milli_fahrenheit, 77_000);
        assert_eq!(measurement.milli_percent, 60_000);

        let check = [0xBEu8, 0xEF];
        // Safety: the input is a valid slice.
        assert_eq!(
            unsafe { pamoja_sht3x_crc(check.as_ptr(), check.len()) },
            0x92,
            "Sensirion's published check value"
        );

        let corrupt = [0x66u8, 0x67, 0x93, 0x99, 0x99, 0xBE];
        // Safety: the input is a valid slice and the out-pointer is writable.
        let status = unsafe {
            pamoja_sht3x_parse_measurement(corrupt.as_ptr(), corrupt.len(), &mut measurement)
        };
        assert_eq!(
            status,
            PamojaStatus::Codec,
            "a read corrupted on the bus must not be trusted"
        );
    }

    #[test]
    fn an_sht3x_status_and_command_follow_the_tables() {
        let frame = [0x80u8, 0x10, 0xE1];
        let mut status = PamojaSht3xStatus {
            bits: 0,
            alert_pending: 0,
            heater_on: 0,
            humidity_tracking_alert: 0,
            temperature_tracking_alert: 0,
            reset_detected: 0,
            command_failed: 0,
            write_checksum_failed: 0,
        };
        let mut command = 0u16;
        // Safety: the input is a valid slice and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_sht3x_parse_status(frame.as_ptr(), frame.len(), &mut status),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_sht3x_single_shot(2, true, &mut command),
                PamojaStatus::Ok
            );
        }
        assert_eq!(status.bits, PAMOJA_SHT3X_STATUS_DEFAULT);
        assert_eq!(status.alert_pending, 1);
        assert_eq!(status.reset_detected, 1);
        assert_eq!(status.heater_on, 0);
        assert_eq!(command, PAMOJA_SHT3X_COMMAND_SINGLE_SHOT_HIGH_STRETCH);

        // Safety: the out-pointer is writable.
        let status = unsafe { pamoja_sht3x_single_shot(3, true, &mut command) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
    }

    #[test]
    fn an_scd4x_frame_decodes_and_its_crc_is_checked() {
        let frame = [0x01u8, 0xF4, 0x33, 0x66, 0x67, 0xA2, 0x5E, 0xB9, 0x3C];
        let mut measurement = PamojaScd4xMeasurement {
            co2_ppm: 0,
            temperature_raw: 0,
            humidity_raw: 0,
            milli_celsius: 0,
            celsius: 0.0,
            humidity_milli_percent: 0,
            relative_humidity_percent: 0.0,
        };
        // Safety: the input is a valid slice and the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_scd4x_parse_measurement(frame.as_ptr(), frame.len(), &mut measurement),
                PamojaStatus::Ok
            );
        }
        assert_eq!(measurement.co2_ppm, 500);
        assert_eq!(measurement.milli_celsius, 25_003);
        assert_eq!(measurement.humidity_milli_percent, 37_002);

        let corrupt = [0x01u8, 0xF4, 0xCC, 0x66, 0x67, 0xA2, 0x5E, 0xB9, 0x3C];
        // Safety: the input is a valid slice and the out-pointer is writable.
        let status = unsafe {
            pamoja_scd4x_parse_measurement(corrupt.as_ptr(), corrupt.len(), &mut measurement)
        };
        assert_eq!(status, PamojaStatus::Codec);

        // The offset scales by 2^16, not by the 2^16 - 1 the measurement words use.
        assert_eq!(pamoja_scd4x_temperature_offset_word(5_400), 0x07E6);

        let serial_frame = [0xF8u8, 0x96, 0x31, 0x9F, 0x07, 0xC2, 0x3B, 0xBE, 0x89];
        let mut serial = 0u64;
        // Safety: the input is a valid slice and the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_scd4x_serial_number(serial_frame.as_ptr(), serial_frame.len(), &mut serial),
                PamojaStatus::Ok
            );
        }
        assert_eq!(serial, 273_325_796_834_238);
    }

    #[test]
    fn a_tmp117_register_decodes_to_its_datasheet_row() {
        assert_eq!(pamoja_tmp117_micro_celsius(0x0C80), 25_000_000);
        assert_eq!(pamoja_tmp117_nano_celsius(-1), -7_812_500);
        assert_eq!(pamoja_tmp117_raw_from_micro_celsius(-25_000_000), -3_200);
        assert_eq!(pamoja_tmp117_device_id(0x1117), 0x117);
        assert_eq!(pamoja_tmp117_revision(0x1117), 1);
        assert!(pamoja_tmp117_high_alert(0xA220));
        assert!(!pamoja_tmp117_low_alert(0xA220));
        assert!(pamoja_tmp117_data_ready(0xA220));

        let mut config = PamojaTmp117Config {
            high_alert: 0,
            low_alert: 0,
            data_ready: 0,
            eeprom_busy: 0,
            mode: 0,
            cycle: 0,
            averaging: 0,
            therm_mode: 0,
            alert_active_high: 0,
            alert_pin_data_ready: 0,
            soft_reset: 0,
        };
        // Safety: the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_tmp117_config_from_bits(PAMOJA_TMP117_CONFIG_RESET, &mut config),
                PamojaStatus::Ok
            );
        }
        assert_eq!(
            pamoja_tmp117_config_bits(config),
            PAMOJA_TMP117_CONFIG_RESET
        );
        assert_eq!(pamoja_tmp117_cycle_micros(5, 0), 4_000_000);
    }

    #[test]
    fn an_hdc1080_measurement_decodes_and_an_undefined_code_is_refused() {
        let bytes = [0x60u8, 0x00, 0x40, 0x00];
        let mut measurement = PamojaHdc1080Measurement {
            temperature_raw: 0,
            humidity_raw: 0,
            milli_celsius: 0,
            celsius: 0.0,
            milli_percent: 0,
            relative_humidity: 0.0,
        };
        let mut config = PamojaHdc1080Config {
            software_reset: 0,
            heater: 0,
            sequential: 0,
            battery_low: 0,
            temperature_resolution_bits: 0,
            humidity_resolution_bits: 0,
        };
        let mut register = 0u16;
        // Safety: the input is a valid slice and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_hdc1080_parse_measurement(bytes.as_ptr(), bytes.len(), &mut measurement),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_hdc1080_config_from_register(
                    PAMOJA_HDC1080_CONFIGURATION_RESET,
                    &mut config
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_hdc1080_config_to_register(config, &mut register),
                PamojaStatus::Ok
            );
        }
        assert_eq!(measurement.milli_celsius, 21_875);
        assert_eq!(measurement.milli_percent, 25_000);
        assert_eq!(register, PAMOJA_HDC1080_CONFIGURATION_RESET);

        // Safety: the out-pointer is writable.
        let status = unsafe { pamoja_hdc1080_config_from_register(0x1300, &mut config) };
        assert_eq!(
            status,
            PamojaStatus::Codec,
            "the humidity-resolution code the datasheet leaves undefined is refused"
        );
    }

    #[test]
    fn an_opt3001_result_decodes_and_a_reserved_range_has_no_full_scale() {
        assert_eq!(pamoja_opt3001_milli_lux(0xBFFF), 83_865_600);
        assert_eq!(pamoja_opt3001_raw_from_milli_lux(88_800), 0x28AC);

        let mut full_scale = 0u32;
        // Safety: the out-pointer is writable.
        unsafe {
            assert!(pamoja_opt3001_full_scale_milli_lux(0, &mut full_scale));
            assert_eq!(full_scale, 40_950);
            assert!(
                !pamoja_opt3001_full_scale_milli_lux(
                    PAMOJA_OPT3001_RANGE_AUTOMATIC,
                    &mut full_scale
                ),
                "a reserved range number has no full scale"
            );
        }

        let mut config = PamojaOpt3001Config {
            range_number: 0,
            long_conversion: 0,
            mode: 0,
            overflow: 0,
            conversion_ready: 0,
            flag_high: 0,
            flag_low: 0,
            latched_window: 0,
            active_high: 0,
            mask_exponent: 0,
            fault_count: 0,
        };
        // Safety: the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_opt3001_config_from_bits(PAMOJA_OPT3001_CONFIGURATION_RESET, &mut config),
                PamojaStatus::Ok
            );
        }
        assert_eq!(
            pamoja_opt3001_config_bits(config),
            PAMOJA_OPT3001_CONFIGURATION_RESET
        );
    }

    #[test]
    fn an_ina226_converts_at_its_calibrated_resolution_and_checks_its_die() {
        // The datasheet's worked design example: 15 A across a 2 milliohm shunt at
        // 1 mA per count.
        const CURRENT_LSB: u32 = 1_000;
        assert_eq!(pamoja_ina226_calibration(CURRENT_LSB, 2), 2_560);
        assert_eq!(pamoja_ina226_minimum_current_lsb_microamps(15_000_000), 458);
        assert_eq!(pamoja_ina226_shunt_nanovolts(8_000), 20_000_000);
        assert_eq!(pamoja_ina226_bus_microvolts(9_584), 11_980_000);
        assert_eq!(
            pamoja_ina226_current_microamps(10_000, CURRENT_LSB),
            10_000_000
        );
        // The power LSB is fixed at twenty-five times the current LSB.
        assert_eq!(
            pamoja_ina226_power_microwatts(4_792, CURRENT_LSB),
            119_800_000
        );

        let mut die = PamojaIna226DieId {
            device: 0,
            revision: 0,
        };
        // Safety: the out-pointer is writable.
        unsafe {
            assert_eq!(
                pamoja_ina226_identify(PAMOJA_INA226_MANUFACTURER_ID, 0x2260, &mut die),
                PamojaStatus::Ok
            );
            assert_eq!(die.device, PAMOJA_INA226_DEVICE_ID);
            assert_eq!(
                pamoja_ina226_identify(PAMOJA_INA226_MANUFACTURER_ID, 0x2270, &mut die),
                PamojaStatus::Codec,
                "a die that is not an INA226 is refused"
            );
        }
    }
}
