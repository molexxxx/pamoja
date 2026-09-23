//! The C ABI for the sensor drivers that run over an I2C bus.
//!
//! A driver runs the datasheet's whole conversation with a part over a [`PamojaI2cBus`]:
//! reset, identify, read the calibration, configure, measure. It holds a share of the bus, so
//! the caller may free its own bus handle straight after building the driver, or keep it to
//! look at the bus between reads. A driver waits as its datasheet asks through the bus's
//! delay, which sleeps only when real parts are on the other end.
//!
//! Every part has a simulated twin that answers a driver with nothing plugged in: a
//! `_sim_part` that reads the datasheet's typical values and a `_sim_reporting` that reads
//! whatever it is asked to. Either goes on a simulated bus with
//! [`pamoja_i2c_bus_attach`](crate::hal::pamoja_i2c_bus_attach).
//!
//! A driver's failure is [`PamojaStatus::Io`] when the bus failed or the part never finished,
//! and [`PamojaStatus::Codec`] when a part answered but is not the one the driver expected or
//! its checksum failed, with the reason in the last error message either way.
//!
//! The DS18B20 is the exception: on a Linux board it is not driven over a bus but read through
//! the files the kernel's 1-Wire driver serves, which [`pamoja_ds18b20_thermometer_read`] does.

use std::fmt::{Debug, Display};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

use pamoja_hal::bus::{BusDelay, I2cBus};
use pamoja_sensors::driver::I2cRegisters;
use pamoja_sensors::{
    ads1115, bme280, bmp280, ds18b20, hdc1080, ina219, ina226, opt3001, scd4x, sht3x, tmp117,
    DriverError,
};

use crate::hal::{PamojaI2cBus, PamojaI2cPart};
use crate::sensors::{
    conversion_time, humidity_resolution, repeatability_from_code, temperature_resolution,
    PamojaAds1115Config, PamojaBme280Measurement, PamojaBmp280Coefficients, PamojaBmp280Reading,
    PamojaDs18b20Reading, PamojaHdc1080Config, PamojaHdc1080Measurement, PamojaIna219Config,
    PamojaIna226Config, PamojaIna226DieId, PamojaIna226MaskEnable, PamojaOpt3001Config,
    PamojaScd4xMeasurement, PamojaSht3xMeasurement, PamojaSht3xStatus,
    PAMOJA_BME280_CALIBRATION_HUMIDITY_LEN, PAMOJA_BME280_CALIBRATION_TEMP_PRESS_LEN,
    PAMOJA_BME280_MEASUREMENT_LEN, PAMOJA_BMP280_CALIBRATION_LEN, PAMOJA_BMP280_DATA_LEN,
};
use crate::{read_str, set_last_error, PamojaStatus, PamojaString};

type Bme280Driver = bme280::Bme280<I2cRegisters<I2cBus>, BusDelay>;
type Bmp280Driver = bmp280::Bmp280<I2cRegisters<I2cBus>, BusDelay>;
type Tmp117Driver = tmp117::Tmp117<I2cBus, BusDelay>;
type Opt3001Driver = opt3001::Opt3001<I2cBus, BusDelay>;
type Hdc1080Driver = hdc1080::Hdc1080<I2cBus, BusDelay>;
type Ina219Driver = ina219::Ina219<I2cBus, BusDelay>;
type Ina226Driver = ina226::Ina226<I2cBus, BusDelay>;
type Ads1115Driver = ads1115::Ads1115<I2cBus, BusDelay>;
type Sht3xDriver = sht3x::Sht3x<I2cBus, BusDelay>;
type Scd4xDriver = scd4x::Scd4x<I2cBus, BusDelay>;

/// The status a simulated BME280 reports when it is neither measuring nor loading its
/// calibration.
pub const PAMOJA_BME280_SIM_STATUS_IDLE: u8 = 0x00;

/// The status a simulated BMP280 reports when it is neither measuring nor loading its
/// calibration.
pub const PAMOJA_BMP280_SIM_STATUS_IDLE: u8 = 0x00;

/// The temperature a simulated TMP117 reports unless it is asked for another.
pub const PAMOJA_TMP117_SIM_CELSIUS: f32 = 21.25;

/// The illuminance a simulated OPT3001 reports unless it is asked for another.
pub const PAMOJA_OPT3001_SIM_LUX: f32 = 380.0;

/// The temperature a simulated HDC1080 reports unless it is asked for another.
pub const PAMOJA_HDC1080_SIM_CELSIUS: f32 = 22.5;

/// The relative humidity a simulated HDC1080 reports unless it is asked for another.
pub const PAMOJA_HDC1080_SIM_RELATIVE_HUMIDITY: f32 = 45.0;

/// The shunt a simulated INA219 sits across, in milliohms: the common breakout's.
pub const PAMOJA_INA219_SIM_SHUNT_MILLIOHMS: u32 = 100;

/// The largest current a simulated INA219 is sized for, in microamps.
pub const PAMOJA_INA219_SIM_MAX_MICROAMPS: u32 = 3_200_000;

/// The bus voltage a simulated INA219 reports unless it is asked for another, in millivolts.
pub const PAMOJA_INA219_SIM_BUS_MILLIVOLTS: u32 = 12_000;

/// The current a simulated INA219 reports unless it is asked for another, in microamps.
pub const PAMOJA_INA219_SIM_MICROAMPS: i32 = 500_000;

/// The shunt a simulated INA226 sits across, in milliohms.
pub const PAMOJA_INA226_SIM_SHUNT_MILLIOHMS: u32 = 100;

/// The largest current a simulated INA226 is sized for, in microamps.
pub const PAMOJA_INA226_SIM_MAX_MICROAMPS: u32 = 3_200_000;

/// The bus voltage a simulated INA226 reports unless it is asked for another, in microvolts.
pub const PAMOJA_INA226_SIM_BUS_MICROVOLTS: u32 = 12_000_000;

/// The current a simulated INA226 reports unless it is asked for another, in microamps.
pub const PAMOJA_INA226_SIM_MICROAMPS: i32 = 500_000;

/// The voltage a simulated ADS1115 reports unless it is asked for another: half a 3.3 V
/// supply.
pub const PAMOJA_ADS1115_SIM_VOLTS: f32 = 1.65;

/// The temperature a simulated SHT3x reports unless it is asked for another.
pub const PAMOJA_SHT3X_SIM_CELSIUS: f32 = 22.5;

/// The relative humidity a simulated SHT3x reports unless it is asked for another.
pub const PAMOJA_SHT3X_SIM_RELATIVE_HUMIDITY: f32 = 45.0;

/// The carbon dioxide a simulated SCD4x reports unless it is asked for another, in parts per
/// million.
pub const PAMOJA_SCD4X_SIM_CO2_PPM: u16 = 800;

/// The temperature a simulated SCD4x reports unless it is asked for another.
pub const PAMOJA_SCD4X_SIM_CELSIUS: f32 = 22.5;

/// The relative humidity a simulated SCD4x reports unless it is asked for another.
pub const PAMOJA_SCD4X_SIM_RELATIVE_HUMIDITY: f32 = 45.0;

/// The serial number every simulated SCD4x reports.
pub const PAMOJA_SCD4X_SIM_SERIAL: u64 = 0x0000_5A4D_0C1E_2B3F;

const _: () = assert!(PAMOJA_BME280_SIM_STATUS_IDLE == bme280::sim::STATUS_IDLE);
const _: () = assert!(PAMOJA_BMP280_SIM_STATUS_IDLE == bmp280::sim::STATUS_IDLE);
const _: () = assert!(PAMOJA_TMP117_SIM_CELSIUS == tmp117::sim::CELSIUS);
const _: () = assert!(PAMOJA_OPT3001_SIM_LUX == opt3001::sim::LUX);
const _: () = assert!(PAMOJA_HDC1080_SIM_CELSIUS == hdc1080::sim::CELSIUS);
const _: () = assert!(PAMOJA_HDC1080_SIM_RELATIVE_HUMIDITY == hdc1080::sim::RELATIVE_HUMIDITY);
const _: () = assert!(PAMOJA_INA219_SIM_SHUNT_MILLIOHMS == ina219::sim::SHUNT_MILLIOHMS);
const _: () = assert!(PAMOJA_INA219_SIM_MAX_MICROAMPS == ina219::sim::MAX_MICROAMPS);
const _: () = assert!(PAMOJA_INA219_SIM_BUS_MILLIVOLTS == ina219::sim::BUS_MILLIVOLTS);
const _: () = assert!(PAMOJA_INA219_SIM_MICROAMPS == ina219::sim::MICROAMPS);
const _: () = assert!(PAMOJA_INA226_SIM_SHUNT_MILLIOHMS == ina226::sim::SHUNT_MILLIOHMS);
const _: () = assert!(PAMOJA_INA226_SIM_MAX_MICROAMPS == ina226::sim::MAX_MICROAMPS);
const _: () = assert!(PAMOJA_INA226_SIM_BUS_MICROVOLTS == ina226::sim::BUS_MICROVOLTS);
const _: () = assert!(PAMOJA_INA226_SIM_MICROAMPS == ina226::sim::MICROAMPS);
const _: () = assert!(PAMOJA_ADS1115_SIM_VOLTS == ads1115::sim::VOLTS);
const _: () = assert!(PAMOJA_SHT3X_SIM_CELSIUS == sht3x::sim::CELSIUS);
const _: () = assert!(PAMOJA_SHT3X_SIM_RELATIVE_HUMIDITY == sht3x::sim::RELATIVE_HUMIDITY);
const _: () = assert!(PAMOJA_SCD4X_SIM_CO2_PPM == scd4x::sim::CO2_PPM);
const _: () = assert!(PAMOJA_SCD4X_SIM_CELSIUS == scd4x::sim::CELSIUS);
const _: () = assert!(PAMOJA_SCD4X_SIM_RELATIVE_HUMIDITY == scd4x::sim::RELATIVE_HUMIDITY);
const _: () = assert!(PAMOJA_SCD4X_SIM_SERIAL == scd4x::sim::SERIAL);

/// How a BME280 driver measures: the oversampling of each measurement and the IIR filter.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaBme280Settings {
    /// The temperature oversampling code, `0..=5`, where `0` skips the measurement.
    pub temperature: u8,
    /// The pressure oversampling code, `0..=5`, where `0` skips the measurement.
    pub pressure: u8,
    /// The humidity oversampling code, `0..=5`, where `0` skips the measurement.
    pub humidity: u8,
    /// The IIR filter code, `0..=4`, where `0` is off.
    pub filter: u8,
}

/// How a BMP280 driver measures: the oversampling of each measurement and the IIR filter.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaBmp280Settings {
    /// The temperature oversampling code, `0..=5`, where `0` skips the measurement.
    pub temperature: u8,
    /// The pressure oversampling code, `0..=5`, where `0` skips the measurement.
    pub pressure: u8,
    /// The IIR filter's `filter[2:0]` code, `0` for the filter off, written as given.
    pub filter: u8,
}

/// A TMP117 temperature result.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaTmp117Reading {
    /// The temperature register, 7.8125 millidegrees Celsius per count.
    pub raw: i16,
    /// The temperature in micro-degrees Celsius, exact in integer arithmetic.
    pub micro_celsius: i32,
    /// The temperature in degrees Celsius.
    pub celsius: f32,
}

/// The TMP117's alert flags: whether a result since they were last read crossed a limit.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaTmp117Alerts {
    /// `1` when a result was above the high limit.
    pub high: u8,
    /// `1` when a result was below the low limit.
    pub low: u8,
}

/// How an OPT3001 driver measures.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaOpt3001Settings {
    /// `1` integrates each conversion for 800 ms, for resolution; `0` for 100 ms, for speed.
    pub long_conversion: u8,
    /// The full-scale range number, `0..=11`, or `12` to let the part choose.
    pub range_number: u8,
}

/// An OPT3001 illuminance result.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaOpt3001Reading {
    /// The result register: a four-bit exponent over a twelve-bit mantissa.
    pub raw: u16,
    /// The illuminance in millilux, exact in integer arithmetic.
    pub milli_lux: u32,
    /// The illuminance in lux.
    pub lux: f32,
}

/// How an HDC1080 driver measures: the resolution of each channel.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaHdc1080Settings {
    /// The temperature resolution in bits: 14 or 11.
    pub temperature_resolution_bits: u8,
    /// The humidity resolution in bits: 14, 11, or 8.
    pub humidity_resolution_bits: u8,
}

/// How an INA219 driver measures: the shunt, the current it is sized for, and the register
/// settings.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaIna219Settings {
    /// The shunt resistance in milliohms.
    pub shunt_milliohms: u32,
    /// The largest current the shunt will carry, in microamps, which sets the finest current
    /// step the calibration allows.
    pub max_microamps: u32,
    /// A current step in microamps per count to use instead, such as a round 100, or `0` to
    /// take the finest step for `max_microamps`.
    pub current_lsb_microamps: u32,
    /// The range, gain, and converter settings; the mode is chosen per conversion.
    pub config: PamojaIna219Config,
}

