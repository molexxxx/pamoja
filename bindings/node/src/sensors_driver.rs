//! Generated Node bindings for the sensor drivers that run over an I2C bus.
//!
//! A driver runs the datasheet's whole conversation with a part over an `I2cBus`: reset,
//! identify, read the calibration, configure, measure. It holds its own share of the bus, and
//! each call runs on a worker thread and resolves when the part has answered, since a
//! measurement waits the datasheet's time before it reads. Every part has a simulated twin
//! that answers a driver with nothing plugged in.
//!
//! The DS18B20 is read the way a Linux process reaches one, through the files the kernel's
//! 1-Wire driver serves, which a program can also write for itself to test against.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use napi::bindgen_prelude::{spawn_blocking, Buffer};
use napi_derive::napi;
use pamoja_hal::bus::{BusDelay, BusError, I2cBus as Bus};
use pamoja_sensors::driver::I2cRegisters;
use pamoja_sensors::{
    ads1115, bme280, bmp280, ds18b20, hdc1080, ina219, ina226, opt3001, scd4x, sht3x, tmp117,
    DriverError,
};

use crate::hal::{CommandPart, I2cBus, I2cPart, WordPart};
use crate::sensors::{
    Ads1115Config, Bme280Measurement, Bmp280Coefficients, Bmp280Reading, Ds18b20Reading,
    Hdc1080Configuration, Hdc1080Measurement, Ina219Configuration, Ina226Configuration,
    Ina226DieId, Ina226MaskEnable, Opt3001Configuration, Scd4xMeasurement, Sht3xMeasurement,
    Sht3xRepeatability, Sht3xStatus,
};

type Bme280Driver = bme280::Bme280<I2cRegisters<Bus>, BusDelay>;
type Bmp280Driver = bmp280::Bmp280<I2cRegisters<Bus>, BusDelay>;
type Tmp117Driver = tmp117::Tmp117<Bus, BusDelay>;
type Opt3001Driver = opt3001::Opt3001<Bus, BusDelay>;
type Hdc1080Driver = hdc1080::Hdc1080<Bus, BusDelay>;
type Ina219Driver = ina219::Ina219<Bus, BusDelay>;
type Ina226Driver = ina226::Ina226<Bus, BusDelay>;
type Ads1115Driver = ads1115::Ads1115<Bus, BusDelay>;
type Sht3xDriver = sht3x::Sht3x<Bus, BusDelay>;
type Scd4xDriver = scd4x::Scd4x<Bus, BusDelay>;

/// How a BME280 driver measures. A field left out keeps the default: every measurement at
/// oversampling x1 and the filter off.
#[napi(object)]
pub struct Bme280Settings {
    /// The temperature oversampling code, `0..=5`, where `0` skips the measurement.
    pub temperature: Option<u8>,
    /// The pressure oversampling code, `0..=5`, where `0` skips the measurement.
    pub pressure: Option<u8>,
    /// The humidity oversampling code, `0..=5`, where `0` skips the measurement.
    pub humidity: Option<u8>,
    /// The IIR filter code, `0..=4`, where `0` is off.
    pub filter: Option<u8>,
}

/// How a BMP280 driver measures. A field left out keeps the default: both measurements at
/// oversampling x1 and the filter off.
#[napi(object)]
pub struct Bmp280Settings {
    /// The temperature oversampling code, `0..=5`, where `0` skips the measurement.
    pub temperature: Option<u8>,
    /// The pressure oversampling code, `0..=5`, where `0` skips the measurement.
    pub pressure: Option<u8>,
    /// The IIR filter's three-bit `filter[2:0]` code, `0` for the filter off, written as given.
    pub filter: Option<u8>,
}

/// How a TMP117 driver measures. A field left out keeps the default.
#[napi(object)]
pub struct Tmp117Settings {
    /// The averaging code, `0..=3`, for 1, 8, 32, or 64 conversions per result; `1`, eight,
    /// is the factory setting and the default.
    pub averaging: Option<u8>,
}

/// A TMP117 temperature result.
#[napi(object)]
pub struct Tmp117Reading {
    /// The temperature register, 7.8125 millidegrees Celsius per count.
    pub raw: i16,
    /// The temperature in micro-degrees Celsius, exact in integer arithmetic.
    pub micro_celsius: i32,
    /// The temperature in degrees Celsius.
    pub celsius: f64,
}

/// The TMP117's alert flags: whether a result since they were last read crossed a limit.
#[napi(object)]
pub struct Tmp117Alerts {
    /// A result was above the high limit.
    pub high: bool,
    /// A result was below the low limit.
    pub low: bool,
}

/// How an OPT3001 driver measures. A field left out keeps the part's reset setting.
#[napi(object)]
pub struct Opt3001Settings {
    /// Whether each conversion integrates for 800 ms, for resolution, rather than 100 ms, for
    /// speed; 800 ms unless given.
    pub long_conversion: Option<bool>,
    /// The full-scale range number, `0..=11`, or `12` to let the part choose, which it does
    /// unless given.
    pub range_number: Option<u8>,
}

/// An OPT3001 illuminance result.
#[napi(object)]
pub struct Opt3001Reading {
    /// The result register: a four-bit exponent over a twelve-bit mantissa.
    pub raw: u16,
    /// The illuminance in millilux, exact in integer arithmetic.
    pub milli_lux: u32,
    /// The illuminance in lux.
    pub lux: f64,
}

/// How an HDC1080 driver measures. A field left out keeps 14 bits.
#[napi(object)]
pub struct Hdc1080Settings {
    /// The temperature resolution in bits: 14 or 11.
    pub temperature_resolution_bits: Option<u8>,
    /// The humidity resolution in bits: 14, 11, or 8.
    pub humidity_resolution_bits: Option<u8>,
}

