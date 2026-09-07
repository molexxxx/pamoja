//! Generated Node bindings for the sensor drivers.
//!
//! These mirror the `pamoja-sensors` Rust API: the decode half of eleven common
//! parts, turning the register bytes a bus driver read into the physical reading
//! the datasheet says they mean.
//!
//! A BME280 or BMP280 calibration is read once at start-up and reused for every
//! measurement, so each is a class. Everything else is a plain function over the
//! bytes or the register value a caller already holds.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_sensors::{
    ads1115, bme280, bmp280, ds18b20, hdc1080, ina219, ina226, opt3001, scd4x, sht3x, tmp117,
    SensorError,
};

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

/// Builds the nine bytes a DS18B20 in the given state puts on the bus, CRC last.
#[napi(js_name = "ds18b20BuildScratchpad")]
pub fn ds18b20_build_scratchpad(
    celsius: f64,
    bits: u8,
    alarm_high: i8,
    alarm_low: i8,
) -> napi::Result<Buffer> {
    let resolution = resolution(bits)?;
    let raw = ds18b20::temperature_from_celsius(celsius as f32, resolution);
    let scratchpad = ds18b20::Scratchpad::new(raw, resolution, alarm_high, alarm_low);
    Ok(Buffer::from(scratchpad.to_bytes().to_vec()))
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

/// Builds the INA219 shunt-voltage register a monitor reports for a shunt voltage.
#[napi]
pub fn ina219_shunt_register(microvolts: i32) -> i16 {
    ina219::shunt_register(microvolts)
}

/// Builds the INA219 bus-voltage register a monitor reports for a bus voltage.
#[napi]
pub fn ina219_bus_register(millivolts: u32) -> u16 {
    ina219::bus_register(millivolts)
}

/// Builds the INA219 current register a monitor reports for a current.
#[napi]
pub fn ina219_current_register(microamps: i32, current_lsb_microamps: u32) -> i16 {
    ina219::current_register(microamps, current_lsb_microamps)
}

/// Builds the INA219 power register a monitor reports for a power.
#[napi]
pub fn ina219_power_register(microwatts: u32, current_lsb_microamps: u32) -> u16 {
    ina219::power_register(microwatts, current_lsb_microamps)
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

/// A BMP280's per-chip trimming coefficients, as they sit in its registers.
#[napi(object)]
pub struct Bmp280Coefficients {
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
#[napi(object)]
pub struct Bmp280Reading {
    /// The temperature in degrees Celsius.
    pub celsius: f64,
    /// The pressure in pascals.
    pub pascals: u32,
    /// The pressure in hectopascals, the unit a barometer is usually quoted in.
    pub hectopascals: f64,
}

/// The uncompensated codes a BMP280 burst read carries.
#[napi(object)]
pub struct Bmp280Measurement {
    /// The 20-bit pressure code.
    pub pressure: u32,
    /// The 20-bit temperature code.
    pub temperature: u32,
    /// Whether pressure oversampling was off, so the code carries no reading.
    pub pressure_skipped: bool,
    /// Whether temperature oversampling was off, so the code carries no reading.
    pub temperature_skipped: bool,
}

/// A BMP280 `ctrl_meas` register, field by field.
#[napi(object)]
pub struct Bmp280CtrlMeas {
    /// The temperature oversampling code, `0..=5`, where `0` skips the measurement.
    pub temperature: u8,
    /// The pressure oversampling code, `0..=5`, where `0` skips the measurement.
    pub pressure: u8,
    /// The power mode code: `0` sleep, `1` forced, `3` normal.
    pub mode: u8,
}

/// A BMP280 `config` register, field by field.
#[napi(object)]
pub struct Bmp280Config {
    /// The normal-mode standby code, `0..=7`.
    pub standby: u8,
    /// The IIR filter code, `0..=7`.
    pub filter: u8,
    /// Whether the 3-wire SPI interface is enabled.
    pub spi3wire: bool,
}

/// A BMP280's factory calibration, read once and reused for every measurement.
#[napi]
pub struct Bmp280Calibration {
    inner: bmp280::Calibration,
}

#[napi]
impl Bmp280Calibration {
    /// Builds a calibration from the 24 bytes read out of the device's registers.
    #[napi(constructor)]
    pub fn new(bytes: Buffer) -> napi::Result<Self> {
        let registers: [u8; bmp280::CALIBRATION_LEN] = bytes
            .as_ref()
            .try_into()
            .map_err(|_| length_error("calibration", bmp280::CALIBRATION_LEN))?;
        Ok(Self {
            inner: bmp280::Calibration::parse(&registers),
        })
    }

    /// Turns a six-byte burst read into a compensated reading.
    #[napi]
    pub fn compensate(&self, measurement: Buffer) -> napi::Result<Bmp280Reading> {
        let registers: [u8; bmp280::DATA_LEN] = measurement
            .as_ref()
            .try_into()
            .map_err(|_| length_error("measurement", bmp280::DATA_LEN))?;
        Ok(self
            .inner
            .compensate(&bmp280::Measurement::parse(&registers))
            .into())
    }

    /// Rebuilds the 24 calibration bytes a device holding these coefficients returns.
    #[napi]
    pub fn to_bytes(&self) -> Buffer {
        Buffer::from(self.inner.to_bytes().to_vec())
    }

    /// The trimming coefficients the calibration bytes carried.
    #[napi(getter)]
    pub fn coefficients(&self) -> Bmp280Coefficients {
        self.inner.into()
    }
}

/// Unpacks the six data bytes a BMP280 burst read returns.
#[napi]
pub fn bmp280_parse_measurement(data: Buffer) -> napi::Result<Bmp280Measurement> {
    let registers: [u8; bmp280::DATA_LEN] = data
        .as_ref()
        .try_into()
        .map_err(|_| length_error("measurement", bmp280::DATA_LEN))?;
    Ok(bmp280::Measurement::parse(&registers).into())
}

/// Reports whether a raw pressure code says the channel's oversampling is off.
#[napi]
pub fn bmp280_pressure_skipped(pressure: u32) -> bool {
    bmp280::Measurement {
        pressure,
        temperature: 0,
    }
    .pressure_skipped()
}

/// Reports whether a raw temperature code says the channel's oversampling is off.
#[napi]
pub fn bmp280_temperature_skipped(temperature: u32) -> bool {
    bmp280::Measurement {
        pressure: 0,
        temperature,
    }
    .temperature_skipped()
}

/// Builds the six data bytes a BMP280 holding these codes would return.
#[napi]
pub fn bmp280_measurement_bytes(pressure: u32, temperature: u32) -> Buffer {
    let measurement = bmp280::Measurement {
        pressure,
        temperature,
    };
    Buffer::from(measurement.to_bytes().to_vec())
}

/// Reports whether a BMP280 status byte says a conversion is running.
#[napi]
pub fn bmp280_measuring(status: u8) -> bool {
    bmp280::measuring(status)
}

/// Reports whether a BMP280 status byte says the calibration image is loading.
#[napi]
pub fn bmp280_image_updating(status: u8) -> bool {
    bmp280::image_updating(status)
}

/// Packs a BMP280 `ctrl_meas` register value.
#[napi]
pub fn bmp280_ctrl_meas_bits(config: Bmp280CtrlMeas) -> u8 {
    bmp280::CtrlMeas::from(config).bits()
}

/// Parses a BMP280 `ctrl_meas` register value.
#[napi]
pub fn bmp280_ctrl_meas_from_bits(bits: u8) -> Bmp280CtrlMeas {
    bmp280::CtrlMeas::from_bits(bits).into()
}

/// Packs a BMP280 `config` register value.
#[napi]
pub fn bmp280_config_bits(config: Bmp280Config) -> u8 {
    bmp280::Config::from(config).bits()
}

/// Parses a BMP280 `config` register value.
#[napi]
pub fn bmp280_config_from_bits(bits: u8) -> Bmp280Config {
    bmp280::Config::from_bits(bits).into()
}

/// Returns how many samples a BMP280 oversampling code averages.
#[napi]
pub fn bmp280_oversampling_factor(code: u8) -> u8 {
    bmp280::Oversampling::from_code(code).factor()
}

/// Returns the normal-mode standby period a BMP280 code selects, in microseconds.
#[napi]
pub fn bmp280_standby_micros(code: u8) -> u32 {
    bmp280::Standby::from_code(code).microseconds()
}

/// How hard an SHT3x works at one measurement, trading noise against time and power.
#[napi(string_enum, js_name = "Sht3xRepeatability")]
pub enum Sht3xRepeatability {
    /// The fastest and least precise setting.
    Low,
    /// The middle setting.
    Medium,
    /// The slowest and most precise setting.
    High,
}

/// How often an SHT3x in periodic mode takes a measurement.
#[napi(string_enum, js_name = "Sht3xRate")]
pub enum Sht3xRate {
    /// One measurement every two seconds.
    HalfMps,
    /// One measurement per second.
    OneMps,
    /// Two measurements per second.
    TwoMps,
    /// Four measurements per second.
    FourMps,
    /// Ten measurements per second.
    TenMps,
}

/// A decoded SHT3x temperature and humidity pair.
#[napi(object, js_name = "Sht3xMeasurement")]
pub struct Sht3xMeasurement {
    /// The raw temperature word.
    pub temperature_raw: u16,
    /// The raw humidity word.
    pub humidity_raw: u16,
    /// The temperature in milli-degrees Celsius, exact in integer arithmetic.
    pub milli_celsius: i32,
    /// The temperature in degrees Celsius.
    pub celsius: f64,
    /// The temperature in milli-degrees Fahrenheit.
    pub milli_fahrenheit: i32,
    /// The temperature in degrees Fahrenheit.
    pub fahrenheit: f64,
    /// The relative humidity in milli-percent.
    pub milli_percent: u32,
    /// The relative humidity as a percentage.
    pub relative_humidity: f64,
}

/// A decoded SHT3x status register.
#[napi(object, js_name = "Sht3xStatus")]
pub struct Sht3xStatus {
    /// The 16-bit status word the flags were read from.
    pub bits: u16,
    /// Whether at least one alert condition is pending.
    pub alert_pending: bool,
    /// Whether the on-die heater is running.
    pub heater_on: bool,
    /// Whether a humidity tracking alert is set.
    pub humidity_tracking_alert: bool,
    /// Whether a temperature tracking alert is set.
    pub temperature_tracking_alert: bool,
    /// Whether the part has reset since the flag was last cleared.
    pub reset_detected: bool,
    /// Whether the last command could not be processed.
    pub command_failed: bool,
    /// Whether the last write failed its checksum.
    pub write_checksum_failed: bool,
}

/// Computes the CRC-8 an SHT3x appends to every data word.
#[napi(js_name = "sht3xCrc")]
pub fn sht3x_crc(data: Buffer) -> u8 {
    sht3x::crc(data.as_ref())
}

/// Reads a CRC-checked three-byte SHT3x word frame.
#[napi(js_name = "sht3xWord")]
pub fn sht3x_word(frame: Buffer) -> napi::Result<u16> {
    let frame: [u8; 3] = frame
        .as_ref()
        .try_into()
        .map_err(|_| length_error("word frame", 3))?;
    sht3x::word(&frame).map_err(to_napi)
}

/// Builds the three bytes an SHT3x sends for a word: the word then its CRC.
#[napi(js_name = "sht3xWordBytes")]
pub fn sht3x_word_bytes(value: u16) -> Buffer {
    Buffer::from(sht3x::word_bytes(value).to_vec())
}

/// Parses and CRC-checks a six-byte SHT3x measurement frame.
#[napi(js_name = "sht3xParseMeasurement")]
pub fn sht3x_parse_measurement(frame: Buffer) -> napi::Result<Sht3xMeasurement> {
    let frame: [u8; 6] = frame
        .as_ref()
        .try_into()
        .map_err(|_| length_error("measurement frame", 6))?;
    Ok(sht3x::Measurement::parse(&frame).map_err(to_napi)?.into())
}

/// Builds the six bytes an SHT3x sends for a pair of raw words.
#[napi(js_name = "sht3xMeasurementBytes")]
pub fn sht3x_measurement_bytes(temperature_raw: u16, humidity_raw: u16) -> Buffer {
    let measurement = sht3x::Measurement {
        temperature_raw,
        humidity_raw,
    };
    Buffer::from(measurement.to_bytes().to_vec())
}

/// Converts a raw SHT3x temperature word to milli-degrees Celsius.
#[napi(js_name = "sht3xMilliCelsius")]
pub fn sht3x_milli_celsius(raw: u16) -> i32 {
    sht3x::milli_celsius(raw)
}

/// Converts a raw SHT3x temperature word to degrees Celsius.
#[napi(js_name = "sht3xCelsius")]
pub fn sht3x_celsius(raw: u16) -> f64 {
    f64::from(sht3x::celsius(raw))
}

/// Converts a raw SHT3x temperature word to milli-degrees Fahrenheit.
#[napi(js_name = "sht3xMilliFahrenheit")]
pub fn sht3x_milli_fahrenheit(raw: u16) -> i32 {
    sht3x::milli_fahrenheit(raw)
}

/// Converts a raw SHT3x temperature word to degrees Fahrenheit.
#[napi(js_name = "sht3xFahrenheit")]
pub fn sht3x_fahrenheit(raw: u16) -> f64 {
    f64::from(sht3x::fahrenheit(raw))
}

/// Converts a raw SHT3x humidity word to milli-percent.
#[napi(js_name = "sht3xMilliPercent")]
pub fn sht3x_milli_percent(raw: u16) -> u32 {
    sht3x::milli_percent(raw)
}

/// Converts a raw SHT3x humidity word to a relative humidity percentage.
#[napi(js_name = "sht3xRelativeHumidity")]
pub fn sht3x_relative_humidity(raw: u16) -> f64 {
    f64::from(sht3x::relative_humidity(raw))
}

/// Builds the SHT3x temperature word that decodes to a temperature.
#[napi(js_name = "sht3xTemperatureRawFromMilliCelsius")]
pub fn sht3x_temperature_raw_from_milli_celsius(milli_celsius: i32) -> u16 {
    sht3x::temperature_raw_from_milli_celsius(milli_celsius)
}

/// Builds the SHT3x temperature word that decodes to a temperature in Celsius.
#[napi(js_name = "sht3xTemperatureRawFromCelsius")]
pub fn sht3x_temperature_raw_from_celsius(celsius: f64) -> u16 {
    sht3x::temperature_raw_from_celsius(celsius as f32)
}

/// Builds the SHT3x temperature word that decodes to a temperature in Fahrenheit.
#[napi(js_name = "sht3xTemperatureRawFromMilliFahrenheit")]
pub fn sht3x_temperature_raw_from_milli_fahrenheit(milli_fahrenheit: i32) -> u16 {
    sht3x::temperature_raw_from_milli_fahrenheit(milli_fahrenheit)
}

/// Builds the SHT3x humidity word that decodes to a relative humidity.
#[napi(js_name = "sht3xHumidityRawFromMilliPercent")]
pub fn sht3x_humidity_raw_from_milli_percent(milli_percent: u32) -> u16 {
    sht3x::humidity_raw_from_milli_percent(milli_percent)
}

/// Builds the SHT3x humidity word that decodes to a relative humidity percentage.
#[napi(js_name = "sht3xHumidityRawFromRelativeHumidity")]
pub fn sht3x_humidity_raw_from_relative_humidity(percent: f64) -> u16 {
    sht3x::humidity_raw_from_relative_humidity(percent as f32)
}

/// Parses and CRC-checks a three-byte SHT3x status frame.
#[napi(js_name = "sht3xParseStatus")]
pub fn sht3x_parse_status(frame: Buffer) -> napi::Result<Sht3xStatus> {
    let frame: [u8; 3] = frame
        .as_ref()
        .try_into()
        .map_err(|_| length_error("status frame", 3))?;
    Ok(sht3x::Status::parse(&frame).map_err(to_napi)?.into())
}

/// Splits an SHT3x status word into its flags.
#[napi(js_name = "sht3xStatusFromBits")]
pub fn sht3x_status_from_bits(bits: u16) -> Sht3xStatus {
    sht3x::Status::from_bits(bits).into()
}

/// Builds the three bytes an SHT3x sends for a status word, CRC last.
#[napi(js_name = "sht3xStatusBytes")]
pub fn sht3x_status_bytes(bits: u16) -> Buffer {
    Buffer::from(sht3x::Status::from_bits(bits).to_bytes().to_vec())
}

/// Returns the SHT3x single-shot command for a repeatability and clock mode.
#[napi(js_name = "sht3xSingleShot")]
pub fn sht3x_single_shot(repeatability: Sht3xRepeatability, clock_stretching: bool) -> u16 {
    sht3x::single_shot(repeatability.into(), clock_stretching)
}

/// Returns the SHT3x periodic-mode command for a repeatability and rate.
#[napi(js_name = "sht3xPeriodic")]
pub fn sht3x_periodic(repeatability: Sht3xRepeatability, rate: Sht3xRate) -> u16 {
    sht3x::periodic(repeatability.into(), rate.into())
}

/// Returns how long an SHT3x measurement may take, in microseconds.
#[napi(js_name = "sht3xMaxMeasurementMicros")]
pub fn sht3x_max_measurement_micros(repeatability: Sht3xRepeatability) -> u32 {
    sht3x::Repeatability::from(repeatability).max_measurement_micros()
}

/// Returns how long an SHT3x measurement typically takes, in microseconds.
#[napi(js_name = "sht3xTypicalMeasurementMicros")]
pub fn sht3x_typical_measurement_micros(repeatability: Sht3xRepeatability) -> u32 {
    sht3x::Repeatability::from(repeatability).typical_measurement_micros()
}

/// Returns the gap between SHT3x periodic measurements, in microseconds.
#[napi(js_name = "sht3xIntervalMicros")]
pub fn sht3x_interval_micros(rate: Sht3xRate) -> u32 {
    sht3x::Rate::from(rate).interval_micros()
}

/// A decoded SCD4x measurement frame.
#[napi(object, js_name = "Scd4xMeasurement")]
pub struct Scd4xMeasurement {
    /// The CO2 concentration in parts per million.
    pub co2_ppm: u16,
    /// The raw temperature word.
    pub temperature_raw: u16,
    /// The raw humidity word.
    pub humidity_raw: u16,
    /// The temperature in milli-degrees Celsius, exact in integer arithmetic.
    pub milli_celsius: i32,
    /// The temperature in degrees Celsius.
    pub celsius: f64,
    /// The relative humidity in milli-percent.
    pub humidity_milli_percent: u32,
    /// The relative humidity as a percentage.
    pub relative_humidity_percent: f64,
}

/// Computes the CRC-8 an SCD4x appends to every data word.
#[napi(js_name = "scd4xCrc")]
pub fn scd4x_crc(data: Buffer) -> u8 {
    scd4x::crc(data.as_ref())
}

/// Reads a CRC-checked three-byte SCD4x word frame.
#[napi(js_name = "scd4xWord")]
pub fn scd4x_word(frame: Buffer) -> napi::Result<u16> {
    let frame: [u8; 3] = frame
        .as_ref()
        .try_into()
        .map_err(|_| length_error("word frame", 3))?;
    scd4x::word(&frame).map_err(to_napi)
}

/// Builds the three bytes an SCD4x sends for a word: the word then its CRC.
#[napi(js_name = "scd4xWordFrame")]
pub fn scd4x_word_frame(value: u16) -> Buffer {
    Buffer::from(scd4x::word_frame(value).to_vec())
}

/// Builds the two bytes that send a bare SCD4x command.
#[napi(js_name = "scd4xCommandFrame")]
pub fn scd4x_command_frame(command: u16) -> Buffer {
    Buffer::from(scd4x::command_frame(command).to_vec())
}

/// Builds the five bytes that send an SCD4x command with an argument.
#[napi(js_name = "scd4xWriteFrame")]
pub fn scd4x_write_frame(command: u16, value: u16) -> Buffer {
    Buffer::from(scd4x::write_frame(command, value).to_vec())
}

/// Returns how long an SCD4x command may take, in milliseconds.
#[napi(js_name = "scd4xMaxDurationMs")]
pub fn scd4x_max_duration_ms(command: u16) -> Option<u16> {
    scd4x::max_duration_ms(command)
}

/// Reports whether an SCD4x accepts a command while it is measuring.
#[napi(js_name = "scd4xAllowedDuringMeasurement")]
pub fn scd4x_allowed_during_measurement(command: u16) -> bool {
    scd4x::allowed_during_measurement(command)
}

/// Parses and CRC-checks a nine-byte SCD4x measurement frame.
#[napi(js_name = "scd4xParseMeasurement")]
pub fn scd4x_parse_measurement(frame: Buffer) -> napi::Result<Scd4xMeasurement> {
    let frame: [u8; 9] = frame
        .as_ref()
        .try_into()
        .map_err(|_| length_error("measurement frame", 9))?;
    Ok(scd4x::Measurement::parse(&frame).map_err(to_napi)?.into())
}

/// Builds the SCD4x measurement a sensor reporting these physical values would send.
#[napi(js_name = "scd4xMeasurementFromPhysical")]
pub fn scd4x_measurement_from_physical(
    co2_ppm: u16,
    milli_celsius: i32,
    humidity_milli_percent: u32,
) -> Scd4xMeasurement {
    scd4x::Measurement::from_physical(co2_ppm, milli_celsius, humidity_milli_percent).into()
}

/// Builds the nine bytes an SCD4x sends for a set of raw words.
#[napi(js_name = "scd4xMeasurementBytes")]
pub fn scd4x_measurement_bytes(co2_ppm: u16, temperature_raw: u16, humidity_raw: u16) -> Buffer {
    let measurement = scd4x::Measurement {
        co2_ppm,
        temperature_raw,
        humidity_raw,
    };
    Buffer::from(measurement.to_bytes().to_vec())
}

/// Converts a raw SCD4x temperature word to milli-degrees Celsius.
#[napi(js_name = "scd4xMilliCelsius")]
pub fn scd4x_milli_celsius(raw: u16) -> i32 {
    scd4x::milli_celsius(raw)
}

/// Converts a raw SCD4x temperature word to degrees Celsius.
#[napi(js_name = "scd4xCelsius")]
pub fn scd4x_celsius(raw: u16) -> f64 {
    f64::from(scd4x::celsius(raw))
}

/// Builds the SCD4x temperature word that decodes to a temperature.
#[napi(js_name = "scd4xTemperatureRaw")]
pub fn scd4x_temperature_raw(milli_celsius: i32) -> u16 {
    scd4x::temperature_raw(milli_celsius)
}

/// Converts a raw SCD4x humidity word to milli-percent.
#[napi(js_name = "scd4xHumidityMilliPercent")]
pub fn scd4x_humidity_milli_percent(raw: u16) -> u32 {
    scd4x::humidity_milli_percent(raw)
}

/// Converts a raw SCD4x humidity word to a relative humidity percentage.
#[napi(js_name = "scd4xRelativeHumidityPercent")]
pub fn scd4x_relative_humidity_percent(raw: u16) -> f64 {
    f64::from(scd4x::relative_humidity_percent(raw))
}

/// Builds the SCD4x humidity word that decodes to a relative humidity.
#[napi(js_name = "scd4xHumidityRaw")]
pub fn scd4x_humidity_raw(milli_percent: u32) -> u16 {
    scd4x::humidity_raw(milli_percent)
}

/// Reports whether an SCD4x data-ready word says a measurement is waiting.
#[napi(js_name = "scd4xDataReady")]
pub fn scd4x_data_ready(word: u16) -> bool {
    scd4x::data_ready(word)
}

/// Builds the SCD4x word that programs a temperature offset.
#[napi(js_name = "scd4xTemperatureOffsetWord")]
pub fn scd4x_temperature_offset_word(milli_celsius: u32) -> u16 {
    scd4x::temperature_offset_word(milli_celsius)
}

/// Converts an SCD4x temperature-offset word back to milli-degrees Celsius.
#[napi(js_name = "scd4xTemperatureOffsetMilliCelsius")]
pub fn scd4x_temperature_offset_milli_celsius(word: u16) -> u32 {
    scd4x::temperature_offset_milli_celsius(word)
}

/// Builds the SCD4x word that programs an ambient pressure.
#[napi(js_name = "scd4xAmbientPressureWord")]
pub fn scd4x_ambient_pressure_word(pascals: u32) -> u16 {
    scd4x::ambient_pressure_word(pascals)
}

/// Converts an SCD4x ambient-pressure word back to pascals.
#[napi(js_name = "scd4xAmbientPressurePascals")]
pub fn scd4x_ambient_pressure_pascals(word: u16) -> u32 {
    scd4x::ambient_pressure_pascals(word)
}

/// Reads the correction an SCD4x reports after a forced recalibration.
#[napi(js_name = "scd4xForcedRecalibrationCorrectionPpm")]
pub fn scd4x_forced_recalibration_correction_ppm(word: u16) -> Option<i32> {
    scd4x::forced_recalibration_correction_ppm(word)
}

/// Builds the word an SCD4x returns for a forced-recalibration outcome.
#[napi(js_name = "scd4xForcedRecalibrationWord")]
pub fn scd4x_forced_recalibration_word(correction_ppm: Option<i32>) -> u16 {
    scd4x::forced_recalibration_word(correction_ppm)
}

/// Reports whether an SCD4x word says automatic self-calibration is on.
#[napi(js_name = "scd4xAutomaticSelfCalibrationEnabled")]
pub fn scd4x_automatic_self_calibration_enabled(word: u16) -> bool {
    scd4x::automatic_self_calibration_enabled(word)
}

/// Builds the SCD4x word that turns automatic self-calibration on or off.
#[napi(js_name = "scd4xAutomaticSelfCalibrationWord")]
pub fn scd4x_automatic_self_calibration_word(enabled: bool) -> u16 {
    scd4x::automatic_self_calibration_word(enabled)
}

/// Reports whether an SCD4x self-test word says the part passed.
#[napi(js_name = "scd4xSelfTestPassed")]
pub fn scd4x_self_test_passed(word: u16) -> bool {
    scd4x::self_test_passed(word)
}

/// Reads a CRC-checked nine-byte SCD4x serial-number frame.
#[napi(js_name = "scd4xSerialNumber")]
pub fn scd4x_serial_number(frame: Buffer) -> napi::Result<i64> {
    let frame: [u8; 9] = frame
        .as_ref()
        .try_into()
        .map_err(|_| length_error("serial number frame", 9))?;
    scd4x::serial_number(&frame)
        .map(|serial| serial as i64)
        .map_err(to_napi)
}

/// Builds the nine bytes an SCD4x sends for a serial number.
#[napi(js_name = "scd4xSerialNumberFrame")]
pub fn scd4x_serial_number_frame(serial: i64) -> Buffer {
    Buffer::from(scd4x::serial_number_frame(serial as u64).to_vec())
}

/// A TMP117 configuration register, field by field.
#[napi(object)]
pub struct Tmp117Configuration {
    /// Whether a result went above the high limit.
    pub high_alert: bool,
    /// Whether a result went below the low limit.
    pub low_alert: bool,
    /// Whether a conversion has completed since the register was last read.
    pub data_ready: bool,
    /// Whether an EEPROM write is still in progress.
    pub eeprom_busy: bool,
    /// The conversion-mode code: `0` continuous, `1` shutdown, `3` one-shot.
    pub mode: u8,
    /// The conversion-cycle code, `0..=7`.
    pub cycle: u8,
    /// The averaging code, `0..=3`.
    pub averaging: u8,
    /// Whether the limits act as a therm hysteresis band rather than as alerts.
    pub therm_mode: bool,
    /// Whether the ALERT pin is active high.
    pub alert_active_high: bool,
    /// Whether the ALERT pin reflects data ready rather than the alert flags.
    pub alert_pin_data_ready: bool,
    /// Whether writing this triggers a software reset.
    pub soft_reset: bool,
}

/// Converts a raw TMP117 temperature register to nano-degrees Celsius.
#[napi]
pub fn tmp117_nano_celsius(raw: i16) -> i64 {
    tmp117::nano_celsius(raw)
}

/// Converts a raw TMP117 temperature register to micro-degrees Celsius.
#[napi]
pub fn tmp117_micro_celsius(raw: i16) -> i32 {
    tmp117::micro_celsius(raw)
}

/// Converts a raw TMP117 temperature register to degrees Celsius.
#[napi]
pub fn tmp117_celsius(raw: i16) -> f64 {
    f64::from(tmp117::celsius(raw))
}

/// Builds the TMP117 temperature register that decodes to a temperature.
#[napi]
pub fn tmp117_raw_from_micro_celsius(micro_celsius: i32) -> i16 {
    tmp117::raw_from_micro_celsius(micro_celsius)
}

/// Builds the TMP117 temperature register that decodes to a temperature in Celsius.
#[napi]
pub fn tmp117_raw_from_celsius(celsius: f64) -> i16 {
    tmp117::raw_from_celsius(celsius as f32)
}

/// Builds the two bytes a TMP117 sends for a temperature register.
#[napi]
pub fn tmp117_temperature_bytes(raw: i16) -> Buffer {
    Buffer::from(tmp117::temperature_bytes(raw).to_vec())
}

/// Reads the two bytes a TMP117 sends for a temperature register.
#[napi]
pub fn tmp117_temperature_from_bytes(bytes: Buffer) -> napi::Result<i16> {
    let bytes: [u8; 2] = bytes
        .as_ref()
        .try_into()
        .map_err(|_| length_error("temperature register", 2))?;
    Ok(tmp117::temperature_from_bytes(bytes))
}

/// Reads the device identifier out of a TMP117 device-ID register.
#[napi]
pub fn tmp117_device_id(raw: u16) -> u16 {
    tmp117::device_id(raw)
}

/// Reads the die revision out of a TMP117 device-ID register.
#[napi]
pub fn tmp117_revision(raw: u16) -> u8 {
    tmp117::revision(raw)
}

/// Reports whether a TMP117 configuration register flags a high alert.
#[napi]
pub fn tmp117_high_alert(config: u16) -> bool {
    tmp117::high_alert(config)
}

/// Reports whether a TMP117 configuration register flags a low alert.
#[napi]
pub fn tmp117_low_alert(config: u16) -> bool {
    tmp117::low_alert(config)
}

/// Reports whether a TMP117 configuration register says a result is ready.
#[napi]
pub fn tmp117_data_ready(config: u16) -> bool {
    tmp117::data_ready(config)
}

/// Reports whether a TMP117 configuration register says an EEPROM write is running.
#[napi]
pub fn tmp117_eeprom_busy(config: u16) -> bool {
    tmp117::eeprom_busy(config)
}

/// Reports whether a TMP117 EEPROM unlock register says a write is running.
#[napi]
pub fn tmp117_eeprom_unlock_busy(unlock: u16) -> bool {
    tmp117::eeprom_unlock_busy(unlock)
}

/// Assembles the 16-bit TMP117 configuration register value.
#[napi]
pub fn tmp117_config_bits(config: Tmp117Configuration) -> u16 {
    tmp117::Configuration::from(config).bits()
}

/// Parses a 16-bit TMP117 configuration register value.
#[napi]
pub fn tmp117_config_from_bits(bits: u16) -> Tmp117Configuration {
    tmp117::Configuration::from_bits(bits).into()
}

/// Returns how many conversions a TMP117 averaging code folds into one result.
#[napi]
pub fn tmp117_averaging_conversions(code: u8) -> u8 {
    tmp117::Averaging::from_code(code).conversions()
}

/// Returns how long a TMP117 averaging code takes to convert, in microseconds.
#[napi]
pub fn tmp117_averaging_micros(code: u8) -> u32 {
    tmp117::Averaging::from_code(code).conversion_micros()
}

/// Returns the nominal cycle a TMP117 conversion-cycle code selects, in microseconds.
#[napi]
pub fn tmp117_cycle_nominal_micros(code: u8) -> u32 {
    tmp117::ConversionCycle::from_code(code).nominal_micros()
}

/// Returns the TMP117 result-update interval for a cycle and averaging code.
#[napi]
pub fn tmp117_cycle_micros(cycle: u8, averaging: u8) -> u32 {
    tmp117::ConversionCycle::from_code(cycle).cycle_micros(tmp117::Averaging::from_code(averaging))
}

/// A decoded HDC1080 temperature and humidity pair.
#[napi(object)]
pub struct Hdc1080Measurement {
    /// The raw temperature register.
    pub temperature_raw: u16,
    /// The raw humidity register.
    pub humidity_raw: u16,
    /// The temperature in milli-degrees Celsius, exact in integer arithmetic.
    pub milli_celsius: i32,
    /// The temperature in degrees Celsius.
    pub celsius: f64,
    /// The relative humidity in milli-percent.
    pub milli_percent: u32,
    /// The relative humidity as a percentage.
    pub relative_humidity: f64,
}

/// An HDC1080 configuration register, field by field.
#[napi(object)]
pub struct Hdc1080Configuration {
    /// Whether writing this resets the part.
    pub software_reset: bool,
    /// Whether the on-die heater runs during measurements.
    pub heater: bool,
    /// Whether one trigger acquires temperature and humidity in sequence.
    pub sequential: bool,
    /// Whether the supply has dropped below 2.8 V, which the part reports back.
    pub battery_low: bool,
    /// The temperature resolution in bits: 14 or 11.
    pub temperature_resolution_bits: u8,
    /// The humidity resolution in bits: 14, 11, or 8.
    pub humidity_resolution_bits: u8,
}

/// Converts a raw HDC1080 temperature register to milli-degrees Celsius.
#[napi]
pub fn hdc1080_milli_celsius(raw: u16) -> i32 {
    hdc1080::milli_celsius(raw)
}

/// Converts a raw HDC1080 temperature register to degrees Celsius.
#[napi]
pub fn hdc1080_celsius(raw: u16) -> f64 {
    f64::from(hdc1080::celsius(raw))
}

/// Converts a raw HDC1080 humidity register to milli-percent.
#[napi]
pub fn hdc1080_milli_percent(raw: u16) -> u32 {
    hdc1080::milli_percent(raw)
}

/// Converts a raw HDC1080 humidity register to a relative humidity percentage.
#[napi]
pub fn hdc1080_relative_humidity(raw: u16) -> f64 {
    f64::from(hdc1080::relative_humidity(raw))
}

/// Builds the HDC1080 temperature register that decodes to a temperature.
#[napi]
pub fn hdc1080_temperature_register(milli_celsius: i32) -> u16 {
    hdc1080::temperature_register(milli_celsius)
}

/// Builds the HDC1080 humidity register that decodes to a relative humidity.
#[napi]
pub fn hdc1080_humidity_register(milli_percent: u32) -> u16 {
    hdc1080::humidity_register(milli_percent)
}

/// Joins the three HDC1080 serial-ID registers into the 40-bit serial number.
#[napi]
pub fn hdc1080_serial_id(high: u16, mid: u16, low: u16) -> i64 {
    hdc1080::serial_id(high, mid, low) as i64
}

/// Splits a serial number back into the three HDC1080 serial-ID registers.
#[napi]
pub fn hdc1080_serial_id_registers(serial: i64) -> Vec<u16> {
    hdc1080::serial_id_registers(serial as u64).to_vec()
}

/// Parses the four bytes an HDC1080 sequential read returns.
#[napi]
pub fn hdc1080_parse_measurement(bytes: Buffer) -> napi::Result<Hdc1080Measurement> {
    let bytes: [u8; 4] = bytes
        .as_ref()
        .try_into()
        .map_err(|_| length_error("measurement", 4))?;
    Ok(hdc1080::Measurement::parse(&bytes).into())
}

/// Builds the HDC1080 measurement a sensor reporting these physical values would send.
#[napi]
pub fn hdc1080_measurement_from_physical(
    milli_celsius: i32,
    milli_percent: u32,
) -> Hdc1080Measurement {
    hdc1080::Measurement::from_physical(milli_celsius, milli_percent).into()
}

/// Builds the four bytes an HDC1080 sends for a pair of raw registers.
#[napi]
pub fn hdc1080_measurement_bytes(temperature_raw: u16, humidity_raw: u16) -> Buffer {
    let measurement = hdc1080::Measurement {
        temperature: temperature_raw,
        humidity: humidity_raw,
    };
    Buffer::from(measurement.to_bytes().to_vec())
}

/// Parses an HDC1080 configuration register value.
#[napi]
pub fn hdc1080_configuration_from_register(raw: u16) -> napi::Result<Hdc1080Configuration> {
    Ok(hdc1080::Configuration::from_register(raw)
        .map_err(to_napi)?
        .into())
}

/// Assembles an HDC1080 configuration register value.
#[napi]
pub fn hdc1080_configuration_to_register(config: Hdc1080Configuration) -> napi::Result<u16> {
    Ok(hdc1080::Configuration::try_from(config)?.to_register())
}

/// Returns how long to wait after triggering an HDC1080 in this configuration.
#[napi]
pub fn hdc1080_conversion_time_micros(config: Hdc1080Configuration) -> napi::Result<u32> {
    Ok(hdc1080::Configuration::try_from(config)?.conversion_time_micros())
}

/// Returns how long an HDC1080 temperature conversion takes, in microseconds.
#[napi]
pub fn hdc1080_temperature_conversion_micros(bits: u8) -> napi::Result<u32> {
    Ok(temperature_resolution(bits)?.conversion_time_micros())
}

/// Returns how long an HDC1080 humidity conversion takes, in microseconds.
#[napi]
pub fn hdc1080_humidity_conversion_micros(bits: u8) -> napi::Result<u32> {
    Ok(humidity_resolution(bits)?.conversion_time_micros())
}

/// An OPT3001 configuration register, field by field.
#[napi(object)]
pub struct Opt3001Configuration {
    /// The full-scale range number, `0..=11`, or `12` to set the range automatically.
    pub range_number: u8,
    /// Whether a conversion takes 800 ms rather than 100 ms.
    pub long_conversion: bool,
    /// The mode code: `0` shutdown, `1` single shot, `2` continuous.
    pub mode: u8,
    /// Whether the last result overflowed its range.
    pub overflow: bool,
    /// Whether a conversion has completed since the register was last read.
    pub conversion_ready: bool,
    /// Whether the result went above the high limit.
    pub flag_high: bool,
    /// Whether the result went below the low limit.
    pub flag_low: bool,
    /// Whether the INT pin latches until the configuration register is read.
    pub latched_window: bool,
    /// Whether the INT pin is active high.
    pub active_high: bool,
    /// Whether the limit registers carry a mantissa alone, without an exponent.
    pub mask_exponent: bool,
    /// The fault-count code, `0..=3`, for one, two, four, or eight faults.
    pub fault_count: u8,
}

/// Returns the illuminance one count carries at an OPT3001 exponent.
#[napi]
pub fn opt3001_lsb_milli_lux(exponent: u8) -> Option<u32> {
    opt3001::lsb_milli_lux(exponent)
}

/// Returns the full scale an OPT3001 range number covers, in milli-lux.
#[napi]
pub fn opt3001_full_scale_milli_lux(range_number: u8) -> Option<u32> {
    opt3001::full_scale_milli_lux(range_number)
}

/// Converts a raw OPT3001 result register to milli-lux.
#[napi]
pub fn opt3001_milli_lux(raw: u16) -> u32 {
    opt3001::milli_lux(raw)
}

/// Converts a raw OPT3001 result register to lux.
#[napi]
pub fn opt3001_lux(raw: u16) -> f64 {
    f64::from(opt3001::lux(raw))
}

/// Builds the OPT3001 result register that decodes to an illuminance.
#[napi]
pub fn opt3001_raw_from_milli_lux(milli_lux: u32) -> u16 {
    opt3001::raw_from_milli_lux(milli_lux)
}

/// Reads the two bytes an OPT3001 sends for a register.
#[napi]
pub fn opt3001_word_from_bytes(bytes: Buffer) -> napi::Result<u16> {
    let bytes: [u8; 2] = bytes
        .as_ref()
        .try_into()
        .map_err(|_| length_error("register", 2))?;
    Ok(opt3001::word_from_bytes(bytes))
}

/// Builds the two bytes an OPT3001 sends for a register.
#[napi]
pub fn opt3001_word_to_bytes(word: u16) -> Buffer {
    Buffer::from(opt3001::word_to_bytes(word).to_vec())
}

/// Assembles the 16-bit OPT3001 configuration register value.
#[napi]
pub fn opt3001_config_bits(config: Opt3001Configuration) -> u16 {
    opt3001::Configuration::from(config).bits()
}

/// Parses a 16-bit OPT3001 configuration register value.
#[napi]
pub fn opt3001_config_from_bits(bits: u16) -> Opt3001Configuration {
    opt3001::Configuration::from_bits(bits).into()
}

/// Returns the conversion time an OPT3001 setting selects, in milliseconds.
#[napi]
pub fn opt3001_conversion_millis(long_conversion: bool) -> u16 {
    conversion_time(long_conversion).millis()
}

/// Returns how many consecutive faults an OPT3001 fault-count code requires.
#[napi]
pub fn opt3001_fault_count(code: u8) -> u8 {
    opt3001::FaultCount::from_code(code).count()
}

/// Reports whether an OPT3001 range number sets the full scale automatically.
#[napi]
pub fn opt3001_is_automatic_range(range_number: u8) -> bool {
    range_number == opt3001::RANGE_AUTOMATIC
}

/// The limit comparison an INA226 alert pin responds to.
#[napi(string_enum)]
pub enum Ina226AlertFunction {
    /// Shunt voltage above the alert limit.
    ShuntOverLimit,
    /// Shunt voltage below the alert limit.
    ShuntUnderLimit,
    /// Bus voltage above the alert limit.
    BusOverLimit,
    /// Bus voltage below the alert limit.
    BusUnderLimit,
    /// Power above the alert limit.
    PowerOverLimit,
}

/// An INA226 configuration register, field by field.
#[napi(object)]
pub struct Ina226Configuration {
    /// Whether writing this resets the part.
    pub reset: bool,
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
#[napi(object)]
pub struct Ina226MaskEnable {
    /// Alert when the shunt voltage exceeds the limit.
    pub shunt_over_limit: bool,
    /// Alert when the shunt voltage drops below the limit.
    pub shunt_under_limit: bool,
    /// Alert when the bus voltage exceeds the limit.
    pub bus_over_limit: bool,
    /// Alert when the bus voltage drops below the limit.
    pub bus_under_limit: bool,
    /// Alert when the power exceeds the limit.
    pub power_over_limit: bool,
    /// Also alert when a conversion completes.
    pub conversion_ready: bool,
    /// Whether the selected limit function caused the last alert.
    pub alert_function_flag: bool,
    /// Whether every conversion and multiplication has completed.
    pub conversion_ready_flag: bool,
    /// Whether an arithmetic overflow left current and power invalid.
    pub math_overflow: bool,
    /// Whether the alert pin is active high.
    pub alert_active_high: bool,
    /// Whether the alert pin latches until this register is read.
    pub alert_latch: bool,
}

/// A decoded INA226 die-ID register.
#[napi(object)]
pub struct Ina226DieId {
    /// The 12-bit device identifier.
    pub device: u16,
    /// The 4-bit die revision.
    pub revision: u8,
}

/// Returns the I2C address an INA226's A1 and A0 pin codes select.
#[napi]
pub fn ina226_address(a1: u8, a0: u8) -> napi::Result<u8> {
    Ok(ina226::address(address_pin(a1)?, address_pin(a0)?))
}

/// Returns how many samples an INA226 averaging code folds into one result.
#[napi]
pub fn ina226_averaging_samples(code: u8) -> u16 {
    averaging(code).samples()
}

/// Returns the conversion time an INA226 code selects, in microseconds.
#[napi]
pub fn ina226_conversion_micros(code: u8) -> u32 {
    conversion_micros(code).microseconds()
}

/// Reports whether an INA226 mode code converts the shunt voltage.
#[napi]
pub fn ina226_measures_shunt(code: u8) -> bool {
    mode(code).measures_shunt()
}

/// Reports whether an INA226 mode code converts the bus voltage.
#[napi]
pub fn ina226_measures_bus(code: u8) -> bool {
    mode(code).measures_bus()
}

/// Reports whether an INA226 mode code keeps converting after the first result.
#[napi]
pub fn ina226_is_continuous(code: u8) -> bool {
    mode(code).is_continuous()
}

/// Parses an INA226 configuration register value.
#[napi]
pub fn ina226_config_from_register(raw: u16) -> Ina226Configuration {
    ina226::Configuration::from_register(raw).into()
}

/// Assembles an INA226 configuration register value.
#[napi]
pub fn ina226_config_to_register(config: Ina226Configuration) -> u16 {
    ina226::Configuration::from(config).to_register()
}

/// Returns how often an INA226 in this configuration updates its results.
#[napi]
pub fn ina226_update_micros(config: Ina226Configuration) -> u32 {
    ina226::Configuration::from(config).update_microseconds()
}

/// Parses an INA226 Mask/Enable register value.
#[napi]
pub fn ina226_mask_enable_from_register(raw: u16) -> Ina226MaskEnable {
    ina226::MaskEnable::from_register(raw).into()
}

/// Assembles an INA226 Mask/Enable register value.
#[napi]
pub fn ina226_mask_enable_to_register(mask: Ina226MaskEnable) -> u16 {
    ina226::MaskEnable::from(mask).to_register()
}

/// Returns the alert function an INA226 pin actually responds to.
#[napi]
pub fn ina226_active_alert_function(mask: Ina226MaskEnable) -> Option<Ina226AlertFunction> {
    ina226::MaskEnable::from(mask)
        .active_alert_function()
        .map(Ina226AlertFunction::from)
}

/// Splits an INA226 die-ID register into its device and revision fields.
#[napi]
pub fn ina226_die_id(raw: u16) -> Ina226DieId {
    ina226::DieId::from_register(raw).into()
}

/// Checks that a pair of identification registers belongs to an INA226.
#[napi]
pub fn ina226_identify(manufacturer_id: u16, die_id: u16) -> napi::Result<Ina226DieId> {
    Ok(ina226::identify(manufacturer_id, die_id)
        .map_err(to_napi)?
        .into())
}

/// Computes the INA226 calibration register for a shunt and current resolution.
#[napi]
pub fn ina226_calibration(current_lsb_microamps: u32, shunt_milliohms: u32) -> u16 {
    ina226::calibration(current_lsb_microamps, shunt_milliohms)
}

/// Returns the smallest current resolution that still covers an expected maximum.
#[napi]
pub fn ina226_minimum_current_lsb_microamps(max_expected_microamps: u32) -> u32 {
    ina226::minimum_current_lsb_microamps(max_expected_microamps)
}

/// Converts a raw INA226 shunt-voltage register to nanovolts.
#[napi]
pub fn ina226_shunt_nanovolts(raw: i16) -> i32 {
    ina226::shunt_nanovolts(raw)
}

/// Converts a raw INA226 shunt-voltage register to millivolts.
#[napi]
pub fn ina226_shunt_millivolts(raw: i16) -> f64 {
    f64::from(ina226::shunt_millivolts_f32(raw))
}

/// Converts a raw INA226 bus-voltage register to microvolts.
#[napi]
pub fn ina226_bus_microvolts(raw: u16) -> u32 {
    ina226::bus_microvolts(raw)
}

/// Converts a raw INA226 bus-voltage register to volts.
#[napi]
pub fn ina226_bus_volts(raw: u16) -> f64 {
    f64::from(ina226::bus_volts_f32(raw))
}

/// Converts a raw INA226 current register to microamps.
#[napi]
pub fn ina226_current_microamps(raw: i16, current_lsb_microamps: u32) -> i32 {
    ina226::current_microamps(raw, current_lsb_microamps)
}

/// Converts a raw INA226 current register to amps.
#[napi]
pub fn ina226_current_amps(raw: i16, current_lsb_microamps: u32) -> f64 {
    f64::from(ina226::current_amps_f32(raw, current_lsb_microamps))
}

/// Converts a raw INA226 power register to microwatts.
#[napi]
pub fn ina226_power_microwatts(raw: u16, current_lsb_microamps: u32) -> u32 {
    ina226::power_microwatts(raw, current_lsb_microamps)
}

/// Converts a raw INA226 power register to watts.
#[napi]
pub fn ina226_power_watts(raw: u16, current_lsb_microamps: u32) -> f64 {
    f64::from(ina226::power_watts_f32(raw, current_lsb_microamps))
}

/// Builds the INA226 shunt-voltage register a monitor reports for a shunt voltage.
#[napi]
pub fn ina226_shunt_register(nanovolts: i32) -> i16 {
    ina226::shunt_register(nanovolts)
}

/// Builds the INA226 bus-voltage register a monitor reports for a bus voltage.
#[napi]
pub fn ina226_bus_register(microvolts: u32) -> u16 {
    ina226::bus_register(microvolts)
}

/// Builds the INA226 current register a monitor reports for a current.
#[napi]
pub fn ina226_current_register(microamps: i32, current_lsb_microamps: u32) -> i16 {
    ina226::current_register(microamps, current_lsb_microamps)
}

/// Builds the INA226 power register a monitor reports for a power.
#[napi]
pub fn ina226_power_register(microwatts: u32, current_lsb_microamps: u32) -> u16 {
    ina226::power_register(microwatts, current_lsb_microamps)
}

/// Computes the INA226 current register the chip derives from a shunt reading.
#[napi]
pub fn ina226_current_register_from_shunt(shunt: i16, calibration: u16) -> i16 {
    ina226::current_register_from_shunt(shunt, calibration)
}

/// Computes the INA226 power register the chip derives from a current reading.
#[napi]
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
            celsius: f64::from(value.celsius()),
            pascals: value.pascals(),
            hectopascals: f64::from(value.hectopascals()),
        }
    }
}

impl From<bmp280::Measurement> for Bmp280Measurement {
    fn from(value: bmp280::Measurement) -> Self {
        Bmp280Measurement {
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
            spi_3wire: value.spi3wire,
        }
    }
}

impl From<bmp280::Config> for Bmp280Config {
    fn from(value: bmp280::Config) -> Self {
        Bmp280Config {
            standby: value.standby.code(),
            filter: value.filter,
            spi3wire: value.spi_3wire,
        }
    }
}

impl From<Sht3xRepeatability> for sht3x::Repeatability {
    fn from(value: Sht3xRepeatability) -> Self {
        match value {
            Sht3xRepeatability::Low => sht3x::Repeatability::Low,
            Sht3xRepeatability::Medium => sht3x::Repeatability::Medium,
            Sht3xRepeatability::High => sht3x::Repeatability::High,
        }
    }
}

impl From<Sht3xRate> for sht3x::Rate {
    fn from(value: Sht3xRate) -> Self {
        match value {
            Sht3xRate::HalfMps => sht3x::Rate::HalfMps,
            Sht3xRate::OneMps => sht3x::Rate::OneMps,
            Sht3xRate::TwoMps => sht3x::Rate::TwoMps,
            Sht3xRate::FourMps => sht3x::Rate::FourMps,
            Sht3xRate::TenMps => sht3x::Rate::TenMps,
        }
    }
}

impl From<sht3x::Measurement> for Sht3xMeasurement {
    fn from(value: sht3x::Measurement) -> Self {
        Sht3xMeasurement {
            temperature_raw: value.temperature_raw,
            humidity_raw: value.humidity_raw,
            milli_celsius: value.temperature_milli_celsius(),
            celsius: f64::from(value.temperature_celsius()),
            milli_fahrenheit: value.temperature_milli_fahrenheit(),
            fahrenheit: f64::from(value.temperature_fahrenheit()),
            milli_percent: value.humidity_milli_percent(),
            relative_humidity: f64::from(value.relative_humidity()),
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
            celsius: f64::from(value.celsius()),
            humidity_milli_percent: value.humidity_milli_percent(),
            relative_humidity_percent: f64::from(value.relative_humidity_percent()),
        }
    }
}

impl From<Tmp117Configuration> for tmp117::Configuration {
    fn from(value: Tmp117Configuration) -> Self {
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

impl From<tmp117::Configuration> for Tmp117Configuration {
    fn from(value: tmp117::Configuration) -> Self {
        Tmp117Configuration {
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
            celsius: f64::from(value.celsius()),
            milli_percent: value.milli_percent(),
            relative_humidity: f64::from(value.relative_humidity()),
        }
    }
}

impl From<hdc1080::Configuration> for Hdc1080Configuration {
    fn from(value: hdc1080::Configuration) -> Self {
        Hdc1080Configuration {
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

impl TryFrom<Hdc1080Configuration> for hdc1080::Configuration {
    type Error = napi::Error;

    fn try_from(value: Hdc1080Configuration) -> napi::Result<Self> {
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

impl From<Opt3001Configuration> for opt3001::Configuration {
    fn from(value: Opt3001Configuration) -> Self {
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

impl From<opt3001::Configuration> for Opt3001Configuration {
    fn from(value: opt3001::Configuration) -> Self {
        Opt3001Configuration {
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

impl From<Ina226Configuration> for ina226::Configuration {
    fn from(value: Ina226Configuration) -> Self {
        ina226::Configuration {
            reset: value.reset,
            averaging: averaging(value.averaging),
            bus_conversion_time: conversion_micros(value.bus_conversion_time),
            shunt_conversion_time: conversion_micros(value.shunt_conversion_time),
            mode: mode(value.mode),
        }
    }
}

impl From<ina226::Configuration> for Ina226Configuration {
    fn from(value: ina226::Configuration) -> Self {
        Ina226Configuration {
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

impl From<ina226::AlertFunction> for Ina226AlertFunction {
    fn from(value: ina226::AlertFunction) -> Self {
        match value {
            ina226::AlertFunction::ShuntOverLimit => Ina226AlertFunction::ShuntOverLimit,
            ina226::AlertFunction::ShuntUnderLimit => Ina226AlertFunction::ShuntUnderLimit,
            ina226::AlertFunction::BusOverLimit => Ina226AlertFunction::BusOverLimit,
            ina226::AlertFunction::BusUnderLimit => Ina226AlertFunction::BusUnderLimit,
            ina226::AlertFunction::PowerOverLimit => Ina226AlertFunction::PowerOverLimit,
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

/// Maps a bit count onto the HDC1080 temperature resolution it names.
fn temperature_resolution(bits: u8) -> napi::Result<hdc1080::TemperatureResolution> {
    match bits {
        14 => Ok(hdc1080::TemperatureResolution::Bits14),
        11 => Ok(hdc1080::TemperatureResolution::Bits11),
        _ => Err(napi::Error::from_reason(
            "HDC1080 temperature resolution must be 14 or 11 bits",
        )),
    }
}

/// Maps a bit count onto the HDC1080 humidity resolution it names.
fn humidity_resolution(bits: u8) -> napi::Result<hdc1080::HumidityResolution> {
    match bits {
        14 => Ok(hdc1080::HumidityResolution::Bits14),
        11 => Ok(hdc1080::HumidityResolution::Bits11),
        8 => Ok(hdc1080::HumidityResolution::Bits8),
        _ => Err(napi::Error::from_reason(
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
fn address_pin(code: u8) -> napi::Result<ina226::AddressPin> {
    match code {
        0 => Ok(ina226::AddressPin::Ground),
        1 => Ok(ina226::AddressPin::Supply),
        2 => Ok(ina226::AddressPin::Sda),
        3 => Ok(ina226::AddressPin::Scl),
        _ => Err(napi::Error::from_reason(
            "an INA226 address pin must be tied to GND, VS, SDA, or SCL: code 0 to 3",
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