/// One INA219 conversion: the four result registers as read, and what they mean.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaIna219Reading {
    /// The shunt-voltage register.
    pub shunt: i16,
    /// The bus-voltage register, flags included.
    pub bus: u16,
    /// The current register.
    pub current: i16,
    /// The power register.
    pub power: u16,
    /// The current step the calibration programmed, in microamps per count.
    pub current_lsb_microamps: u32,
    /// The shunt voltage in microvolts.
    pub shunt_microvolts: i32,
    /// The bus voltage in millivolts.
    pub bus_millivolts: u32,
    /// The current in microamps; negative flows the other way through the shunt.
    pub current_microamps: i32,
    /// The power in microwatts.
    pub power_microwatts: u32,
    /// `1` when the part's arithmetic overflowed and the current and power are meaningless.
    pub math_overflow: u8,
}

/// How an INA226 driver measures: the shunt, the current it is sized for, and the register
/// settings.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaIna226Settings {
    /// The shunt resistance in milliohms.
    pub shunt_milliohms: u32,
    /// The largest current the shunt will carry, in microamps, which sets the finest current
    /// step the calibration allows.
    pub max_microamps: u32,
    /// A current step in microamps per count to use instead, or `0` to take the finest step
    /// for `max_microamps`.
    pub current_lsb_microamps: u32,
    /// The averaging and conversion times; the mode is chosen per conversion.
    pub config: PamojaIna226Config,
}

/// One INA226 conversion: the four result registers as read, and what they mean.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaIna226Reading {
    /// The shunt-voltage register.
    pub shunt: i16,
    /// The bus-voltage register.
    pub bus: u16,
    /// The current register.
    pub current: i16,
    /// The power register.
    pub power: u16,
    /// The current step the calibration programmed, in microamps per count.
    pub current_lsb_microamps: u32,
    /// The shunt voltage in nanovolts.
    pub shunt_nanovolts: i32,
    /// The shunt voltage in millivolts.
    pub shunt_millivolts: f32,
    /// The bus voltage in microvolts.
    pub bus_microvolts: u32,
    /// The bus voltage in volts.
    pub bus_volts: f32,
    /// The current in microamps; negative flows the other way through the shunt.
    pub current_microamps: i32,
    /// The current in amps.
    pub current_amps: f32,
    /// The power in microwatts.
    pub power_microwatts: u32,
    /// The power in watts.
    pub power_watts: f32,
    /// `1` when the part's arithmetic overflowed and the current and power are meaningless.
    pub math_overflow: u8,
}

/// How an ADS1115 driver converts: which input, at which range, how fast.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaAds1115Settings {
    /// The input multiplexer code, `0..=7`: `0..=3` a differential pair, `4..=7` one input
    /// against ground.
    pub mux: u8,
    /// The gain code, `0..=7`, which sets the full-scale range.
    pub pga: u8,
    /// The data-rate code, `0..=7`, from 8 to 860 samples per second.
    pub data_rate: u8,
}

/// One ADS1115 conversion.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaAds1115Sample {
    /// The conversion register, two's complement.
    pub raw: i16,
    /// The gain code the conversion ran at.
    pub pga: u8,
    /// The voltage in nanovolts, exact in integer arithmetic.
    pub nanovolts: i64,
    /// The voltage in volts.
    pub volts: f32,
}

/// A BME280 driven over an I2C bus. Opaque; release it with [`pamoja_bme280_free`].
pub struct PamojaBme280 {
    sensor: Bme280Driver,
}

/// A BMP280 driven over an I2C bus. Opaque; release it with [`pamoja_bmp280_free`].
pub struct PamojaBmp280 {
    sensor: Bmp280Driver,
}

/// A TMP117 driven over an I2C bus. Opaque; release it with [`pamoja_tmp117_free`].
pub struct PamojaTmp117 {
    sensor: Tmp117Driver,
}

/// An OPT3001 driven over an I2C bus. Opaque; release it with [`pamoja_opt3001_free`].
pub struct PamojaOpt3001 {
    sensor: Opt3001Driver,
}

/// An HDC1080 driven over an I2C bus. Opaque; release it with [`pamoja_hdc1080_free`].
pub struct PamojaHdc1080 {
    sensor: Hdc1080Driver,
}

/// An INA219 driven over an I2C bus. Opaque; release it with [`pamoja_ina219_free`].
pub struct PamojaIna219 {
    sensor: Ina219Driver,
}

/// An INA226 driven over an I2C bus. Opaque; release it with [`pamoja_ina226_free`].
pub struct PamojaIna226 {
    sensor: Ina226Driver,
}

/// An ADS1115 driven over an I2C bus. Opaque; release it with [`pamoja_ads1115_free`].
pub struct PamojaAds1115 {
    sensor: Ads1115Driver,
}

/// An SHT3x driven over an I2C bus. Opaque; release it with [`pamoja_sht3x_free`].
pub struct PamojaSht3x {
    sensor: Sht3xDriver,
}

/// An SCD40 or SCD41 driven over an I2C bus. Opaque; release it with [`pamoja_scd4x_free`].
pub struct PamojaScd4x {
    sensor: Scd4xDriver,
}

/// A DS18B20 the Linux kernel serves as a `w1_slave` file. Opaque; release it with
/// [`pamoja_ds18b20_thermometer_free`].
pub struct PamojaDs18b20Thermometer {
    thermometer: ds18b20::linux::Thermometer,
}

/// The DS18B20s the kernel has found, in order of their directory names. Opaque; release it
/// with [`pamoja_ds18b20_thermometers_free`].
pub struct PamojaDs18b20Thermometers {
    found: Vec<ds18b20::linux::Thermometer>,
}

/// Returns the settings a BME280 driver starts with: every measurement at oversampling x1 and
/// the filter off.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_bme280_settings_default() -> PamojaBme280Settings {
    PamojaBme280Settings {
        temperature: bme280::Oversampling::X1.code(),
        pressure: bme280::Oversampling::X1.code(),
        humidity: bme280::Oversampling::X1.code(),
        filter: bme280::Filter::Off.code(),
    }
}

/// Creates a BME280 driver on a bus. Nothing is sent until [`pamoja_bme280_init`] or the first
/// [`pamoja_bme280_measure`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `address` - [`crate::sensors::PAMOJA_BME280_I2C_ADDRESS_PRIMARY`] with SDO low, or
///   [`crate::sensors::PAMOJA_BME280_I2C_ADDRESS_SECONDARY`] with SDO high.
/// * `settings` - the oversampling and the filter.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_new(
    bus: *const PamojaI2cBus,
    address: u8,
    settings: PamojaBme280Settings,
    out_sensor: *mut *mut PamojaBme280,
) -> PamojaStatus {
    build(bus, out_sensor, |bus, delay| PamojaBme280 {
        sensor: bme280::Bme280::i2c(bus, address, delay)
            .with_oversampling(
                bme280::Oversampling::from_code(settings.temperature),
                bme280::Oversampling::from_code(settings.pressure),
                bme280::Oversampling::from_code(settings.humidity),
            )
            .with_filter(bme280::Filter::from_code(settings.filter)),
    })
}

/// Resets the part, checks it is a BME280, reads its calibration, and writes the settings, in
/// the order the datasheet requires, leaving the part asleep.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails or the calibration never finishes loading; or
/// [`PamojaStatus::Codec`] when the part at the address is not a BME280.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_init(sensor: *mut PamojaBme280) -> PamojaStatus {
    run(sensor, |held| held.sensor.init())
}

/// Runs one forced measurement and compensates it, initializing the part first if
/// [`pamoja_bme280_init`] has not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_measurement` - receives the reading.
///
/// # Returns
///
/// As [`pamoja_bme280_init`], with [`PamojaStatus::Io`] also when the part is still measuring
/// after the datasheet's time.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_measurement` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_measure(
    sensor: *mut PamojaBme280,
    out_measurement: *mut PamojaBme280Measurement,
) -> PamojaStatus {
    fill(
        sensor,
        out_measurement,
        "out_measurement",
        |held| held.sensor.measure(),
        |reading| PamojaBme280Measurement {
            celsius: reading.celsius(),
            pascals: reading.pascals(),
            hectopascals: reading.hectopascals(),
            relative_humidity_percent: reading.relative_humidity_percent(),
        },
    )
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_bme280_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_free(sensor: *mut PamojaBme280) {
    release(sensor);
}

/// Creates a simulated BME280 holding a real part's calibration and one measurement it took,
/// which compensate to 20.44 C, 848.05 hPa, and 44.65 %.
///
/// # Arguments
///
/// * `address` - the address it answers to.
///
/// # Returns
///
/// The part, which the caller puts on a simulated bus and releases with
/// [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_bme280_sim_part(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(bme280::sim::part(address))
}

/// Creates a simulated BME280 that reads what it is asked to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `celsius` - the temperature it reports.
/// * `hectopascals` - the pressure it reports.
/// * `relative_humidity` - the humidity it reports, as a percentage.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`]. Its readings
/// land within a hundredth of a degree, a hundredth of a hectopascal, and a thousandth of a
/// percent of what was asked for, the nearest its converter can represent.
#[no_mangle]
pub extern "C" fn pamoja_bme280_sim_reporting(
    address: u8,
    celsius: f32,
    hectopascals: f32,
    relative_humidity: f32,
) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(bme280::sim::reporting(
        address,
        celsius,
        hectopascals,
        relative_humidity,
    ))
}

/// Copies the calibration a simulated BME280 holds: the two blocks a driver reads at start-up.
///
/// # Arguments
///
/// * `out_temp_press` - receives the 26-byte temperature and pressure block.
/// * `out_humidity` - receives the 7-byte humidity block.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `out_temp_press` must point to 26 writable bytes and `out_humidity` to 7.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_sim_calibration(
    out_temp_press: *mut u8,
    out_humidity: *mut u8,
) -> PamojaStatus {
    if out_temp_press.is_null() || out_humidity.is_null() {
        set_last_error("out_temp_press and out_humidity must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    std::slice::from_raw_parts_mut(out_temp_press, PAMOJA_BME280_CALIBRATION_TEMP_PRESS_LEN)
        .copy_from_slice(&bme280::sim::CALIBRATION);
    std::slice::from_raw_parts_mut(out_humidity, PAMOJA_BME280_CALIBRATION_HUMIDITY_LEN)
        .copy_from_slice(&bme280::sim::CALIBRATION_HUMIDITY);
    PamojaStatus::Ok
}

/// Copies the one measurement a simulated BME280 holds: the eight data registers a real part
/// read, which compensate to 20.44 C, 848.05 hPa, and 44.65 % against its calibration.
///
/// # Arguments
///
/// * `out_burst` - receives the eight bytes a burst read returns.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `out_burst` must point to 8 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_sim_burst(out_burst: *mut u8) -> PamojaStatus {
    copy_out(out_burst, "out_burst", &bme280::sim::BURST)
}

/// Builds the eight data registers a simulated BME280 holds when it reports a reading.
///
/// # Arguments
///
/// * `celsius` - the temperature.
/// * `hectopascals` - the pressure.
/// * `relative_humidity` - the humidity, as a percentage.
/// * `out_burst` - receives the eight bytes a burst read returns.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `out_burst` must point to 8 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bme280_sim_burst_for(
    celsius: f32,
    hectopascals: f32,
    relative_humidity: f32,
    out_burst: *mut u8,
) -> PamojaStatus {
    let burst: [u8; PAMOJA_BME280_MEASUREMENT_LEN] =
        bme280::sim::burst_for(celsius, hectopascals, relative_humidity);
    copy_out(out_burst, "out_burst", &burst)
}

/// Returns the settings a BMP280 driver starts with: both measurements at oversampling x1 and
/// the filter off.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_bmp280_settings_default() -> PamojaBmp280Settings {
    PamojaBmp280Settings {
        temperature: bmp280::Oversampling::X1.code(),
        pressure: bmp280::Oversampling::X1.code(),
        filter: 0,
    }
}

/// Creates a BMP280 driver on a bus. Nothing is sent until [`pamoja_bmp280_init`] or the first
/// [`pamoja_bmp280_measure`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `address` - [`crate::sensors::PAMOJA_BMP280_I2C_ADDRESS_PRIMARY`] with SDO low, or
///   [`crate::sensors::PAMOJA_BMP280_I2C_ADDRESS_SECONDARY`] with SDO high.
/// * `settings` - the oversampling and the filter.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_new(
    bus: *const PamojaI2cBus,
    address: u8,
    settings: PamojaBmp280Settings,
    out_sensor: *mut *mut PamojaBmp280,
) -> PamojaStatus {
    build(bus, out_sensor, |bus, delay| PamojaBmp280 {
        sensor: bmp280::Bmp280::i2c(bus, address, delay)
            .with_oversampling(
                bmp280::Oversampling::from_code(settings.temperature),
                bmp280::Oversampling::from_code(settings.pressure),
            )
            .with_filter(settings.filter),
    })
}