/// How an INA219 driver measures. A field left out keeps the default: a 100 milliohm shunt
/// sized for 3.2 A, the common breakout, and the power-on register settings.
#[napi(object)]
pub struct Ina219Settings {
    /// The shunt resistance in milliohms.
    pub shunt_milliohms: Option<u32>,
    /// The largest current the shunt will carry, in microamps, which sets the finest current
    /// step the calibration allows.
    pub max_microamps: Option<u32>,
    /// A current step to use instead, in microamps per count, such as a round 100.
    pub current_lsb_microamps: Option<u32>,
    /// The range, gain, and converter settings; the mode is chosen per conversion.
    pub configuration: Option<Ina219Configuration>,
}

/// One INA219 conversion: the four result registers as read, and what they mean.
#[napi(object)]
pub struct Ina219Reading {
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
    /// Whether the part's arithmetic overflowed, leaving current and power meaningless.
    pub math_overflow: bool,
}

/// How an INA226 driver measures. A field left out keeps the default: a 100 milliohm shunt
/// sized for 3.2 A and the power-on register settings.
#[napi(object)]
pub struct Ina226Settings {
    /// The shunt resistance in milliohms.
    pub shunt_milliohms: Option<u32>,
    /// The largest current the shunt will carry, in microamps, which sets the finest current
    /// step the calibration allows.
    pub max_microamps: Option<u32>,
    /// A current step to use instead, in microamps per count.
    pub current_lsb_microamps: Option<u32>,
    /// The averaging and conversion times; the mode is chosen per conversion.
    pub configuration: Option<Ina226Configuration>,
}

/// One INA226 conversion: the four result registers as read, and what they mean.
#[napi(object)]
pub struct Ina226Reading {
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
    pub shunt_millivolts: f64,
    /// The bus voltage in microvolts.
    pub bus_microvolts: u32,
    /// The bus voltage in volts.
    pub bus_volts: f64,
    /// The current in microamps; negative flows the other way through the shunt.
    pub current_microamps: i32,
    /// The current in amps.
    pub current_amps: f64,
    /// The power in microwatts.
    pub power_microwatts: u32,
    /// The power in watts.
    pub power_watts: f64,
    /// Whether the part's arithmetic overflowed, leaving current and power meaningless.
    pub math_overflow: bool,
}

/// How an ADS1115 driver converts. A field left out keeps the part's reset setting: AIN0
/// against AIN1, the 2.048 V range, and 128 samples per second.
#[napi(object)]
pub struct Ads1115Settings {
    /// The input multiplexer code, `0..=7`: `0..=3` a differential pair, `4..=7` one input
    /// against ground.
    pub mux: Option<u8>,
    /// The gain code, `0..=7`, which sets the full-scale range.
    pub pga: Option<u8>,
    /// The data-rate code, `0..=7`, from 8 to 860 samples per second.
    pub data_rate: Option<u8>,
}

/// One ADS1115 conversion.
#[napi(object)]
pub struct Ads1115Sample {
    /// The conversion register, two's complement.
    pub raw: i16,
    /// The gain code the conversion ran at.
    pub pga: u8,
    /// The voltage in nanovolts, exact in integer arithmetic.
    pub nanovolts: i64,
    /// The voltage in volts.
    pub volts: f64,
}

/// How an SHT3x driver measures. A field left out keeps the default, high repeatability.
#[napi(object, js_name = "Sht3xSettings")]
pub struct Sht3xSettings {
    /// How repeatable each measurement is, against how long it takes.
    pub repeatability: Option<Sht3xRepeatability>,
}

/// A Bosch BME280 driven over an I2C bus, measuring on demand in forced mode.
#[napi]
pub struct Bme280 {
    inner: Arc<Mutex<Bme280Driver>>,
}

#[napi]
impl Bme280 {
    /// A driver for the part at `address` on `bus`. Nothing is sent until `init` or the first
    /// `measure`.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, address: u8, settings: Option<Bme280Settings>) -> Self {
        let x1 = bme280::Oversampling::X1.code();
        let off = bme280::Filter::Off.code();
        let (temperature, pressure, humidity, filter) = match settings {
            Some(settings) => (
                settings.temperature.unwrap_or(x1),
                settings.pressure.unwrap_or(x1),
                settings.humidity.unwrap_or(x1),
                settings.filter.unwrap_or(off),
            ),
            None => (x1, x1, x1, off),
        };
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = bme280::Bme280::i2c(bus, address, delay)
            .with_oversampling(
                bme280::Oversampling::from_code(temperature),
                bme280::Oversampling::from_code(pressure),
                bme280::Oversampling::from_code(humidity),
            )
            .with_filter(bme280::Filter::from_code(filter));
        Bme280 {
            inner: shared(driver),
        }
    }

    /// Resets the part, checks it is a BME280, reads its calibration, and writes the settings,
    /// leaving the part asleep. Rejects when nothing answers or another part does.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Runs one forced measurement and resolves with the compensated reading, initializing the
    /// part first if `init` has not run.
    #[napi]
    pub async fn measure(&self) -> napi::Result<Bme280Measurement> {
        let reading = drive(&self.inner, |driver| driver.measure()).await?;
        Ok(Bme280Measurement {
            celsius: f64::from(reading.celsius()),
            pascals: reading.pascals(),
            hectopascals: f64::from(reading.hectopascals()),
            relative_humidity_percent: f64::from(reading.relative_humidity_percent()),
        })
    }
}

/// A Bosch BMP280 driven over an I2C bus, measuring on demand in forced mode.
#[napi]
pub struct Bmp280 {
    inner: Arc<Mutex<Bmp280Driver>>,
}

#[napi]
impl Bmp280 {
    /// A driver for the part at `address` on `bus`. Nothing is sent until `init` or the first
    /// `measure`.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, address: u8, settings: Option<Bmp280Settings>) -> Self {
        let x1 = bmp280::Oversampling::X1.code();
        let (temperature, pressure, filter) = match settings {
            Some(settings) => (
                settings.temperature.unwrap_or(x1),
                settings.pressure.unwrap_or(x1),
                settings.filter.unwrap_or(0),
            ),
            None => (x1, x1, 0),
        };
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = bmp280::Bmp280::i2c(bus, address, delay)
            .with_oversampling(
                bmp280::Oversampling::from_code(temperature),
                bmp280::Oversampling::from_code(pressure),
            )
            .with_filter(filter);
        Bmp280 {
            inner: shared(driver),
        }
    }

    /// Resets the part, checks it is a BMP280, reads its trimming, and writes the settings,
    /// leaving the part asleep. Rejects when nothing answers or another part does.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Runs one forced measurement and resolves with the compensated reading, initializing the
    /// part first if `init` has not run.
    #[napi]
    pub async fn measure(&self) -> napi::Result<Bmp280Reading> {
        let reading = drive(&self.inner, |driver| driver.measure()).await?;
        Ok(reading.into())
    }

    /// The trimming coefficients read at initialization, or `null` before it.
    #[napi(getter)]
    pub fn coefficients(&self) -> Option<Bmp280Coefficients> {
        held(&self.inner, |driver| {
            driver
                .calibration()
                .map(|calibration| (*calibration).into())
        })
    }
}

/// A Texas Instruments TMP117 driven over an I2C bus, converting on demand in one-shot mode.
#[napi]
pub struct Tmp117 {
    inner: Arc<Mutex<Tmp117Driver>>,
}

#[napi]
impl Tmp117 {
    /// A driver for the part at `address` on `bus`. Nothing is sent until `init` or the first
    /// `measure`.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, address: u8, settings: Option<Tmp117Settings>) -> Self {
        let averaging = settings
            .and_then(|settings| settings.averaging)
            .unwrap_or(tmp117::Averaging::X8.code());
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = tmp117::Tmp117::new(bus, address, delay)
            .with_averaging(tmp117::Averaging::from_code(averaging));
        Tmp117 {
            inner: shared(driver),
        }
    }

    /// Checks the part is a TMP117, waits for its EEPROM to finish loading, and writes the
    /// settings with the part in shutdown. Rejects when nothing answers or another part does.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Runs one conversion and resolves with the temperature, initializing the part first if
    /// `init` has not run.
    #[napi]
    pub async fn measure(&self) -> napi::Result<Tmp117Reading> {
        let reading = drive(&self.inner, |driver| driver.measure()).await?;
        Ok(Tmp117Reading {
            raw: reading.raw(),
            micro_celsius: reading.micro_celsius(),
            celsius: f64::from(reading.celsius()),
        })
    }

    /// Writes the high and low limits the part compares each result against; the factory
    /// limits are 192 C and -256 C.
    #[napi(js_name = "setAlertLimits")]
    pub async fn set_alert_limits(&self, high_celsius: f64, low_celsius: f64) -> napi::Result<()> {
        drive(&self.inner, move |driver| {
            driver.set_alert_limits(high_celsius as f32, low_celsius as f32)
        })
        .await
    }

    /// Resolves with the alert flags: whether a result since the last call was above the high
    /// limit or below the low limit, including results the driver's own reads saw.
    #[napi]
    pub async fn alerts(&self) -> napi::Result<Tmp117Alerts> {
        let alerts = drive(&self.inner, |driver| driver.alerts()).await?;
        Ok(Tmp117Alerts {
            high: alerts.high,
            low: alerts.low,
        })
    }

    /// The silicon revision read at initialization, or `null` before it.
    #[napi(getter, js_name = "siliconRevision")]
    pub fn silicon_revision(&self) -> Option<u8> {
        held(&self.inner, |driver| driver.revision())
    }
}

/// A Texas Instruments OPT3001 driven over an I2C bus, measuring on demand in single-shot mode.
#[napi]
pub struct Opt3001 {
    inner: Arc<Mutex<Opt3001Driver>>,
}

#[napi]
impl Opt3001 {
    /// A driver for the part at `address` on `bus`. Nothing is sent until `init` or the first
    /// `measure`.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, address: u8, settings: Option<Opt3001Settings>) -> Self {
        let (long_conversion, range) = match settings {
            Some(settings) => (
                settings.long_conversion.unwrap_or(true),
                settings.range_number.unwrap_or(opt3001::RANGE_AUTOMATIC),
            ),
            None => (true, opt3001::RANGE_AUTOMATIC),
        };
        let conversion_time = if long_conversion {
            opt3001::ConversionTime::Ms800
        } else {
            opt3001::ConversionTime::Ms100
        };
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = opt3001::Opt3001::new(bus, address, delay)
            .with_conversion_time(conversion_time)
            .with_range(range);
        Opt3001 {
            inner: shared(driver),
        }
    }

    /// Checks the part is an OPT3001 and writes the settings with the part in shutdown.
    /// Rejects when nothing answers or another part does.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Runs one conversion and resolves with the illuminance, initializing the part first if
    /// `init` has not run.
    #[napi]
    pub async fn measure(&self) -> napi::Result<Opt3001Reading> {
        let reading = drive(&self.inner, |driver| driver.measure()).await?;
        Ok(Opt3001Reading {
            raw: reading.raw(),
            milli_lux: reading.milli_lux(),
            lux: f64::from(reading.lux()),
        })
    }

    /// Writes the low and high limits the part's interrupt pin compares each result against,
    /// in millilux.
    #[napi(js_name = "setLimits")]
    pub async fn set_limits(&self, low_milli_lux: u32, high_milli_lux: u32) -> napi::Result<()> {
        drive(&self.inner, move |driver| {
            driver.set_limits(low_milli_lux, high_milli_lux)
        })
        .await
    }

    /// The configuration the driver writes, with the part in shutdown.
    #[napi(getter)]
    pub fn configuration(&self) -> Opt3001Configuration {
        held(&self.inner, |driver| driver.configuration().into())
    }
}