/// Resets the part, checks it is a BMP280, reads its trimming coefficients, and writes the
/// settings, leaving the part asleep.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails or the trimming never finishes loading; or
/// [`PamojaStatus::Codec`] when the part at the address is not a BMP280.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_init(sensor: *mut PamojaBmp280) -> PamojaStatus {
    run(sensor, |held| held.sensor.init())
}

/// Runs one forced measurement and compensates it, initializing the part first if
/// [`pamoja_bmp280_init`] has not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_reading` - receives the reading.
///
/// # Returns
///
/// As [`pamoja_bmp280_init`], with [`PamojaStatus::Io`] also when the part is still measuring
/// after the datasheet's time.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_reading` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_measure(
    sensor: *mut PamojaBmp280,
    out_reading: *mut PamojaBmp280Reading,
) -> PamojaStatus {
    fill(
        sensor,
        out_reading,
        "out_reading",
        |held| held.sensor.measure(),
        |reading| PamojaBmp280Reading {
            celsius: reading.celsius(),
            pascals: reading.pascals(),
            hectopascals: reading.hectopascals(),
        },
    )
}

/// Copies the trimming coefficients a BMP280 driver read at initialization.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_coefficients` - receives the coefficients.
///
/// # Returns
///
/// `true` with the coefficients in `out_coefficients`; `false` before the part has been
/// initialized, or for a null argument.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_coefficients` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_coefficients(
    sensor: *const PamojaBmp280,
    out_coefficients: *mut PamojaBmp280Coefficients,
) -> bool {
    let Some(held) = sensor.as_ref() else {
        return false;
    };
    store(
        out_coefficients,
        held.sensor.calibration().map(|c| (*c).into()),
    )
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_bmp280_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_free(sensor: *mut PamojaBmp280) {
    release(sensor);
}

/// Creates a simulated BMP280 holding a real part's trimming and one measurement it took,
/// which compensate to 20.44 C and 848.05 hPa.
///
/// # Arguments
///
/// * `address` - the address it answers to.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_bmp280_sim_part(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(bmp280::sim::part(address))
}

/// Creates a simulated BMP280 that reads what it is asked to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `celsius` - the temperature it reports.
/// * `hectopascals` - the pressure it reports.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`]. Its readings
/// land within a hundredth of a degree and a hundredth of a hectopascal.
#[no_mangle]
pub extern "C" fn pamoja_bmp280_sim_reporting(
    address: u8,
    celsius: f32,
    hectopascals: f32,
) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(bmp280::sim::reporting(address, celsius, hectopascals))
}

/// Copies the 24 trimming bytes a simulated BMP280 holds.
///
/// # Arguments
///
/// * `out_calibration` - receives the bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `out_calibration` must point to 24 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_sim_calibration(out_calibration: *mut u8) -> PamojaStatus {
    let calibration: [u8; PAMOJA_BMP280_CALIBRATION_LEN] = bmp280::sim::CALIBRATION;
    copy_out(out_calibration, "out_calibration", &calibration)
}

/// Copies the six data registers a simulated BMP280 holds: one measurement a real part took.
///
/// # Arguments
///
/// * `out_burst` - receives the six bytes a burst read returns.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `out_burst` must point to 6 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_sim_burst(out_burst: *mut u8) -> PamojaStatus {
    let burst: [u8; PAMOJA_BMP280_DATA_LEN] = bmp280::sim::BURST;
    copy_out(out_burst, "out_burst", &burst)
}

/// Builds the six data registers a simulated BMP280 holds when it reports a reading.
///
/// # Arguments
///
/// * `celsius` - the temperature.
/// * `hectopascals` - the pressure.
/// * `out_burst` - receives the six bytes a burst read returns.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `out_burst` must point to 6 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_bmp280_sim_burst_for(
    celsius: f32,
    hectopascals: f32,
    out_burst: *mut u8,
) -> PamojaStatus {
    copy_out(
        out_burst,
        "out_burst",
        &bmp280::sim::burst_for(celsius, hectopascals),
    )
}

/// Creates a TMP117 driver on a bus. Nothing is sent until [`pamoja_tmp117_init`] or the first
/// [`pamoja_tmp117_measure`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `address` - one of the `PAMOJA_TMP117_ADDRESS_` constants, by where ADD0 is tied.
/// * `averaging` - the averaging code, `0..=3`, for 1, 8, 32, or 64 conversions per result;
///   `1`, eight, is the factory setting.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_tmp117_new(
    bus: *const PamojaI2cBus,
    address: u8,
    averaging: u8,
    out_sensor: *mut *mut PamojaTmp117,
) -> PamojaStatus {
    build(bus, out_sensor, |bus, delay| PamojaTmp117 {
        sensor: tmp117::Tmp117::new(bus, address, delay)
            .with_averaging(tmp117::Averaging::from_code(averaging)),
    })
}

/// Checks the part is a TMP117, waits for its EEPROM to finish loading, and writes the
/// settings with the part in shutdown.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails or the EEPROM never reports ready; or
/// [`PamojaStatus::Codec`] when the device id is not a TMP117's.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_tmp117_init(sensor: *mut PamojaTmp117) -> PamojaStatus {
    run(sensor, |held| held.sensor.init())
}

/// Runs one conversion and reads the temperature, initializing the part first if
/// [`pamoja_tmp117_init`] has not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_reading` - receives the temperature.
///
/// # Returns
///
/// As [`pamoja_tmp117_init`], with [`PamojaStatus::Io`] also when the part never reports the
/// conversion done.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_reading` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_tmp117_measure(
    sensor: *mut PamojaTmp117,
    out_reading: *mut PamojaTmp117Reading,
) -> PamojaStatus {
    fill(
        sensor,
        out_reading,
        "out_reading",
        |held| held.sensor.measure(),
        |reading| PamojaTmp117Reading {
            raw: reading.raw(),
            micro_celsius: reading.micro_celsius(),
            celsius: reading.celsius(),
        },
    )
}

/// Writes the high and low limits the part compares each result against.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `high_celsius` - the high limit; the factory value is 192 C.
/// * `low_celsius` - the low limit; the factory value is -256 C.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null driver, or
/// [`PamojaStatus::Io`] when the bus fails.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_tmp117_set_alert_limits(
    sensor: *mut PamojaTmp117,
    high_celsius: f32,
    low_celsius: f32,
) -> PamojaStatus {
    run(sensor, |held| {
        held.sensor.set_alert_limits(high_celsius, low_celsius)
    })
}

/// Reads the alert flags: whether a result since the last call was above the high limit or
/// below the low limit, including results the driver's own reads saw.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_alerts` - receives the flags.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null argument, or
/// [`PamojaStatus::Io`] when the bus fails.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_alerts` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_tmp117_alerts(
    sensor: *mut PamojaTmp117,
    out_alerts: *mut PamojaTmp117Alerts,
) -> PamojaStatus {
    fill(
        sensor,
        out_alerts,
        "out_alerts",
        |held| held.sensor.alerts(),
        |alerts| PamojaTmp117Alerts {
            high: u8::from(alerts.high),
            low: u8::from(alerts.low),
        },
    )
}

/// Reports the silicon revision a TMP117 driver read at initialization.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_revision` - receives the revision.
///
/// # Returns
///
/// `true` with the revision in `out_revision`; `false` before the part has been initialized,
/// or for a null argument.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_revision` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_tmp117_silicon_revision(
    sensor: *const PamojaTmp117,
    out_revision: *mut u8,
) -> bool {
    let Some(held) = sensor.as_ref() else {
        return false;
    };
    store(out_revision, held.sensor.revision())
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_tmp117_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_tmp117_free(sensor: *mut PamojaTmp117) {
    release(sensor);
}

/// Creates a simulated TMP117 reading [`PAMOJA_TMP117_SIM_CELSIUS`].
///
/// # Arguments
///
/// * `address` - the address it answers to.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_tmp117_sim_part(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(tmp117::sim::part(address))
}

/// Creates a simulated TMP117 that reads what it is asked to. Its configuration register keeps
/// the flags the part sets for itself, with the data-ready flag set.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `celsius` - the temperature it reports, which lands on the nearest 7.8125 millidegrees.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_tmp117_sim_reporting(address: u8, celsius: f32) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(tmp117::sim::reporting(address, celsius))
}

/// Returns the settings an OPT3001 driver starts with, the part's own reset settings: the
/// 800 ms integration time and the range chosen automatically.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_opt3001_settings_default() -> PamojaOpt3001Settings {
    PamojaOpt3001Settings {
        long_conversion: 1,
        range_number: opt3001::RANGE_AUTOMATIC,
    }
}

/// Creates an OPT3001 driver on a bus. Nothing is sent until [`pamoja_opt3001_init`] or the
/// first [`pamoja_opt3001_measure`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `address` - one of the `PAMOJA_OPT3001_I2C_ADDRESS_` constants, by where ADDR is tied.
/// * `settings` - the integration time and the range.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_new(
    bus: *const PamojaI2cBus,
    address: u8,
    settings: PamojaOpt3001Settings,
    out_sensor: *mut *mut PamojaOpt3001,
) -> PamojaStatus {
    build(bus, out_sensor, |bus, delay| PamojaOpt3001 {
        sensor: opt3001::Opt3001::new(bus, address, delay)
            .with_conversion_time(conversion_time(settings.long_conversion != 0))
            .with_range(settings.range_number),
    })
}

/// Checks the part is an OPT3001 and writes the settings with the part in shutdown.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails; or [`PamojaStatus::Codec`] when either id register
/// is not an OPT3001's.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_init(sensor: *mut PamojaOpt3001) -> PamojaStatus {
    run(sensor, |held| held.sensor.init())
}

/// Runs one conversion and reads the illuminance, initializing the part first if
/// [`pamoja_opt3001_init`] has not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_reading` - receives the illuminance.
///
/// # Returns
///
/// As [`pamoja_opt3001_init`], with [`PamojaStatus::Io`] also when the conversion never
/// finishes.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_reading` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_measure(
    sensor: *mut PamojaOpt3001,
    out_reading: *mut PamojaOpt3001Reading,
) -> PamojaStatus {
    fill(
        sensor,
        out_reading,
        "out_reading",
        |held| held.sensor.measure(),
        |reading| PamojaOpt3001Reading {
            raw: reading.raw(),
            milli_lux: reading.milli_lux(),
            lux: reading.lux(),
        },
    )
}

/// Writes the low and high limits the part's interrupt pin compares each result against.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `low_milli_lux` - the low limit, in millilux.
/// * `high_milli_lux` - the high limit, in millilux.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null driver, or
/// [`PamojaStatus::Io`] when the bus fails.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_set_limits(
    sensor: *mut PamojaOpt3001,
    low_milli_lux: u32,
    high_milli_lux: u32,
) -> PamojaStatus {
    run(sensor, |held| {
        held.sensor.set_limits(low_milli_lux, high_milli_lux)
    })
}

/// Copies the configuration an OPT3001 driver writes, with the part in shutdown.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_config` - receives the configuration.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_config` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_configuration(
    sensor: *const PamojaOpt3001,
    out_config: *mut PamojaOpt3001Config,
) -> PamojaStatus {
    look(sensor, out_config, "out_config", |held| {
        held.sensor.configuration().into()
    })
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_opt3001_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_opt3001_free(sensor: *mut PamojaOpt3001) {
    release(sensor);
}

/// Creates a simulated OPT3001 reading [`PAMOJA_OPT3001_SIM_LUX`].
///
/// # Arguments
///
/// * `address` - the address it answers to.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_opt3001_sim_part(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(opt3001::sim::part(address))
}

/// Creates a simulated OPT3001 that reads what it is asked to. Its configuration register
/// keeps the flags the part sets for itself, with the conversion-ready flag set.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `lux` - the illuminance it reports, which lands on the nearest step its exponent and
///   mantissa represent.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_opt3001_sim_reporting(address: u8, lux: f32) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(opt3001::sim::reporting(address, lux))
}

/// Returns the settings an HDC1080 driver starts with: both channels at 14 bits.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_hdc1080_settings_default() -> PamojaHdc1080Settings {
    PamojaHdc1080Settings {
        temperature_resolution_bits: 14,
        humidity_resolution_bits: 14,
    }
}