/// A Texas Instruments HDC1080 driven over an I2C bus, measuring temperature then humidity from
/// one trigger.
#[napi]
pub struct Hdc1080 {
    inner: Arc<Mutex<Hdc1080Driver>>,
}

#[napi]
impl Hdc1080 {
    /// A driver for the part on `bus`, which has one address. Nothing is sent until `init` or
    /// the first `measure`. Throws for a resolution the part does not have.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, settings: Option<Hdc1080Settings>) -> napi::Result<Self> {
        let (temperature_bits, humidity_bits) = match settings {
            Some(settings) => (
                settings.temperature_resolution_bits.unwrap_or(14),
                settings.humidity_resolution_bits.unwrap_or(14),
            ),
            None => (14, 14),
        };
        let temperature = match temperature_bits {
            14 => hdc1080::TemperatureResolution::Bits14,
            11 => hdc1080::TemperatureResolution::Bits11,
            _ => {
                return Err(napi::Error::from_reason(
                    "HDC1080 temperature resolution must be 14 or 11 bits",
                ))
            }
        };
        let humidity = match humidity_bits {
            14 => hdc1080::HumidityResolution::Bits14,
            11 => hdc1080::HumidityResolution::Bits11,
            8 => hdc1080::HumidityResolution::Bits8,
            _ => {
                return Err(napi::Error::from_reason(
                    "HDC1080 humidity resolution must be 14, 11, or 8 bits",
                ))
            }
        };
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = hdc1080::Hdc1080::new(bus, delay).with_resolutions(temperature, humidity);
        Ok(Hdc1080 {
            inner: shared(driver),
        })
    }

    /// Checks the part is an HDC1080 and writes the configuration. Rejects when nothing
    /// answers or another part does.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Triggers one acquisition of both channels and resolves with them, initializing the part
    /// first if `init` has not run. Rejects when the part does not acknowledge the read, which
    /// it refuses until its results are ready.
    #[napi]
    pub async fn measure(&self) -> napi::Result<Hdc1080Measurement> {
        let measurement = drive(&self.inner, |driver| driver.measure()).await?;
        Ok(measurement.into())
    }

    /// Switches the on-die heater, which runs only during acquisitions, on or off.
    #[napi]
    pub async fn heater(&self, on: bool) -> napi::Result<()> {
        drive(&self.inner, move |driver| driver.heater(on)).await
    }

    /// The configuration the driver writes.
    #[napi(getter)]
    pub fn configuration(&self) -> Hdc1080Configuration {
        held(&self.inner, |driver| driver.configuration().into())
    }
}

/// A Texas Instruments INA219 driven over an I2C bus, measuring shunt and bus voltage, current,
/// and power on demand.
#[napi]
pub struct Ina219 {
    inner: Arc<Mutex<Ina219Driver>>,
}

#[napi]
impl Ina219 {
    /// A driver for the part at `address` on `bus`. Nothing is sent until `init` or the first
    /// `measure`.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, address: u8, settings: Option<Ina219Settings>) -> Self {
        let settings = settings.unwrap_or(Ina219Settings {
            shunt_milliohms: None,
            max_microamps: None,
            current_lsb_microamps: None,
            configuration: None,
        });
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let mut driver = ina219::Ina219::new(bus, address, delay)
            .with_shunt(
                settings.shunt_milliohms.unwrap_or(100),
                settings.max_microamps.unwrap_or(3_200_000),
            )
            .with_configuration(
                settings
                    .configuration
                    .map_or_else(ina219::Configuration::default, Into::into),
            );
        if let Some(lsb) = settings.current_lsb_microamps {
            driver = driver.with_current_lsb(lsb);
        }
        Ina219 {
            inner: shared(driver),
        }
    }

    /// Resets the part, writes the configuration and the calibration, and reads the
    /// calibration back. Rejects when nothing answers or the calibration does not hold.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Triggers one shunt and bus conversion and resolves with every result, initializing the
    /// part first if `init` has not run.
    #[napi]
    pub async fn measure(&self) -> napi::Result<Ina219Reading> {
        let reading = drive(&self.inner, |driver| driver.measure()).await?;
        Ok(Ina219Reading {
            shunt: reading.shunt,
            bus: reading.bus,
            current: reading.current,
            power: reading.power,
            current_lsb_microamps: reading.current_lsb_microamps,
            shunt_microvolts: reading.shunt_microvolts(),
            bus_millivolts: reading.bus_millivolts(),
            current_microamps: reading.current_microamps(),
            power_microwatts: reading.power_microwatts(),
            math_overflow: reading.math_overflow(),
        })
    }

    /// The current step the driver programs, in microamps per count.
    #[napi(getter, js_name = "currentLsbMicroamps")]
    pub fn current_lsb_microamps(&self) -> u32 {
        held(&self.inner, |driver| driver.current_lsb_microamps())
    }

    /// The calibration word the driver programs.
    #[napi(getter, js_name = "calibrationWord")]
    pub fn calibration_word(&self) -> u16 {
        held(&self.inner, |driver| driver.calibration())
    }
}

/// A Texas Instruments INA226 driven over an I2C bus, measuring shunt and bus voltage, current,
/// and power on demand.
#[napi]
pub struct Ina226 {
    inner: Arc<Mutex<Ina226Driver>>,
}

#[napi]
impl Ina226 {
    /// A driver for the part at `address` on `bus`. Nothing is sent until `init` or the first
    /// `measure`.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, address: u8, settings: Option<Ina226Settings>) -> Self {
        let settings = settings.unwrap_or(Ina226Settings {
            shunt_milliohms: None,
            max_microamps: None,
            current_lsb_microamps: None,
            configuration: None,
        });
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let mut driver = ina226::Ina226::new(bus, address, delay)
            .with_shunt(
                settings.shunt_milliohms.unwrap_or(100),
                settings.max_microamps.unwrap_or(3_200_000),
            )
            .with_configuration(
                settings
                    .configuration
                    .map_or(ina226::Configuration::RESET, Into::into),
            );
        if let Some(lsb) = settings.current_lsb_microamps {
            driver = driver.with_current_lsb(lsb);
        }
        Ina226 {
            inner: shared(driver),
        }
    }

    /// Resets the part, checks it is an INA226, writes the configuration and the calibration,
    /// and reads the calibration back. Rejects when nothing answers or another part does.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Triggers one shunt and bus conversion and resolves with every result, initializing the
    /// part first if `init` has not run.
    #[napi]
    pub async fn measure(&self) -> napi::Result<Ina226Reading> {
        let reading = drive(&self.inner, |driver| driver.measure()).await?;
        Ok(Ina226Reading {
            shunt: reading.shunt,
            bus: reading.bus,
            current: reading.current,
            power: reading.power,
            current_lsb_microamps: reading.current_lsb_microamps,
            shunt_nanovolts: reading.shunt_nanovolts(),
            shunt_millivolts: f64::from(reading.shunt_millivolts()),
            bus_microvolts: reading.bus_microvolts(),
            bus_volts: f64::from(reading.bus_volts()),
            current_microamps: reading.current_microamps(),
            current_amps: f64::from(reading.current_amps()),
            power_microwatts: reading.power_microwatts(),
            power_watts: f64::from(reading.power_watts()),
            math_overflow: reading.math_overflow,
        })
    }

    /// Programs the alert pin: which limit it watches, one function at a time, and the limit,
    /// in the units of the register the function watches.
    #[napi(js_name = "setAlert")]
    pub async fn set_alert(&self, mask: Ina226MaskEnable, limit: u16) -> napi::Result<()> {
        let mask = ina226::MaskEnable::from(mask);
        drive(&self.inner, move |driver| driver.set_alert(mask, limit)).await
    }

    /// The current step the driver programs, in microamps per count.
    #[napi(getter, js_name = "currentLsbMicroamps")]
    pub fn current_lsb_microamps(&self) -> u32 {
        held(&self.inner, |driver| driver.current_lsb_microamps())
    }

    /// The calibration word the driver programs.
    #[napi(getter, js_name = "calibrationWord")]
    pub fn calibration_word(&self) -> u16 {
        held(&self.inner, |driver| driver.calibration())
    }

    /// The die id read at initialization, or `null` before it.
    #[napi(getter)]
    pub fn identity(&self) -> Option<Ina226DieId> {
        held(&self.inner, |driver| driver.die_id().map(Into::into))
    }
}

/// A Texas Instruments ADS1115 driven over an I2C bus, converting one input on demand.
#[napi]
pub struct Ads1115 {
    inner: Arc<Mutex<Ads1115Driver>>,
}

#[napi]
impl Ads1115 {
    /// A driver for the part at `address` on `bus`. Nothing is sent until `init` or the first
    /// `sample`.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, address: u8, settings: Option<Ads1115Settings>) -> Self {
        let reset = ads1115::Config::default();
        let (mux, pga, data_rate) = match settings {
            Some(settings) => (
                settings.mux.map_or(reset.mux, ads1115::Mux::from_code),
                settings.pga.map_or(reset.pga, ads1115::Pga::from_code),
                settings
                    .data_rate
                    .map_or(reset.data_rate, ads1115::DataRate::from_code),
            ),
            None => (reset.mux, reset.pga, reset.data_rate),
        };
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = ads1115::Ads1115::new(bus, address, delay)
            .with_input(mux)
            .with_gain(pga)
            .with_data_rate(data_rate);
        Ads1115 {
            inner: shared(driver),
        }
    }

    /// Writes the input, range, and data rate, and reads the configuration back. Rejects when
    /// nothing answers or the configuration reads back differently.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Runs one conversion of the configured input and resolves with it, initializing the
    /// part first if `init` has not run.
    #[napi]
    pub async fn sample(&self) -> napi::Result<Ads1115Sample> {
        let sample = drive(&self.inner, |driver| driver.sample()).await?;
        Ok(sample_of(sample))
    }

    /// Converts another input once, by its multiplexer code, leaving the configured input as
    /// it was.
    #[napi(js_name = "sampleInput")]
    pub async fn sample_input(&self, mux: u8) -> napi::Result<Ads1115Sample> {
        let mux = ads1115::Mux::from_code(mux);
        let sample = drive(&self.inner, move |driver| driver.sample_input(mux)).await?;
        Ok(sample_of(sample))
    }

    /// The configuration the driver writes.
    #[napi(getter)]
    pub fn config(&self) -> Ads1115Config {
        held(&self.inner, |driver| driver.config().into())
    }
}

/// A Sensirion SHT3x driven over an I2C bus, measuring on demand in single-shot mode.
#[napi(js_name = "Sht3x")]
pub struct Sht3x {
    inner: Arc<Mutex<Sht3xDriver>>,
}