/// Creates an HDC1080 driver on a bus; the part has one address. Nothing is sent until
/// [`pamoja_hdc1080_init`] or the first [`pamoja_hdc1080_measure`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `settings` - the resolution of each channel.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument or a resolution the part does not have.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_new(
    bus: *const PamojaI2cBus,
    settings: PamojaHdc1080Settings,
    out_sensor: *mut *mut PamojaHdc1080,
) -> PamojaStatus {
    let Some(temperature) = temperature_resolution(settings.temperature_resolution_bits) else {
        return clear_then(out_sensor, crate::sensors::bad_temperature_resolution());
    };
    let Some(humidity) = humidity_resolution(settings.humidity_resolution_bits) else {
        return clear_then(out_sensor, crate::sensors::bad_humidity_resolution());
    };
    build(bus, out_sensor, |bus, delay| PamojaHdc1080 {
        sensor: hdc1080::Hdc1080::new(bus, delay).with_resolutions(temperature, humidity),
    })
}

/// Checks the part is an HDC1080 and writes the configuration.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails; or [`PamojaStatus::Codec`] when either id register
/// is not an HDC1080's.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_init(sensor: *mut PamojaHdc1080) -> PamojaStatus {
    run(sensor, |held| held.sensor.init())
}

/// Triggers one acquisition of both channels and reads them, initializing the part first if
/// [`pamoja_hdc1080_init`] has not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_measurement` - receives the temperature and humidity.
///
/// # Returns
///
/// As [`pamoja_hdc1080_init`]. A part that does not acknowledge the read before its results
/// are ready is [`PamojaStatus::Io`].
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_measurement` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_measure(
    sensor: *mut PamojaHdc1080,
    out_measurement: *mut PamojaHdc1080Measurement,
) -> PamojaStatus {
    fill(
        sensor,
        out_measurement,
        "out_measurement",
        |held| held.sensor.measure(),
        PamojaHdc1080Measurement::from,
    )
}

/// Switches the on-die heater, which runs only during acquisitions, on or off.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `on` - whether the heater runs.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null driver, or
/// [`PamojaStatus::Io`] when the bus fails.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_heater(
    sensor: *mut PamojaHdc1080,
    on: bool,
) -> PamojaStatus {
    run(sensor, |held| held.sensor.heater(on))
}

/// Copies the configuration an HDC1080 driver writes.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_config` - receives the configuration.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_config` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_configuration(
    sensor: *const PamojaHdc1080,
    out_config: *mut PamojaHdc1080Config,
) -> PamojaStatus {
    look(sensor, out_config, "out_config", |held| {
        held.sensor.configuration().into()
    })
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_hdc1080_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_hdc1080_free(sensor: *mut PamojaHdc1080) {
    release(sensor);
}

/// Creates a simulated HDC1080 reading [`PAMOJA_HDC1080_SIM_CELSIUS`] and
/// [`PAMOJA_HDC1080_SIM_RELATIVE_HUMIDITY`].
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_hdc1080_sim_part() -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(hdc1080::sim::part())
}

/// Creates a simulated HDC1080 that reads what it is asked to.
///
/// # Arguments
///
/// * `celsius` - the temperature it reports.
/// * `relative_humidity` - the humidity it reports, as a percentage.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`]. Its readings
/// land within three thousandths of a degree and two thousandths of a percent.
#[no_mangle]
pub extern "C" fn pamoja_hdc1080_sim_reporting(
    celsius: f32,
    relative_humidity: f32,
) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(hdc1080::sim::reporting(celsius, relative_humidity))
}

/// Returns the settings an INA219 driver starts with: a 100 milliohm shunt sized for 3.2 A,
/// the common breakout, the finest current step for it, and the power-on register settings.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_ina219_settings_default() -> PamojaIna219Settings {
    PamojaIna219Settings {
        shunt_milliohms: 100,
        max_microamps: 3_200_000,
        current_lsb_microamps: 0,
        config: ina219::Configuration::default().into(),
    }
}

/// Creates an INA219 driver on a bus. Nothing is sent until [`pamoja_ina219_init`] or the first
/// [`pamoja_ina219_measure`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `address` - [`crate::sensors::PAMOJA_INA219_BASE_ADDRESS`] plus what A1 and A0 add.
/// * `settings` - the shunt, the current it is sized for, and the register settings.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina219_new(
    bus: *const PamojaI2cBus,
    address: u8,
    settings: PamojaIna219Settings,
    out_sensor: *mut *mut PamojaIna219,
) -> PamojaStatus {
    build(bus, out_sensor, |bus, delay| {
        let mut sensor = ina219::Ina219::new(bus, address, delay)
            .with_shunt(settings.shunt_milliohms, settings.max_microamps)
            .with_configuration(settings.config.into());
        if settings.current_lsb_microamps != 0 {
            sensor = sensor.with_current_lsb(settings.current_lsb_microamps);
        }
        PamojaIna219 { sensor }
    })
}

/// Resets the part, writes the configuration and the calibration, and reads the calibration
/// back, which is the identity check a part with no id register allows.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails; or [`PamojaStatus::Codec`] when the calibration
/// register does not hold what was written.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina219_init(sensor: *mut PamojaIna219) -> PamojaStatus {
    run(sensor, |held| held.sensor.init())
}

/// Triggers one shunt and bus conversion and reads every result, initializing the part first
/// if [`pamoja_ina219_init`] has not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_reading` - receives the results.
///
/// # Returns
///
/// As [`pamoja_ina219_init`], with [`PamojaStatus::Io`] also when the conversion-ready flag
/// never sets.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_reading` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina219_measure(
    sensor: *mut PamojaIna219,
    out_reading: *mut PamojaIna219Reading,
) -> PamojaStatus {
    fill(
        sensor,
        out_reading,
        "out_reading",
        |held| held.sensor.measure(),
        |reading| PamojaIna219Reading {
            shunt: reading.shunt,
            bus: reading.bus,
            current: reading.current,
            power: reading.power,
            current_lsb_microamps: reading.current_lsb_microamps,
            shunt_microvolts: reading.shunt_microvolts(),
            bus_millivolts: reading.bus_millivolts(),
            current_microamps: reading.current_microamps(),
            power_microwatts: reading.power_microwatts(),
            math_overflow: u8::from(reading.math_overflow()),
        },
    )
}

/// Returns the current step an INA219 driver programs.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// Microamps per count of the current register, or 0 for a null driver.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina219_current_lsb(sensor: *const PamojaIna219) -> u32 {
    sensor
        .as_ref()
        .map_or(0, |held| held.sensor.current_lsb_microamps())
}

/// Returns the calibration word an INA219 driver programs.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// The word, from the shunt and the current step by the datasheet's equation, or 0 for a
/// null driver.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina219_calibration_word(sensor: *const PamojaIna219) -> u16 {
    sensor.as_ref().map_or(0, |held| held.sensor.calibration())
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_ina219_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina219_free(sensor: *mut PamojaIna219) {
    release(sensor);
}

/// Creates a simulated INA219 carrying [`PAMOJA_INA219_SIM_MICROAMPS`] at
/// [`PAMOJA_INA219_SIM_BUS_MILLIVOLTS`] through the shunt a driver starts with.
///
/// # Arguments
///
/// * `address` - the address it answers to.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_ina219_sim_part(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(ina219::sim::part(address))
}

/// Creates a simulated INA219 that reads what it is asked to, calibrated for the same shunt and
/// largest current a driver is given.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `shunt_milliohms` - the shunt, as in the driver's settings.
/// * `max_microamps` - the largest current, as in the driver's settings.
/// * `bus_millivolts` - the bus voltage it reports.
/// * `microamps` - the current it reports; negative flows the other way through the shunt.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`]. Its readings
/// land on the steps its registers count in: 4 mV of bus, 10 uV of shunt, and the
/// calibration's current step.
#[no_mangle]
pub extern "C" fn pamoja_ina219_sim_reporting(
    address: u8,
    shunt_milliohms: u32,
    max_microamps: u32,
    bus_millivolts: u32,
    microamps: i32,
) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(ina219::sim::reporting(
        address,
        shunt_milliohms,
        max_microamps,
        bus_millivolts,
        microamps,
    ))
}

/// Returns the settings an INA226 driver starts with: a 100 milliohm shunt sized for 3.2 A,
/// the finest current step for it, and the power-on register settings.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_ina226_settings_default() -> PamojaIna226Settings {
    PamojaIna226Settings {
        shunt_milliohms: 100,
        max_microamps: 3_200_000,
        current_lsb_microamps: 0,
        config: ina226::Configuration::RESET.into(),
    }
}

/// Creates an INA226 driver on a bus. Nothing is sent until [`pamoja_ina226_init`] or the first
/// [`pamoja_ina226_measure`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `address` - the address A1 and A0 select, which
///   [`crate::sensors::pamoja_ina226_address`] works out.
/// * `settings` - the shunt, the current it is sized for, and the register settings.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_new(
    bus: *const PamojaI2cBus,
    address: u8,
    settings: PamojaIna226Settings,
    out_sensor: *mut *mut PamojaIna226,
) -> PamojaStatus {
    build(bus, out_sensor, |bus, delay| {
        let mut sensor = ina226::Ina226::new(bus, address, delay)
            .with_shunt(settings.shunt_milliohms, settings.max_microamps)
            .with_configuration(settings.config.into());
        if settings.current_lsb_microamps != 0 {
            sensor = sensor.with_current_lsb(settings.current_lsb_microamps);
        }
        PamojaIna226 { sensor }
    })
}

/// Resets the part, checks it is an INA226, writes the configuration and the calibration, and
/// reads the calibration back.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails; or [`PamojaStatus::Codec`] when the id registers
/// are not an INA226's or the calibration does not read back.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_init(sensor: *mut PamojaIna226) -> PamojaStatus {
    run(sensor, |held| held.sensor.init())
}

/// Triggers one shunt and bus conversion and reads every result, initializing the part first
/// if [`pamoja_ina226_init`] has not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_reading` - receives the results.
///
/// # Returns
///
/// As [`pamoja_ina226_init`], with [`PamojaStatus::Io`] also when the conversion-ready flag
/// never sets.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_reading` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_measure(
    sensor: *mut PamojaIna226,
    out_reading: *mut PamojaIna226Reading,
) -> PamojaStatus {
    fill(
        sensor,
        out_reading,
        "out_reading",
        |held| held.sensor.measure(),
        |reading| PamojaIna226Reading {
            shunt: reading.shunt,
            bus: reading.bus,
            current: reading.current,
            power: reading.power,
            current_lsb_microamps: reading.current_lsb_microamps,
            shunt_nanovolts: reading.shunt_nanovolts(),
            shunt_millivolts: reading.shunt_millivolts(),
            bus_microvolts: reading.bus_microvolts(),
            bus_volts: reading.bus_volts(),
            current_microamps: reading.current_microamps(),
            current_amps: reading.current_amps(),
            power_microwatts: reading.power_microwatts(),
            power_watts: reading.power_watts(),
            math_overflow: u8::from(reading.math_overflow),
        },
    )
}

/// Programs the alert pin: which limit it watches, and the limit.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `mask` - the mask/enable settings, one alert function at a time.
/// * `limit` - the alert-limit register, in the units of the register the function watches.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null driver, or
/// [`PamojaStatus::Io`] when the bus fails.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_set_alert(
    sensor: *mut PamojaIna226,
    mask: PamojaIna226MaskEnable,
    limit: u16,
) -> PamojaStatus {
    run(sensor, |held| held.sensor.set_alert(mask.into(), limit))
}

/// Returns the current step an INA226 driver programs.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// Microamps per count of the current register, or 0 for a null driver.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_current_lsb(sensor: *const PamojaIna226) -> u32 {
    sensor
        .as_ref()
        .map_or(0, |held| held.sensor.current_lsb_microamps())
}

/// Returns the calibration word an INA226 driver programs.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// The word, or 0 for a null driver.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_calibration_word(sensor: *const PamojaIna226) -> u16 {
    sensor.as_ref().map_or(0, |held| held.sensor.calibration())
}

/// Reports the die id an INA226 driver read at initialization.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_die_id` - receives the device and revision.
///
/// # Returns
///
/// `true` with the id in `out_die_id`; `false` before the part has been initialized, or for a
/// null argument.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_die_id` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_identity(
    sensor: *const PamojaIna226,
    out_die_id: *mut PamojaIna226DieId,
) -> bool {
    let Some(held) = sensor.as_ref() else {
        return false;
    };
    store(out_die_id, held.sensor.die_id().map(Into::into))
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_ina226_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ina226_free(sensor: *mut PamojaIna226) {
    release(sensor);
}

/// Creates a simulated INA226 carrying [`PAMOJA_INA226_SIM_MICROAMPS`] at
/// [`PAMOJA_INA226_SIM_BUS_MICROVOLTS`] through the shunt a driver starts with.
///
/// # Arguments
///
/// * `address` - the address it answers to.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_ina226_sim_part(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(ina226::sim::part(address))
}