#[napi]
impl Sht3x {
    /// A driver for the part at `address` on `bus`. Nothing is sent until `init` or the first
    /// `measure`.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, address: u8, settings: Option<Sht3xSettings>) -> Self {
        let repeatability = settings
            .and_then(|settings| settings.repeatability)
            .map_or(sht3x::Repeatability::High, Into::into);
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = sht3x::Sht3x::new(bus, address, delay).with_repeatability(repeatability);
        Sht3x {
            inner: shared(driver),
        }
    }

    /// Soft-resets the part and reads its status; a status word whose checksum holds is what
    /// confirms an SHT3x answers. Rejects when nothing answers or the word fails its checksum.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Runs one single-shot measurement and resolves with it, initializing the part first if
    /// `init` has not run. Rejects when a data word fails its checksum.
    #[napi]
    pub async fn measure(&self) -> napi::Result<Sht3xMeasurement> {
        let measurement = drive(&self.inner, |driver| driver.measure()).await?;
        Ok(measurement.into())
    }

    /// Reads the status register, which `lastStatus` keeps as well.
    #[napi(js_name = "readStatus")]
    pub async fn read_status(&self) -> napi::Result<Sht3xStatus> {
        let status = drive(&self.inner, |driver| driver.read_status()).await?;
        Ok(status.into())
    }

    /// Switches the plausibility-check heater on, initializing the part first if needed.
    #[napi(js_name = "heaterOn")]
    pub async fn heater_on(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.heater_on()).await
    }

    /// Switches the heater off, which is its state after any reset.
    #[napi(js_name = "heaterOff")]
    pub async fn heater_off(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.heater_off()).await
    }

    /// The status register as it was last read, or `null` before it has been.
    #[napi(getter, js_name = "lastStatus")]
    pub fn last_status(&self) -> Option<Sht3xStatus> {
        held(&self.inner, |driver| driver.status().map(Into::into))
    }
}

/// A Sensirion SCD40 or SCD41 driven over an I2C bus in periodic measurement, a result every
/// five seconds.
#[napi(js_name = "Scd4x")]
pub struct Scd4x {
    inner: Arc<Mutex<Scd4xDriver>>,
}

#[napi]
impl Scd4x {
    /// A driver for the part on `bus`, which has one address. Nothing is sent until `init` or
    /// the first `measure`.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus) -> Self {
        let bus = bus.inner.clone();
        let delay = bus.delay();
        Scd4x {
            inner: shared(scd4x::Scd4x::new(bus, delay)),
        }
    }

    /// Stops any running measurement, reads the serial number, and starts periodic
    /// measurement. Rejects when nothing answers or the serial number fails its checksum.
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.init()).await
    }

    /// Waits for the next periodic result and resolves with it, initializing the part first if
    /// `init` has not run.
    #[napi]
    pub async fn measure(&self) -> napi::Result<Scd4xMeasurement> {
        let measurement = drive(&self.inner, |driver| driver.measure()).await?;
        Ok(measurement.into())
    }

    /// Runs one on-demand measurement on an SCD41, which takes five seconds. The part must not
    /// be measuring periodically: call `stop` first, or use this in place of `init`.
    #[napi(js_name = "measureSingleShot")]
    pub async fn measure_single_shot(&self) -> napi::Result<Scd4xMeasurement> {
        let measurement = drive(&self.inner, |driver| driver.measure_single_shot()).await?;
        Ok(measurement.into())
    }

    /// Asks the part whether a periodic result is waiting, so `measure` would read at once.
    #[napi(js_name = "dataReady")]
    pub async fn data_ready(&self) -> napi::Result<bool> {
        drive(&self.inner, |driver| driver.data_ready()).await
    }

    /// Stops periodic measurement, after which the part takes its settings commands.
    #[napi]
    pub async fn stop(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.stop()).await
    }

    /// Starts periodic measurement.
    #[napi]
    pub async fn start(&self) -> napi::Result<()> {
        drive(&self.inner, |driver| driver.start()).await
    }

    /// Sets the temperature offset that compensates the part's own warmth, in millidegrees,
    /// until power is lost.
    #[napi(js_name = "setTemperatureOffset")]
    pub async fn set_temperature_offset(&self, milli_celsius: u32) -> napi::Result<()> {
        drive(&self.inner, move |driver| {
            driver.set_temperature_offset(milli_celsius)
        })
        .await
    }

    /// Sets the altitude the part corrects its carbon dioxide reading for, in meters.
    #[napi(js_name = "setSensorAltitude")]
    pub async fn set_sensor_altitude(&self, meters: u16) -> napi::Result<()> {
        drive(&self.inner, move |driver| {
            driver.set_sensor_altitude(meters)
        })
        .await
    }

    /// The 48-bit serial number read at initialization, or `null` before it.
    #[napi(getter)]
    pub fn serial(&self) -> Option<i64> {
        held(&self.inner, |driver| {
            driver.serial().map(|serial| serial as i64)
        })
    }
}

/// A DS18B20 the Linux kernel serves as a `w1_slave` file under `/sys/bus/w1/devices`.
#[napi(js_name = "Ds18b20Thermometer")]
pub struct Ds18b20Thermometer {
    inner: ds18b20::linux::Thermometer,
}

#[napi]
impl Ds18b20Thermometer {
    /// A thermometer named by the serial in its directory name, the twelve hex digits after
    /// `28-`.
    #[napi(factory, js_name = "forSerial")]
    pub fn for_serial(serial: String) -> Self {
        Ds18b20Thermometer {
            inner: ds18b20::linux::Thermometer::new(&serial),
        }
    }

    /// A thermometer named by the path of its `w1_slave` file.
    #[napi(factory)]
    pub fn at(path: String) -> Self {
        Ds18b20Thermometer {
            inner: ds18b20::linux::Thermometer::at(path),
        }
    }

    /// Every DS18B20 the kernel has found, one per `28-` directory under `devices`, which is
    /// `/sys/bus/w1/devices` unless given. Throws when the directory cannot be listed, which
    /// usually means the 1-Wire overlay is off.
    #[napi]
    pub fn discover(devices: Option<String>) -> napi::Result<Vec<Ds18b20Thermometer>> {
        let devices = devices.unwrap_or_else(|| ds18b20::linux::DEVICES.to_owned());
        ds18b20::linux::Thermometer::discover_in(Path::new(&devices))
            .map(|found| {
                found
                    .into_iter()
                    .map(|inner| Ds18b20Thermometer { inner })
                    .collect()
            })
            .map_err(|error| napi::Error::from_reason(format!("{devices}: {error}")))
    }

    /// The path of the file the thermometer reads.
    #[napi(getter)]
    pub fn path(&self) -> String {
        self.inner.path().to_string_lossy().into_owned()
    }

    /// Reads the file on a worker thread, which makes the kernel run a conversion, and
    /// resolves with the decoded reading. Rejects when the file cannot be read or the kernel
    /// or this decoder rejects the checksum.
    #[napi]
    pub async fn read(&self) -> napi::Result<Ds18b20Reading> {
        let thermometer = self.inner.clone();
        spawn_blocking(move || thermometer.read_scratchpad())
            .await
            .map_err(|error| napi::Error::from_reason(format!("the read did not finish: {error}")))?
            .map(Into::into)
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }
}

/// A simulated BME280 holding a real part's calibration and one measurement it took, which
/// compensate to 20.44 C, 848.05 hPa, and 44.65 %.
#[napi]
pub fn bme280_sim_part(address: u8) -> I2cPart {
    I2cPart {
        inner: bme280::sim::part(address),
    }
}

/// A simulated BME280 that reads what it is asked to, to within what its converter can
/// represent.
#[napi]
pub fn bme280_sim_reporting(
    address: u8,
    celsius: f64,
    hectopascals: f64,
    relative_humidity: f64,
) -> I2cPart {
    I2cPart {
        inner: bme280::sim::reporting(
            address,
            celsius as f32,
            hectopascals as f32,
            relative_humidity as f32,
        ),
    }
}

/// The 26-byte temperature and pressure calibration block a simulated BME280 holds.
#[napi]
pub fn bme280_sim_calibration() -> Buffer {
    Buffer::from(bme280::sim::CALIBRATION.to_vec())
}

/// The 7-byte humidity calibration block a simulated BME280 holds.
#[napi]
pub fn bme280_sim_calibration_humidity() -> Buffer {
    Buffer::from(bme280::sim::CALIBRATION_HUMIDITY.to_vec())
}

/// The eight data registers a simulated BME280 holds: one measurement a real part took.
#[napi]
pub fn bme280_sim_burst() -> Buffer {
    Buffer::from(bme280::sim::BURST.to_vec())
}

/// The eight data registers that compensate to a reading against the simulated calibration.
#[napi]
pub fn bme280_sim_burst_for(celsius: f64, hectopascals: f64, relative_humidity: f64) -> Buffer {
    Buffer::from(
        bme280::sim::burst_for(
            celsius as f32,
            hectopascals as f32,
            relative_humidity as f32,
        )
        .to_vec(),
    )
}

/// A simulated BMP280 holding a real part's trimming and one measurement it took, which
/// compensate to 20.44 C and 848.05 hPa.
#[napi]
pub fn bmp280_sim_part(address: u8) -> I2cPart {
    I2cPart {
        inner: bmp280::sim::part(address),
    }
}

/// A simulated BMP280 that reads what it is asked to, within a hundredth of a degree and of a
/// hectopascal.
#[napi]
pub fn bmp280_sim_reporting(address: u8, celsius: f64, hectopascals: f64) -> I2cPart {
    I2cPart {
        inner: bmp280::sim::reporting(address, celsius as f32, hectopascals as f32),
    }
}

/// The 24 trimming bytes a simulated BMP280 holds.
#[napi]
pub fn bmp280_sim_calibration() -> Buffer {
    Buffer::from(bmp280::sim::CALIBRATION.to_vec())
}

/// The six data registers a simulated BMP280 holds: one measurement a real part took.
#[napi]
pub fn bmp280_sim_burst() -> Buffer {
    Buffer::from(bmp280::sim::BURST.to_vec())
}

/// The six data registers that compensate to a reading against the simulated trimming.
#[napi]
pub fn bmp280_sim_burst_for(celsius: f64, hectopascals: f64) -> Buffer {
    Buffer::from(bmp280::sim::burst_for(celsius as f32, hectopascals as f32).to_vec())
}

/// A simulated TMP117 reading 21.25 C, its configuration register keeping the flags the part
/// sets for itself, with the data-ready flag set.
#[napi]
pub fn tmp117_sim_part(address: u8) -> WordPart {
    WordPart {
        inner: tmp117::sim::part(address),
    }
}

/// A simulated TMP117 that reads what it is asked to, to the nearest 7.8125 millidegrees.
#[napi]
pub fn tmp117_sim_reporting(address: u8, celsius: f64) -> WordPart {
    WordPart {
        inner: tmp117::sim::reporting(address, celsius as f32),
    }
}

/// A simulated OPT3001 reading 380 lux, its conversion-ready flag set.
#[napi]
pub fn opt3001_sim_part(address: u8) -> WordPart {
    WordPart {
        inner: opt3001::sim::part(address),
    }
}

/// A simulated OPT3001 that reads what it is asked to, to the nearest step its exponent and
/// mantissa represent.
#[napi]
pub fn opt3001_sim_reporting(address: u8, lux: f64) -> WordPart {
    WordPart {
        inner: opt3001::sim::reporting(address, lux as f32),
    }
}

/// A simulated HDC1080 reading 22.5 C and 45 %.
#[napi]
pub fn hdc1080_sim_part() -> WordPart {
    WordPart {
        inner: hdc1080::sim::part(),
    }
}