/// Creates a simulated INA226 that reads what it is asked to, calibrated for the same shunt and
/// largest current a driver is given.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `shunt_milliohms` - the shunt, as in the driver's settings.
/// * `max_microamps` - the largest current, as in the driver's settings.
/// * `bus_microvolts` - the bus voltage it reports.
/// * `microamps` - the current it reports; negative flows the other way through the shunt.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`]. Its readings
/// land on the steps its registers count in: 1.25 mV of bus, 2.5 uV of shunt, and the
/// calibration's current step.
#[no_mangle]
pub extern "C" fn pamoja_ina226_sim_reporting(
    address: u8,
    shunt_milliohms: u32,
    max_microamps: u32,
    bus_microvolts: u32,
    microamps: i32,
) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(ina226::sim::reporting(
        address,
        shunt_milliohms,
        max_microamps,
        bus_microvolts,
        microamps,
    ))
}

/// Returns the settings an ADS1115 driver starts with, the part's own reset settings: AIN0
/// against AIN1, the 2.048 V range, and 128 samples per second.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_ads1115_settings_default() -> PamojaAds1115Settings {
    let config = ads1115::Config::default();
    PamojaAds1115Settings {
        mux: config.mux.code(),
        pga: config.pga.code(),
        data_rate: config.data_rate.code(),
    }
}

/// Creates an ADS1115 driver on a bus. Nothing is sent until [`pamoja_ads1115_init`] or the
/// first [`pamoja_ads1115_sample`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `address` - one of the `PAMOJA_ADS1115_ADDRESS_` constants, by where ADDR is tied.
/// * `settings` - the input, the range, and the data rate.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ads1115_new(
    bus: *const PamojaI2cBus,
    address: u8,
    settings: PamojaAds1115Settings,
    out_sensor: *mut *mut PamojaAds1115,
) -> PamojaStatus {
    build(bus, out_sensor, |bus, delay| PamojaAds1115 {
        sensor: ads1115::Ads1115::new(bus, address, delay)
            .with_input(ads1115::Mux::from_code(settings.mux))
            .with_gain(ads1115::Pga::from_code(settings.pga))
            .with_data_rate(ads1115::DataRate::from_code(settings.data_rate)),
    })
}

/// Writes the input, range, and data rate, and reads the configuration back, which is the
/// identity check a part with no id register allows.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails; or [`PamojaStatus::Codec`] when the configuration
/// reads back differently.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ads1115_init(sensor: *mut PamojaAds1115) -> PamojaStatus {
    run(sensor, |held| held.sensor.init())
}

/// Runs one conversion of the configured input, initializing the part first if
/// [`pamoja_ads1115_init`] has not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_sample` - receives the conversion, with the range it ran at.
///
/// # Returns
///
/// As [`pamoja_ads1115_init`], with [`PamojaStatus::Io`] also when the part never reports the
/// conversion done.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_sample` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ads1115_sample(
    sensor: *mut PamojaAds1115,
    out_sample: *mut PamojaAds1115Sample,
) -> PamojaStatus {
    fill(
        sensor,
        out_sample,
        "out_sample",
        |held| held.sensor.sample(),
        sample_of,
    )
}

/// Converts another input once, leaving the configured input as it was.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `mux` - the input multiplexer code for this one conversion.
/// * `out_sample` - receives the conversion.
///
/// # Returns
///
/// As [`pamoja_ads1115_sample`].
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_sample` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ads1115_sample_input(
    sensor: *mut PamojaAds1115,
    mux: u8,
    out_sample: *mut PamojaAds1115Sample,
) -> PamojaStatus {
    fill(
        sensor,
        out_sample,
        "out_sample",
        |held| held.sensor.sample_input(ads1115::Mux::from_code(mux)),
        sample_of,
    )
}

/// Copies the configuration an ADS1115 driver writes.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_config` - receives the configuration.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_config` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ads1115_config(
    sensor: *const PamojaAds1115,
    out_config: *mut PamojaAds1115Config,
) -> PamojaStatus {
    look(sensor, out_config, "out_config", |held| {
        held.sensor.config().into()
    })
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_ads1115_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ads1115_free(sensor: *mut PamojaAds1115) {
    release(sensor);
}

/// Creates a simulated ADS1115 reading [`PAMOJA_ADS1115_SIM_VOLTS`] at the range a driver
/// starts with.
///
/// # Arguments
///
/// * `address` - the address it answers to.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_ads1115_sim_part(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(ads1115::sim::part(address))
}

/// Creates a simulated ADS1115 that reads what it is asked to at the range a driver converts
/// at.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `pga` - the gain code the driver converts at.
/// * `volts` - the voltage it reports, held to the range.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`]. Its reading
/// lands on the nearest of the 32768 steps either side of zero the range divides into.
#[no_mangle]
pub extern "C" fn pamoja_ads1115_sim_reporting(
    address: u8,
    pga: u8,
    volts: f32,
) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(ads1115::sim::reporting(
        address,
        ads1115::Pga::from_code(pga),
        volts,
    ))
}

/// Creates an SHT3x driver on a bus. Nothing is sent until [`pamoja_sht3x_init`] or the first
/// [`pamoja_sht3x_measure`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `address` - `PAMOJA_SHT3X_I2C_ADDRESS_A` with ADDR low, `PAMOJA_SHT3X_I2C_ADDRESS_B` with
///   ADDR high.
/// * `repeatability` - `0` low, `1` medium, or `2` high, which trades noise against
///   measurement time.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument or a repeatability code the part does not have.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_new(
    bus: *const PamojaI2cBus,
    address: u8,
    repeatability: u8,
    out_sensor: *mut *mut PamojaSht3x,
) -> PamojaStatus {
    let Some(repeatability) = repeatability_from_code(repeatability) else {
        return clear_then(out_sensor, crate::sensors::bad_repeatability());
    };
    build(bus, out_sensor, |bus, delay| PamojaSht3x {
        sensor: sht3x::Sht3x::new(bus, address, delay).with_repeatability(repeatability),
    })
}

/// Soft-resets the part and reads its status; a status word whose CRC checks is what confirms
/// an SHT3x answers, as the part has no id register.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails; or [`PamojaStatus::Codec`] when the status word
/// fails its CRC.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_init(sensor: *mut PamojaSht3x) -> PamojaStatus {
    run(sensor, |held| held.sensor.init())
}

/// Runs one single-shot measurement, initializing the part first if [`pamoja_sht3x_init`] has
/// not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_measurement` - receives the CRC-checked temperature and humidity.
///
/// # Returns
///
/// As [`pamoja_sht3x_init`], with [`PamojaStatus::Codec`] also when a data word fails its CRC.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_measurement` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_measure(
    sensor: *mut PamojaSht3x,
    out_measurement: *mut PamojaSht3xMeasurement,
) -> PamojaStatus {
    fill(
        sensor,
        out_measurement,
        "out_measurement",
        |held| held.sensor.measure(),
        PamojaSht3xMeasurement::from,
    )
}

/// Reads the status register.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_status` - receives the status.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null argument,
/// [`PamojaStatus::Io`] when the bus fails, or [`PamojaStatus::Codec`] when the word fails its
/// CRC.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_status` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_read_status(
    sensor: *mut PamojaSht3x,
    out_status: *mut PamojaSht3xStatus,
) -> PamojaStatus {
    fill(
        sensor,
        out_status,
        "out_status",
        |held| held.sensor.read_status(),
        PamojaSht3xStatus::from,
    )
}

/// Reports the status an SHT3x driver last read, without reading it again.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_status` - receives the status.
///
/// # Returns
///
/// `true` with the status in `out_status`; `false` before the status has been read, or for a
/// null argument.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_status` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_last_status(
    sensor: *const PamojaSht3x,
    out_status: *mut PamojaSht3xStatus,
) -> bool {
    let Some(held) = sensor.as_ref() else {
        return false;
    };
    store(out_status, held.sensor.status().map(Into::into))
}

/// Switches the plausibility-check heater on, initializing the part first if needed, since
/// the reset in initialization switches it off.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// As [`pamoja_sht3x_init`].
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_heater_on(sensor: *mut PamojaSht3x) -> PamojaStatus {
    run(sensor, |held| held.sensor.heater_on())
}

/// Switches the heater off, which is its state after any reset.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// As [`pamoja_sht3x_init`].
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_heater_off(sensor: *mut PamojaSht3x) -> PamojaStatus {
    run(sensor, |held| held.sensor.heater_off())
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_sht3x_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sht3x_free(sensor: *mut PamojaSht3x) {
    release(sensor);
}

/// Creates a simulated SHT3x reading [`PAMOJA_SHT3X_SIM_CELSIUS`] and
/// [`PAMOJA_SHT3X_SIM_RELATIVE_HUMIDITY`].
///
/// # Arguments
///
/// * `address` - the address it answers to.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_sht3x_sim_part(address: u8) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(sht3x::sim::part(address))
}

/// Creates a simulated SHT3x that reads what it is asked to. It answers every single-shot
/// command, a periodic fetch, and the status command with the replies a real part gives.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `celsius` - the temperature it reports.
/// * `relative_humidity` - the humidity it reports, as a percentage.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`]. Its readings
/// land within three thousandths of a degree and two thousandths of a percent.
#[no_mangle]
pub extern "C" fn pamoja_sht3x_sim_reporting(
    address: u8,
    celsius: f32,
    relative_humidity: f32,
) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(sht3x::sim::reporting(address, celsius, relative_humidity))
}

/// Creates an SCD40 or SCD41 driver on a bus; the part has one address. Nothing is sent until
/// [`pamoja_scd4x_init`] or the first [`pamoja_scd4x_measure`].
///
/// # Arguments
///
/// * `bus` - the bus the part is on; the driver holds its own share.
/// * `out_sensor` - receives the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the driver in `out_sensor`, or [`PamojaStatus::InvalidArgument`]
/// for a null argument.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_sensor` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_new(
    bus: *const PamojaI2cBus,
    out_sensor: *mut *mut PamojaScd4x,
) -> PamojaStatus {
    build(bus, out_sensor, |bus, delay| PamojaScd4x {
        sensor: scd4x::Scd4x::new(bus, delay),
    })
}

/// Stops any running measurement, reads the serial number, and starts periodic measurement,
/// after which the part has a new result every five seconds.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null driver;
/// [`PamojaStatus::Io`] when the bus fails; or [`PamojaStatus::Codec`] when the serial number
/// fails its CRC.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_init(sensor: *mut PamojaScd4x) -> PamojaStatus {
    run(sensor, |held| held.sensor.init())
}

/// Waits for the next periodic result and reads it, initializing the part first if
/// [`pamoja_scd4x_init`] has not run.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_measurement` - receives the carbon dioxide, temperature, and humidity.
///
/// # Returns
///
/// As [`pamoja_scd4x_init`], with [`PamojaStatus::Io`] also when no result becomes ready and
/// [`PamojaStatus::Codec`] when a word fails its CRC.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_measurement` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_measure(
    sensor: *mut PamojaScd4x,
    out_measurement: *mut PamojaScd4xMeasurement,
) -> PamojaStatus {
    fill(
        sensor,
        out_measurement,
        "out_measurement",
        |held| held.sensor.measure(),
        PamojaScd4xMeasurement::from,
    )
}

/// Runs one on-demand measurement on an SCD41, which takes five seconds. The part must not be
/// measuring periodically: call [`pamoja_scd4x_stop`] first, or use this in place of
/// [`pamoja_scd4x_init`].
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_measurement` - receives the measurement.
///
/// # Returns
///
/// As [`pamoja_scd4x_measure`].
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_measurement` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_measure_single_shot(
    sensor: *mut PamojaScd4x,
    out_measurement: *mut PamojaScd4xMeasurement,
) -> PamojaStatus {
    fill(
        sensor,
        out_measurement,
        "out_measurement",
        |held| held.sensor.measure_single_shot(),
        PamojaScd4xMeasurement::from,
    )
}

/// Asks the part whether a periodic result is waiting.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_ready` - receives `true` when [`pamoja_scd4x_measure`] would read without waiting.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null argument,
/// [`PamojaStatus::Io`] when the bus fails, or [`PamojaStatus::Codec`] when the status word
/// fails its CRC.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_ready` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_poll_ready(
    sensor: *mut PamojaScd4x,
    out_ready: *mut bool,
) -> PamojaStatus {
    fill(
        sensor,
        out_ready,
        "out_ready",
        |held| held.sensor.data_ready(),
        |ready| ready,
    )
}