/// A simulated HDC1080 that reads what it is asked to, within three thousandths of a degree
/// and two thousandths of a percent.
#[napi]
pub fn hdc1080_sim_reporting(celsius: f64, relative_humidity: f64) -> WordPart {
    WordPart {
        inner: hdc1080::sim::reporting(celsius as f32, relative_humidity as f32),
    }
}

/// A simulated INA219 carrying 500 mA at 12 V through the 100 milliohm shunt a driver starts
/// with.
#[napi]
pub fn ina219_sim_part(address: u8) -> WordPart {
    WordPart {
        inner: ina219::sim::part(address),
    }
}

/// A simulated INA219 that reads what it is asked to, calibrated for the same shunt and largest
/// current a driver is given.
#[napi]
pub fn ina219_sim_reporting(
    address: u8,
    shunt_milliohms: u32,
    max_microamps: u32,
    bus_millivolts: u32,
    microamps: i32,
) -> WordPart {
    WordPart {
        inner: ina219::sim::reporting(
            address,
            shunt_milliohms,
            max_microamps,
            bus_millivolts,
            microamps,
        ),
    }
}

/// A simulated INA226 carrying 500 mA at 12 V through the 100 milliohm shunt a driver starts
/// with.
#[napi]
pub fn ina226_sim_part(address: u8) -> WordPart {
    WordPart {
        inner: ina226::sim::part(address),
    }
}

/// A simulated INA226 that reads what it is asked to, calibrated for the same shunt and largest
/// current a driver is given.
#[napi]
pub fn ina226_sim_reporting(
    address: u8,
    shunt_milliohms: u32,
    max_microamps: u32,
    bus_microvolts: u32,
    microamps: i32,
) -> WordPart {
    WordPart {
        inner: ina226::sim::reporting(
            address,
            shunt_milliohms,
            max_microamps,
            bus_microvolts,
            microamps,
        ),
    }
}

/// A simulated ADS1115 reading 1.65 V, half a 3.3 V supply, at the range a driver starts with.
#[napi]
pub fn ads1115_sim_part(address: u8) -> WordPart {
    WordPart {
        inner: ads1115::sim::part(address),
    }
}

/// A simulated ADS1115 that reads what it is asked to at the gain code a driver converts at.
#[napi]
pub fn ads1115_sim_reporting(address: u8, pga: u8, volts: f64) -> WordPart {
    WordPart {
        inner: ads1115::sim::reporting(address, ads1115::Pga::from_code(pga), volts as f32),
    }
}

/// A simulated SHT3x reading 22.5 C and 45 %.
#[napi(js_name = "sht3xSimPart")]
pub fn sht3x_sim_part(address: u8) -> CommandPart {
    CommandPart {
        inner: sht3x::sim::part(address),
    }
}

/// A simulated SHT3x that reads what it is asked to, within three thousandths of a degree and
/// two thousandths of a percent.
#[napi(js_name = "sht3xSimReporting")]
pub fn sht3x_sim_reporting(address: u8, celsius: f64, relative_humidity: f64) -> CommandPart {
    CommandPart {
        inner: sht3x::sim::reporting(address, celsius as f32, relative_humidity as f32),
    }
}

/// A simulated SCD4x reading 800 ppm, 22.5 C, and 45 %, always with a result waiting.
#[napi(js_name = "scd4xSimPart")]
pub fn scd4x_sim_part() -> CommandPart {
    CommandPart {
        inner: scd4x::sim::part(),
    }
}

/// A simulated SCD4x that reads what it is asked to.
#[napi(js_name = "scd4xSimReporting")]
pub fn scd4x_sim_reporting(co2_ppm: u16, celsius: f64, relative_humidity: f64) -> CommandPart {
    CommandPart {
        inner: scd4x::sim::reporting(co2_ppm, celsius as f32, relative_humidity as f32),
    }
}

/// An ADS1115 sample as JavaScript holds it.
fn sample_of(sample: ads1115::Sample) -> Ads1115Sample {
    Ads1115Sample {
        raw: sample.raw,
        pga: sample.pga.code(),
        nanovolts: sample.nanovolts(),
        volts: f64::from(sample.volts()),
    }
}

/// A driver held for calls from worker threads.
fn shared<D>(driver: D) -> Arc<Mutex<D>> {
    Arc::new(Mutex::new(driver))
}

/// Reads something a driver holds without touching the bus.
fn held<D, T>(inner: &Arc<Mutex<D>>, read: impl FnOnce(&D) -> T) -> T {
    let driver = inner.lock().unwrap_or_else(PoisonError::into_inner);
    read(&driver)
}

/// Runs a driver call on a blocking worker thread.
async fn drive<D: Send + 'static, T: Send + 'static>(
    inner: &Arc<Mutex<D>>,
    call: impl FnOnce(&mut D) -> Result<T, DriverError<BusError>> + Send + 'static,
) -> napi::Result<T> {
    let inner = Arc::clone(inner);
    let outcome = spawn_blocking(move || {
        let mut driver = inner.lock().unwrap_or_else(PoisonError::into_inner);
        call(&mut driver)
    })
    .await
    .map_err(|error| {
        napi::Error::from_reason(format!("the driver call did not finish: {error}"))
    })?;
    outcome.map_err(driver_error)
}

/// A driver failure as a JavaScript error, in the bus's own words when the bus was the cause.
fn driver_error(error: DriverError<BusError>) -> napi::Error {
    let reason = match error {
        DriverError::Bus(error) => error.to_string(),
        DriverError::Sensor(error) => error.to_string(),
        DriverError::Timeout => DriverError::<BusError>::Timeout.to_string(),
    };
    napi::Error::from_reason(reason)
}