/// Stops periodic measurement, after which the part takes its settings commands.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null driver, or
/// [`PamojaStatus::Io`] when the bus fails.
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_stop(sensor: *mut PamojaScd4x) -> PamojaStatus {
    run(sensor, |held| held.sensor.stop())
}

/// Starts periodic measurement.
///
/// # Arguments
///
/// * `sensor` - the driver.
///
/// # Returns
///
/// As [`pamoja_scd4x_stop`].
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_start(sensor: *mut PamojaScd4x) -> PamojaStatus {
    run(sensor, |held| held.sensor.start())
}

/// Sets the temperature offset that compensates the part's own warmth, stopping periodic
/// measurement to write it and starting it again. The setting lasts until power is lost.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `milli_celsius` - the offset to subtract, in millidegrees.
///
/// # Returns
///
/// As [`pamoja_scd4x_stop`].
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_set_temperature_offset(
    sensor: *mut PamojaScd4x,
    milli_celsius: u32,
) -> PamojaStatus {
    run(sensor, |held| {
        held.sensor.set_temperature_offset(milli_celsius)
    })
}

/// Sets the altitude the part corrects its carbon dioxide reading for, stopping periodic
/// measurement to write it and starting it again.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `meters` - the altitude above sea level.
///
/// # Returns
///
/// As [`pamoja_scd4x_stop`].
///
/// # Safety
///
/// `sensor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_set_sensor_altitude(
    sensor: *mut PamojaScd4x,
    meters: u16,
) -> PamojaStatus {
    run(sensor, |held| held.sensor.set_sensor_altitude(meters))
}

/// Reports the serial number an SCD4x driver read at initialization.
///
/// # Arguments
///
/// * `sensor` - the driver.
/// * `out_serial` - receives the 48-bit serial number.
///
/// # Returns
///
/// `true` with the serial in `out_serial`; `false` before the part has been initialized, or
/// for a null argument.
///
/// # Safety
///
/// `sensor` must be a live handle or null, and `out_serial` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_serial(
    sensor: *const PamojaScd4x,
    out_serial: *mut u64,
) -> bool {
    let Some(held) = sensor.as_ref() else {
        return false;
    };
    store(out_serial, held.sensor.serial())
}

/// Releases a driver and its share of the bus. A null pointer is ignored.
///
/// # Safety
///
/// `sensor` must be a handle from [`pamoja_scd4x_new`] that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_scd4x_free(sensor: *mut PamojaScd4x) {
    release(sensor);
}

/// Creates a simulated SCD4x reading [`PAMOJA_SCD4X_SIM_CO2_PPM`],
/// [`PAMOJA_SCD4X_SIM_CELSIUS`], and [`PAMOJA_SCD4X_SIM_RELATIVE_HUMIDITY`].
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_scd4x_sim_part() -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(scd4x::sim::part())
}

/// Creates a simulated SCD4x that reads what it is asked to. It answers the serial number,
/// data-ready, and measurement commands, and always has a result waiting.
///
/// # Arguments
///
/// * `co2_ppm` - the carbon dioxide it reports, in parts per million.
/// * `celsius` - the temperature it reports.
/// * `relative_humidity` - the humidity it reports, as a percentage.
///
/// # Returns
///
/// The part, which the caller releases with [`crate::hal::pamoja_i2c_part_free`].
#[no_mangle]
pub extern "C" fn pamoja_scd4x_sim_reporting(
    co2_ppm: u16,
    celsius: f32,
    relative_humidity: f32,
) -> *mut PamojaI2cPart {
    PamojaI2cPart::into_raw(scd4x::sim::reporting(co2_ppm, celsius, relative_humidity))
}

/// Names a DS18B20 the kernel serves by the serial in its directory name, reading
/// `/sys/bus/w1/devices/28-<serial>/w1_slave`.
///
/// # Arguments
///
/// * `serial` - the twelve hex digits after `28-`.
/// * `out_thermometer` - receives the thermometer.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null or non-UTF-8
/// argument. Nothing is read until [`pamoja_ds18b20_thermometer_read`].
///
/// # Safety
///
/// `serial` must be a null-terminated string or null, and `out_thermometer` a writable pointer
/// or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_thermometer_new(
    serial: *const c_char,
    out_thermometer: *mut *mut PamojaDs18b20Thermometer,
) -> PamojaStatus {
    thermometer_from(
        serial,
        "serial",
        out_thermometer,
        ds18b20::linux::Thermometer::new,
    )
}

/// Names a DS18B20 by the path of its `w1_slave` file, for a system that mounts sysfs elsewhere
/// or a test that writes the file itself.
///
/// # Arguments
///
/// * `path` - the file to read.
/// * `out_thermometer` - receives the thermometer.
///
/// # Returns
///
/// As [`pamoja_ds18b20_thermometer_new`].
///
/// # Safety
///
/// `path` must be a null-terminated string or null, and `out_thermometer` a writable pointer or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_thermometer_at(
    path: *const c_char,
    out_thermometer: *mut *mut PamojaDs18b20Thermometer,
) -> PamojaStatus {
    thermometer_from(path, "path", out_thermometer, |path| {
        ds18b20::linux::Thermometer::at(path)
    })
}

/// Returns the path of the file a thermometer reads.
///
/// # Arguments
///
/// * `thermometer` - the thermometer.
///
/// # Returns
///
/// The path, which the caller releases with [`crate::pamoja_string_free`], or null for a null
/// thermometer.
///
/// # Safety
///
/// `thermometer` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_thermometer_path(
    thermometer: *const PamojaDs18b20Thermometer,
) -> *mut PamojaString {
    match thermometer.as_ref() {
        Some(held) => {
            PamojaString::into_raw(held.thermometer.path().to_string_lossy().into_owned())
        }
        None => std::ptr::null_mut(),
    }
}

/// Reads a thermometer's file, which makes the kernel run a conversion, and decodes it.
///
/// # Arguments
///
/// * `thermometer` - the thermometer.
/// * `out_reading` - receives the CRC-checked reading.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null argument;
/// [`PamojaStatus::Io`] when the file cannot be read, because the 1-Wire overlay is off, the
/// probe is gone, or the process may not read it; or [`PamojaStatus::Codec`] when the kernel or
/// this decoder rejects the CRC.
///
/// # Safety
///
/// `thermometer` must be a live handle or null, and `out_reading` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_thermometer_read(
    thermometer: *const PamojaDs18b20Thermometer,
    out_reading: *mut PamojaDs18b20Reading,
) -> PamojaStatus {
    if out_reading.is_null() {
        set_last_error("out_reading must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(held) = thermometer.as_ref() else {
        set_last_error("thermometer must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match catch_unwind(AssertUnwindSafe(|| held.thermometer.read_scratchpad())) {
        Ok(Ok(scratchpad)) => {
            *out_reading = scratchpad.into();
            PamojaStatus::Ok
        }
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            match error {
                ds18b20::linux::ThermometerError::Io(_) => PamojaStatus::Io,
                ds18b20::linux::ThermometerError::Sensor(_) => PamojaStatus::Codec,
            }
        }
        Err(_) => panicked(),
    }
}

/// Releases a thermometer. A null pointer is ignored.
///
/// # Safety
///
/// `thermometer` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_thermometer_free(
    thermometer: *mut PamojaDs18b20Thermometer,
) {
    release(thermometer);
}

/// Lists every DS18B20 the kernel has found: one per `28-` directory.
///
/// # Arguments
///
/// * `devices` - the directory the kernel lists its 1-Wire devices in, or null for
///   `/sys/bus/w1/devices`.
/// * `out_thermometers` - receives the list, sorted by directory name.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null `out_thermometers` or a
/// non-UTF-8 directory; or [`PamojaStatus::Io`] when the directory cannot be listed, which
/// usually means the 1-Wire overlay is off.
///
/// # Safety
///
/// `devices` must be a null-terminated string or null, and `out_thermometers` a writable
/// pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_discover(
    devices: *const c_char,
    out_thermometers: *mut *mut PamojaDs18b20Thermometers,
) -> PamojaStatus {
    if out_thermometers.is_null() {
        set_last_error("out_thermometers must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_thermometers = std::ptr::null_mut();
    let devices = if devices.is_null() {
        ds18b20::linux::DEVICES
    } else {
        match read_str(devices, "devices") {
            Some(devices) => devices,
            None => return PamojaStatus::InvalidArgument,
        }
    };
    match ds18b20::linux::Thermometer::discover_in(Path::new(devices)) {
        Ok(found) => {
            *out_thermometers = Box::into_raw(Box::new(PamojaDs18b20Thermometers { found }));
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(format!("{devices}: {error}"));
            PamojaStatus::Io
        }
    }
}

/// Returns how many thermometers a list holds.
///
/// # Arguments
///
/// * `thermometers` - the list.
///
/// # Returns
///
/// The count, or 0 for a null list.
///
/// # Safety
///
/// `thermometers` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_thermometers_len(
    thermometers: *const PamojaDs18b20Thermometers,
) -> usize {
    thermometers.as_ref().map_or(0, |held| held.found.len())
}

/// Copies one thermometer out of a list.
///
/// # Arguments
///
/// * `thermometers` - the list.
/// * `index` - which one, below [`pamoja_ds18b20_thermometers_len`].
///
/// # Returns
///
/// A new thermometer, which the caller releases with [`pamoja_ds18b20_thermometer_free`], or
/// null for a null list or an index past the end.
///
/// # Safety
///
/// `thermometers` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_thermometers_get(
    thermometers: *const PamojaDs18b20Thermometers,
    index: usize,
) -> *mut PamojaDs18b20Thermometer {
    match thermometers.as_ref().and_then(|held| held.found.get(index)) {
        Some(thermometer) => Box::into_raw(Box::new(PamojaDs18b20Thermometer {
            thermometer: thermometer.clone(),
        })),
        None => std::ptr::null_mut(),
    }
}

/// Releases a list of thermometers. A null pointer is ignored.
///
/// # Safety
///
/// `thermometers` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ds18b20_thermometers_free(
    thermometers: *mut PamojaDs18b20Thermometers,
) {
    release(thermometers);
}

/// An ADS1115 sample as it crosses the boundary.
fn sample_of(sample: ads1115::Sample) -> PamojaAds1115Sample {
    PamojaAds1115Sample {
        raw: sample.raw,
        pga: sample.pga.code(),
        nanovolts: sample.nanovolts(),
        volts: sample.volts(),
    }
}

/// Builds a driver on the caller's bus and hands it back through `out`.
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out` a writable pointer or null.
unsafe fn build<H>(
    bus: *const PamojaI2cBus,
    out: *mut *mut H,
    make: impl FnOnce(I2cBus, BusDelay) -> H,
) -> PamojaStatus {
    if out.is_null() {
        set_last_error("out_sensor must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out = std::ptr::null_mut();
    let Some(bus) = bus.as_ref() else {
        set_last_error("bus must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let bus = bus.bus.clone();
    let delay = bus.delay();
    *out = Box::into_raw(Box::new(make(bus, delay)));
    PamojaStatus::Ok
}

/// Clears a caller's out pointer before returning a refusal that built nothing.
///
/// # Safety
///
/// `out` must be a writable pointer or null.
unsafe fn clear_then<H>(out: *mut *mut H, status: PamojaStatus) -> PamojaStatus {
    if !out.is_null() {
        *out = std::ptr::null_mut();
    }
    status
}

/// Names a DS18B20 from a string argument and hands it back through `out`.
///
/// # Safety
///
/// `text` must be a null-terminated string or null, and `out` a writable pointer or null.
unsafe fn thermometer_from(
    text: *const c_char,
    what: &str,
    out: *mut *mut PamojaDs18b20Thermometer,
    make: impl FnOnce(&str) -> ds18b20::linux::Thermometer,
) -> PamojaStatus {
    if out.is_null() {
        set_last_error("out_thermometer must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out = std::ptr::null_mut();
    let Some(text) = read_str(text, what) else {
        return PamojaStatus::InvalidArgument;
    };
    *out = Box::into_raw(Box::new(PamojaDs18b20Thermometer {
        thermometer: make(text),
    }));
    PamojaStatus::Ok
}

/// Runs a driver call that returns nothing, turning a failure or a panic into a status.
///
/// # Safety
///
/// `handle` must be a live handle or null.
unsafe fn run<H, E: Debug + Display>(
    handle: *mut H,
    call: impl FnOnce(&mut H) -> Result<(), DriverError<E>>,
) -> PamojaStatus {
    match on_driver(handle, call) {
        Ok(()) => PamojaStatus::Ok,
        Err(status) => status,
    }
}

/// Runs a driver call and writes what it returns through `out`, converted for the boundary.
///
/// # Safety
///
/// `handle` must be a live handle or null, and `out` a writable pointer or null.
unsafe fn fill<H, T, R, E: Debug + Display>(
    handle: *mut H,
    out: *mut R,
    name: &str,
    call: impl FnOnce(&mut H) -> Result<T, DriverError<E>>,
    convert: impl FnOnce(T) -> R,
) -> PamojaStatus {
    if out.is_null() {
        set_last_error(format!("{name} must not be null"));
        return PamojaStatus::InvalidArgument;
    }
    match on_driver(handle, call) {
        Ok(value) => {
            *out = convert(value);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Writes something a driver holds, without touching the bus, through `out`.
///
/// # Safety
///
/// `handle` must be a live handle or null, and `out` a writable pointer or null.
unsafe fn look<H, R>(
    handle: *const H,
    out: *mut R,
    name: &str,
    read: impl FnOnce(&H) -> R,
) -> PamojaStatus {
    if out.is_null() {
        set_last_error(format!("{name} must not be null"));
        return PamojaStatus::InvalidArgument;
    }
    let Some(held) = handle.as_ref() else {
        set_last_error("sensor must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out = read(held);
    PamojaStatus::Ok
}

/// Writes a value a driver may not have yet through `out`, reporting whether it had one.
///
/// # Safety
///
/// `out` must be a writable pointer or null.
unsafe fn store<R>(out: *mut R, value: Option<R>) -> bool {
    match (out.is_null(), value) {
        (false, Some(value)) => {
            *out = value;
            true
        }
        _ => false,
    }
}

/// Copies a block of bytes into a caller's buffer of exactly that size.
///
/// # Safety
///
/// `out` must point to `bytes.len()` writable bytes, or be null.
unsafe fn copy_out(out: *mut u8, name: &str, bytes: &[u8]) -> PamojaStatus {
    if out.is_null() {
        set_last_error(format!("{name} must not be null"));
        return PamojaStatus::InvalidArgument;
    }
    std::slice::from_raw_parts_mut(out, bytes.len()).copy_from_slice(bytes);
    PamojaStatus::Ok
}

/// Releases a boxed handle; a null pointer is ignored.
///
/// # Safety
///
/// `handle` must be a boxed handle that has not been freed, or null.
unsafe fn release<H>(handle: *mut H) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Runs a driver call, turning a failure or a panic into a status.
///
/// # Safety
///
/// `handle` must be a live handle or null.
unsafe fn on_driver<H, T, E: Debug + Display>(
    handle: *mut H,
    call: impl FnOnce(&mut H) -> Result<T, DriverError<E>>,
) -> Result<T, PamojaStatus> {
    let Some(held) = handle.as_mut() else {
        set_last_error("sensor must not be null".to_owned());
        return Err(PamojaStatus::InvalidArgument);
    };
    match catch_unwind(AssertUnwindSafe(|| call(held))) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(driver_failed(&error)),
        Err(_) => Err(panicked()),
    }
}

/// Records why a driver failed, in the bus's own words when the bus was the cause, and maps
/// it onto its status.
fn driver_failed<E: Debug + Display>(error: &DriverError<E>) -> PamojaStatus {
    match error {
        DriverError::Bus(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Io
        }
        DriverError::Timeout => {
            set_last_error(error.to_string());
            PamojaStatus::Io
        }
        DriverError::Sensor(sensor) => {
            set_last_error(sensor.to_string());
            PamojaStatus::Codec
        }
    }
}

/// Records a caught panic and reports it as [`PamojaStatus::Panic`].
fn panicked() -> PamojaStatus {
    set_last_error("panic at the FFI boundary".to_owned());
    PamojaStatus::Panic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::{
        pamoja_i2c_bus_attach, pamoja_i2c_bus_free, pamoja_i2c_bus_part, pamoja_i2c_bus_simulated,
        pamoja_i2c_bus_waited_micros, pamoja_i2c_part_free, pamoja_i2c_part_load,
        pamoja_i2c_part_new, pamoja_i2c_part_received, pamoja_i2c_part_received_count,
        pamoja_i2c_part_register, pamoja_i2c_part_word,
    };
    use crate::sensors::{
        pamoja_bme280_ctrl_hum_from_bits, pamoja_bme280_ctrl_meas_from_bits,
        pamoja_bme280_max_measurement_micros, PamojaBme280CtrlMeas,
        PAMOJA_BME280_I2C_ADDRESS_PRIMARY, PAMOJA_BME280_REGISTER_CHIP_ID,
        PAMOJA_BME280_REGISTER_CTRL_HUM, PAMOJA_BME280_REGISTER_CTRL_MEAS,
    };
    use std::ffi::{CStr, CString};
    use std::ptr;

    const PART: u8 = PAMOJA_BME280_I2C_ADDRESS_PRIMARY;

    fn last_error() -> String {
        let message = crate::pamoja_last_error_message();
        if message.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned()
    }

    fn measurement() -> PamojaBme280Measurement {
        PamojaBme280Measurement {
            celsius: 0.0,
            pascals: 0,
            hectopascals: 0.0,
            relative_humidity_percent: 0.0,
        }
    }

    /// A simulated bus holding one part, with the part's own handle already freed.
    unsafe fn bus_with(part: *mut PamojaI2cPart) -> *mut PamojaI2cBus {
        let bus = pamoja_i2c_bus_simulated();
        attach(bus, part);
        bus
    }

    /// Puts a part on a simulated bus and frees the part's own handle.
    unsafe fn attach(bus: *mut PamojaI2cBus, part: *mut PamojaI2cPart) {
        assert_eq!(pamoja_i2c_bus_attach(bus, part), PamojaStatus::Ok);
        pamoja_i2c_part_free(part);
    }

    #[test]
    fn a_driver_measures_a_simulated_part_and_leaves_it_configured() {
        unsafe {
            let bus = bus_with(pamoja_bme280_sim_part(PART));
            let mut sensor = ptr::null_mut();
            assert_eq!(
                pamoja_bme280_new(bus, PART, pamoja_bme280_settings_default(), &mut sensor),
                PamojaStatus::Ok
            );
            let mut reading = measurement();
            assert_eq!(
                pamoja_bme280_measure(sensor, &mut reading),
                PamojaStatus::Ok
            );
            assert_eq!(reading.celsius, 20.44);
            assert_eq!(reading.pascals, 84_805);

            let held = pamoja_i2c_bus_part(bus, PART);
            let mut ctrl = PamojaBme280CtrlMeas {
                temperature: 0,
                pressure: 0,
                mode: 0,
            };
            assert_eq!(
                pamoja_bme280_ctrl_meas_from_bits(
                    pamoja_i2c_part_register(held, PAMOJA_BME280_REGISTER_CTRL_MEAS),
                    &mut ctrl
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                ctrl.mode,
                bme280::Mode::Forced.code(),
                "the last write forced it"
            );
            assert_eq!(
                pamoja_bme280_ctrl_hum_from_bits(pamoja_i2c_part_register(
                    held,
                    PAMOJA_BME280_REGISTER_CTRL_HUM
                )),
                bme280::Oversampling::X1.code()
            );
            pamoja_i2c_part_free(held);

            assert_eq!(
                pamoja_i2c_bus_waited_micros(bus),
                u64::from(bme280::STARTUP_MICROS)
                    + u64::from(pamoja_bme280_max_measurement_micros(1, 1, 1)),
                "the reset's start-up time and one measurement's"
            );
            pamoja_bme280_free(sensor);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn a_part_asked_for_a_reading_reports_it() {
        unsafe {
            let bus = bus_with(pamoja_bme280_sim_reporting(PART, 4.0, 1013.25, 80.0));
            let mut sensor = ptr::null_mut();
            assert_eq!(
                pamoja_bme280_new(bus, PART, pamoja_bme280_settings_default(), &mut sensor),
                PamojaStatus::Ok
            );
            pamoja_i2c_bus_free(bus);

            let mut reading = measurement();
            assert_eq!(
                pamoja_bme280_measure(sensor, &mut reading),
                PamojaStatus::Ok
            );
            assert_eq!(reading.celsius, 4.0);
            assert!((reading.hectopascals - 1013.25).abs() < 0.01);
            assert!((reading.relative_humidity_percent - 80.0).abs() < 0.01);
            pamoja_bme280_free(sensor);
        }
    }

    #[test]
    fn a_missing_part_is_an_io_error_and_another_part_a_codec_error() {
        unsafe {
            let bus = pamoja_i2c_bus_simulated();
            let mut sensor = ptr::null_mut();
            assert_eq!(
                pamoja_bme280_new(bus, PART, pamoja_bme280_settings_default(), &mut sensor),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_bme280_init(sensor), PamojaStatus::Io);
            assert_eq!(last_error(), "nothing answered at 0x76");

            let bmp280 = pamoja_i2c_part_new(PART);
            assert_eq!(
                pamoja_i2c_part_load(bmp280, PAMOJA_BME280_REGISTER_CHIP_ID, [0x58].as_ptr(), 1),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_i2c_bus_attach(bus, bmp280), PamojaStatus::Ok);
            pamoja_i2c_part_free(bmp280);
            assert_eq!(pamoja_bme280_init(sensor), PamojaStatus::Codec);
            assert_eq!(last_error(), "sensor identification mismatch");

            pamoja_bme280_free(sensor);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn the_shipped_calibration_and_a_built_burst_compensate_to_the_reading() {
        unsafe {
            let mut temp_press = [0u8; PAMOJA_BME280_CALIBRATION_TEMP_PRESS_LEN];
            let mut humidity = [0u8; PAMOJA_BME280_CALIBRATION_HUMIDITY_LEN];
            assert_eq!(
                pamoja_bme280_sim_calibration(temp_press.as_mut_ptr(), humidity.as_mut_ptr()),
                PamojaStatus::Ok
            );
            let calibration = bme280::Calibration::from_registers(&temp_press, &humidity);

            let mut shipped = [0u8; PAMOJA_BME280_MEASUREMENT_LEN];
            assert_eq!(
                pamoja_bme280_sim_burst(shipped.as_mut_ptr()),
                PamojaStatus::Ok
            );
            let reading = calibration.compensate(&bme280::RawMeasurement::from_registers(&shipped));
            assert_eq!(reading.celsius(), 20.44);

            let mut burst = [0u8; PAMOJA_BME280_MEASUREMENT_LEN];
            assert_eq!(
                pamoja_bme280_sim_burst_for(-12.5, 990.0, 35.0, burst.as_mut_ptr()),
                PamojaStatus::Ok
            );
            let reading = calibration.compensate(&bme280::RawMeasurement::from_registers(&burst));
            assert_eq!(reading.celsius(), -12.5);
            assert!((reading.hectopascals() - 990.0).abs() < 0.01);
            assert!((reading.relative_humidity_percent() - 35.0).abs() < 0.01);
            assert_eq!(
                pamoja_bme280_sim_burst_for(0.0, 0.0, 0.0, ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_bme280_sim_burst(ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
        }
    }

    #[test]
    fn a_bmp280_reads_its_simulated_twin_and_keeps_the_trimming() {
        unsafe {
            let bus = bus_with(pamoja_bmp280_sim_reporting(0x77, -7.5, 1003.0));
            let mut sensor = ptr::null_mut();
            assert_eq!(
                pamoja_bmp280_new(bus, 0x77, pamoja_bmp280_settings_default(), &mut sensor),
                PamojaStatus::Ok
            );
            let mut coefficients = std::mem::zeroed::<PamojaBmp280Coefficients>();
            assert!(!pamoja_bmp280_coefficients(sensor, &mut coefficients));
            let mut reading = PamojaBmp280Reading {
                celsius: 0.0,
                pascals: 0,
                hectopascals: 0.0,
            };
            assert_eq!(
                pamoja_bmp280_measure(sensor, &mut reading),
                PamojaStatus::Ok
            );
            assert!((reading.celsius + 7.5).abs() < 0.01);
            assert!((reading.hectopascals - 1003.0).abs() < 0.01);
            assert!(pamoja_bmp280_coefficients(sensor, &mut coefficients));
            assert_eq!(
                coefficients,
                PamojaBmp280Coefficients::from(bmp280::sim::calibration())
            );

            let mut burst = [0u8; PAMOJA_BMP280_DATA_LEN];
            let mut calibration = [0u8; PAMOJA_BMP280_CALIBRATION_LEN];
            assert_eq!(
                pamoja_bmp280_sim_burst(burst.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_bmp280_sim_calibration(calibration.as_mut_ptr()),
                PamojaStatus::Ok
            );
            let compensated = bmp280::Calibration::parse(&calibration)
                .compensate(&bmp280::Measurement::parse(&burst));
            assert_eq!(compensated.celsius(), 20.44);
            pamoja_bmp280_free(sensor);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn a_tmp117_reads_its_twin_and_holds_the_alerts_it_consumed() {
        unsafe {
            let bus = bus_with(pamoja_tmp117_sim_part(0x48));
            let mut sensor = ptr::null_mut();
            assert_eq!(
                pamoja_tmp117_new(bus, 0x48, 1, &mut sensor),
                PamojaStatus::Ok
            );
            let mut revision = 0;
            assert!(!pamoja_tmp117_silicon_revision(sensor, &mut revision));
            let mut reading = PamojaTmp117Reading {
                raw: 0,
                micro_celsius: 0,
                celsius: 0.0,
            };
            assert_eq!(
                pamoja_tmp117_measure(sensor, &mut reading),
                PamojaStatus::Ok
            );
            assert_eq!(reading.celsius, PAMOJA_TMP117_SIM_CELSIUS);
            assert_eq!(reading.micro_celsius, 21_250_000);
            assert!(pamoja_tmp117_silicon_revision(sensor, &mut revision));
            assert_eq!(
                pamoja_tmp117_set_alert_limits(sensor, 30.0, 10.0),
                PamojaStatus::Ok
            );
            let mut alerts = PamojaTmp117Alerts { high: 1, low: 1 };
            assert_eq!(pamoja_tmp117_alerts(sensor, &mut alerts), PamojaStatus::Ok);
            assert_eq!(alerts, PamojaTmp117Alerts { high: 0, low: 0 });

            let held = pamoja_i2c_bus_part(bus, 0x48);
            assert_eq!(
                pamoja_i2c_part_word(held, tmp117::register::THIGH_LIMIT),
                tmp117::raw_from_celsius(30.0) as u16
            );
            pamoja_i2c_part_free(held);
            pamoja_tmp117_free(sensor);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn an_opt3001_and_an_hdc1080_read_their_twins() {
        unsafe {
            let bus = bus_with(pamoja_opt3001_sim_reporting(0x44, 1200.0));
            attach(bus, pamoja_hdc1080_sim_reporting(-20.0, 12.5));

            let mut light = ptr::null_mut();
            let settings = PamojaOpt3001Settings {
                long_conversion: 0,
                range_number: opt3001::RANGE_AUTOMATIC,
            };
            assert_eq!(
                pamoja_opt3001_new(bus, 0x44, settings, &mut light),
                PamojaStatus::Ok
            );
            let mut lux = PamojaOpt3001Reading {
                raw: 0,
                milli_lux: 0,
                lux: 0.0,
            };
            assert_eq!(pamoja_opt3001_measure(light, &mut lux), PamojaStatus::Ok);
            assert_eq!(lux.lux, 1200.0);
            assert_eq!(
                pamoja_i2c_bus_waited_micros(bus),
                100_000,
                "one short conversion"
            );
            let mut config = std::mem::zeroed::<PamojaOpt3001Config>();
            assert_eq!(
                pamoja_opt3001_configuration(light, &mut config),
                PamojaStatus::Ok
            );
            assert_eq!(config.long_conversion, 0);

            let mut climate = ptr::null_mut();
            let wrong = PamojaHdc1080Settings {
                temperature_resolution_bits: 12,
                humidity_resolution_bits: 14,
            };
            assert_eq!(
                pamoja_hdc1080_new(bus, wrong, &mut climate),
                PamojaStatus::InvalidArgument
            );
            assert!(climate.is_null());
            assert_eq!(
                pamoja_hdc1080_new(bus, pamoja_hdc1080_settings_default(), &mut climate),
                PamojaStatus::Ok
            );
            let mut reading = std::mem::zeroed::<PamojaHdc1080Measurement>();
            assert_eq!(
                pamoja_hdc1080_measure(climate, &mut reading),
                PamojaStatus::Ok
            );
            assert!((reading.celsius + 20.0).abs() < 0.003);
            assert!((reading.relative_humidity - 12.5).abs() < 0.002);
            assert_eq!(pamoja_hdc1080_heater(climate, true), PamojaStatus::Ok);
            let mut config = std::mem::zeroed::<PamojaHdc1080Config>();
            assert_eq!(
                pamoja_hdc1080_configuration(climate, &mut config),
                PamojaStatus::Ok
            );
            assert_eq!(config.heater, 1);

            pamoja_opt3001_free(light);
            pamoja_hdc1080_free(climate);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn the_current_monitors_read_the_load_their_twins_carry() {
        unsafe {
            let bus = bus_with(pamoja_ina219_sim_part(0x40));
            attach(
                bus,
                pamoja_ina226_sim_reporting(0x45, 2, 20_000_000, 3_300_000, -10_000_000),
            );

            let mut solar = ptr::null_mut();
            assert_eq!(
                pamoja_ina219_new(bus, 0x40, pamoja_ina219_settings_default(), &mut solar),
                PamojaStatus::Ok
            );
            let mut reading = std::mem::zeroed::<PamojaIna219Reading>();
            assert_eq!(pamoja_ina219_measure(solar, &mut reading), PamojaStatus::Ok);
            assert_eq!(reading.bus_millivolts, PAMOJA_INA219_SIM_BUS_MILLIVOLTS);
            assert!(
                (reading.current_microamps - PAMOJA_INA219_SIM_MICROAMPS).abs()
                    < reading.current_lsb_microamps as i32
            );
            assert_eq!(
                pamoja_ina219_current_lsb(solar),
                crate::sensors::pamoja_ina219_minimum_current_lsb_microamps(3_200_000)
            );
            assert_ne!(pamoja_ina219_calibration_word(solar), 0);

            let mut battery = ptr::null_mut();
            let settings = PamojaIna226Settings {
                shunt_milliohms: 2,
                max_microamps: 20_000_000,
                ..pamoja_ina226_settings_default()
            };
            assert_eq!(
                pamoja_ina226_new(bus, 0x45, settings, &mut battery),
                PamojaStatus::Ok
            );
            let mut reading = std::mem::zeroed::<PamojaIna226Reading>();
            assert_eq!(
                pamoja_ina226_measure(battery, &mut reading),
                PamojaStatus::Ok
            );
            assert_eq!(reading.bus_microvolts, 3_300_000);
            assert!((reading.current_amps + 10.0).abs() < 0.001);
            let mut die = PamojaIna226DieId {
                device: 0,
                revision: 0,
            };
            assert!(pamoja_ina226_identity(battery, &mut die));
            assert_eq!(die.device, ina226::DEVICE_ID);

            pamoja_ina219_free(solar);
            pamoja_ina226_free(battery);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn an_ads1115_converts_at_the_range_its_twin_was_built_for() {
        unsafe {
            let wide = ads1115::Pga::Fsr4_096.code();
            let bus = bus_with(pamoja_ads1115_sim_reporting(0x48, wide, 3.0));
            let mut adc = ptr::null_mut();
            let settings = PamojaAds1115Settings {
                pga: wide,
                ..pamoja_ads1115_settings_default()
            };
            assert_eq!(
                pamoja_ads1115_new(bus, 0x48, settings, &mut adc),
                PamojaStatus::Ok
            );
            let mut sample = std::mem::zeroed::<PamojaAds1115Sample>();
            assert_eq!(pamoja_ads1115_sample(adc, &mut sample), PamojaStatus::Ok);
            assert!((sample.volts - 3.0).abs() < 0.001);
            assert_eq!(sample.pga, wide);
            assert_eq!(
                pamoja_ads1115_sample_input(adc, ads1115::Mux::Ain3Gnd.code(), &mut sample),
                PamojaStatus::Ok
            );
            let mut config = std::mem::zeroed::<PamojaAds1115Config>();
            assert_eq!(pamoja_ads1115_config(adc, &mut config), PamojaStatus::Ok);
            assert_eq!(
                config.mux,
                ads1115::Mux::Ain0Ain1.code(),
                "one input once, then back"
            );
            pamoja_ads1115_free(adc);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn the_sensirion_parts_answer_their_commands() {
        unsafe {
            let bus = bus_with(pamoja_sht3x_sim_reporting(0x45, -12.0, 88.5));
            attach(bus, pamoja_scd4x_sim_part());

            let mut sht = ptr::null_mut();
            assert_eq!(
                pamoja_sht3x_new(bus, 0x45, 7, &mut sht),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(pamoja_sht3x_new(bus, 0x45, 2, &mut sht), PamojaStatus::Ok);
            let mut status = std::mem::zeroed::<PamojaSht3xStatus>();
            assert!(!pamoja_sht3x_last_status(sht, &mut status));
            let mut reading = std::mem::zeroed::<PamojaSht3xMeasurement>();
            assert_eq!(pamoja_sht3x_measure(sht, &mut reading), PamojaStatus::Ok);
            assert!((reading.celsius + 12.0).abs() < 0.003);
            assert!(pamoja_sht3x_last_status(sht, &mut status));
            assert_eq!(pamoja_sht3x_heater_on(sht), PamojaStatus::Ok);

            let held = pamoja_i2c_bus_part(bus, 0x45);
            assert_eq!(pamoja_i2c_part_received_count(held), 4);
            let first = pamoja_i2c_part_received(held, 0);
            assert_eq!(
                *crate::pamoja_buffer_data(first),
                sht3x::command::SOFT_RESET.to_be_bytes()[0],
                "the reset went first"
            );
            crate::pamoja_buffer_free(first);
            pamoja_i2c_part_free(held);

            let mut co2 = ptr::null_mut();
            assert_eq!(pamoja_scd4x_new(bus, &mut co2), PamojaStatus::Ok);
            let mut serial = 0;
            assert!(!pamoja_scd4x_serial(co2, &mut serial));
            let mut air = std::mem::zeroed::<PamojaScd4xMeasurement>();
            assert_eq!(pamoja_scd4x_measure(co2, &mut air), PamojaStatus::Ok);
            assert_eq!(air.co2_ppm, PAMOJA_SCD4X_SIM_CO2_PPM);
            assert!(pamoja_scd4x_serial(co2, &mut serial));
            assert_eq!(serial, PAMOJA_SCD4X_SIM_SERIAL);
            let mut ready = false;
            assert_eq!(pamoja_scd4x_poll_ready(co2, &mut ready), PamojaStatus::Ok);
            assert!(ready);
            assert_eq!(
                pamoja_scd4x_set_sensor_altitude(co2, 1_600),
                PamojaStatus::Ok
            );

            pamoja_sht3x_free(sht);
            pamoja_scd4x_free(co2);
            pamoja_i2c_bus_free(bus);
        }
    }

    #[test]
    fn a_thermometer_reads_the_file_the_kernel_serves() {
        let raw = ds18b20::temperature_from_celsius(21.5, ds18b20::Resolution::Bits12);
        let bytes = ds18b20::Scratchpad::new(raw, ds18b20::Resolution::Bits12, 75, -10).to_bytes();
        let hex: Vec<String> = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        let text = format!("{} : crc={:02x} YES\n", hex.join(" "), bytes[8]);

        let dir = std::env::temp_dir().join(format!("pamoja-ffi-w1-{}", std::process::id()));
        let device = dir.join("28-000005e2fdc3");
        std::fs::create_dir_all(&device).unwrap();
        std::fs::write(device.join("w1_slave"), &text).unwrap();

        unsafe {
            let devices = CString::new(dir.to_string_lossy().into_owned()).unwrap();
            let mut found = ptr::null_mut();
            assert_eq!(
                pamoja_ds18b20_discover(devices.as_ptr(), &mut found),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_ds18b20_thermometers_len(found), 1);
            let probe = pamoja_ds18b20_thermometers_get(found, 0);
            assert!(pamoja_ds18b20_thermometers_get(found, 1).is_null());
            pamoja_ds18b20_thermometers_free(found);

            let mut reading = std::mem::zeroed::<PamojaDs18b20Reading>();
            assert_eq!(
                pamoja_ds18b20_thermometer_read(probe, &mut reading),
                PamojaStatus::Ok
            );
            assert_eq!(reading.micro_celsius, 21_500_000);
            let path = pamoja_ds18b20_thermometer_path(probe);
            let path_text = CStr::from_ptr(crate::pamoja_string_data(path))
                .to_string_lossy()
                .into_owned();
            assert!(path_text.ends_with("w1_slave"), "{path_text}");
            crate::pamoja_string_free(path);
            pamoja_ds18b20_thermometer_free(probe);

            let text = CString::new(text).unwrap();
            assert_eq!(
                crate::sensors::pamoja_ds18b20_parse_w1_slave(text.as_ptr(), &mut reading),
                PamojaStatus::Ok
            );

            let serial = CString::new("0000deadbeef").unwrap();
            let mut missing = ptr::null_mut();
            assert_eq!(
                pamoja_ds18b20_thermometer_new(serial.as_ptr(), &mut missing),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_ds18b20_thermometer_read(missing, &mut reading),
                PamojaStatus::Io
            );
            assert!(
                last_error().starts_with("reading the w1_slave file"),
                "{}",
                last_error()
            );
            pamoja_ds18b20_thermometer_free(missing);
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
