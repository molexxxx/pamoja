//! Generated Python bindings for the sensor drivers that run over an I2C bus.
//!
//! A driver runs the datasheet's whole conversation with a part over an `I2cBus`: reset,
//! identify, read the calibration, configure, measure. It holds its own share of the bus, and
//! each call releases the interpreter while the part answers, since a measurement waits the
//! datasheet's time before it reads. Every part has a simulated twin that answers a driver
//! with nothing plugged in.
//!
//! The DS18B20 is read the way a Linux process reaches one, through the files the kernel's
//! 1-Wire driver serves, which a program can also write for itself to test against.

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_hal::bus::{BusDelay, BusError, I2cBus as Bus};
use pamoja_sensors::driver::I2cRegisters;
use pamoja_sensors::{
    ads1115, bme280, bmp280, ds18b20, hdc1080, ina219, ina226, opt3001, scd4x, sht3x, tmp117,
    DriverError,
};

use crate::hal::{CommandPart, I2cBus, I2cPart, WordPart};
use crate::sensors::{
    read_repeatability, Ads1115Config, Bme280Measurement, Bmp280Coefficients, Bmp280Reading,
    Ds18b20Reading, Hdc1080Config, Hdc1080Measurement, Ina219Config, Ina226Config, Ina226DieId,
    Ina226MaskEnable, Opt3001Config, Scd4xMeasurement, Sht3xMeasurement, Sht3xStatus,
};
use crate::PamojaError;

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

/// A TMP117 temperature result.
#[gen_stub_pyclass]
#[pyclass]
pub struct Tmp117Reading {
    /// The temperature register, 7.8125 millidegrees Celsius per count.
    #[pyo3(get)]
    raw: i16,
    /// The temperature in micro-degrees Celsius, exact in integer arithmetic.
    #[pyo3(get)]
    micro_celsius: i32,
    /// The temperature in degrees Celsius.
    #[pyo3(get)]
    celsius: f32,
}

/// The TMP117's alert flags: whether a result since they were last read crossed a limit.
#[gen_stub_pyclass]
#[pyclass]
pub struct Tmp117Alerts {
    /// A result was above the high limit.
    #[pyo3(get)]
    high: bool,
    /// A result was below the low limit.
    #[pyo3(get)]
    low: bool,
}

/// An OPT3001 illuminance result.
#[gen_stub_pyclass]
#[pyclass]
pub struct Opt3001Reading {
    /// The result register: a four-bit exponent over a twelve-bit mantissa.
    #[pyo3(get)]
    raw: u16,
    /// The illuminance in millilux, exact in integer arithmetic.
    #[pyo3(get)]
    milli_lux: u32,
    /// The illuminance in lux.
    #[pyo3(get)]
    lux: f32,
}

/// One INA219 conversion: the four result registers as read, and what they mean.
#[gen_stub_pyclass]
#[pyclass]
pub struct Ina219Reading {
    /// The shunt-voltage register.
    #[pyo3(get)]
    shunt: i16,
    /// The bus-voltage register, flags included.
    #[pyo3(get)]
    bus: u16,
    /// The current register.
    #[pyo3(get)]
    current: i16,
    /// The power register.
    #[pyo3(get)]
    power: u16,
    /// The current step the calibration programmed, in microamps per count.
    #[pyo3(get)]
    current_lsb_microamps: u32,
    /// The shunt voltage in microvolts.
    #[pyo3(get)]
    shunt_microvolts: i32,
    /// The bus voltage in millivolts.
    #[pyo3(get)]
    bus_millivolts: u32,
    /// The current in microamps; negative flows the other way through the shunt.
    #[pyo3(get)]
    current_microamps: i32,
    /// The power in microwatts.
    #[pyo3(get)]
    power_microwatts: u32,
    /// Whether the part's arithmetic overflowed, leaving current and power meaningless.
    #[pyo3(get)]
    math_overflow: bool,
}

/// One INA226 conversion: the four result registers as read, and what they mean.
#[gen_stub_pyclass]
#[pyclass]
pub struct Ina226Reading {
    /// The shunt-voltage register.
    #[pyo3(get)]
    shunt: i16,
    /// The bus-voltage register.
    #[pyo3(get)]
    bus: u16,
    /// The current register.
    #[pyo3(get)]
    current: i16,
    /// The power register.
    #[pyo3(get)]
    power: u16,
    /// The current step the calibration programmed, in microamps per count.
    #[pyo3(get)]
    current_lsb_microamps: u32,
    /// The shunt voltage in nanovolts.
    #[pyo3(get)]
    shunt_nanovolts: i32,
    /// The shunt voltage in millivolts.
    #[pyo3(get)]
    shunt_millivolts: f32,
    /// The bus voltage in microvolts.
    #[pyo3(get)]
    bus_microvolts: u32,
    /// The bus voltage in volts.
    #[pyo3(get)]
    bus_volts: f32,
    /// The current in microamps; negative flows the other way through the shunt.
    #[pyo3(get)]
    current_microamps: i32,
    /// The current in amps.
    #[pyo3(get)]
    current_amps: f32,
    /// The power in microwatts.
    #[pyo3(get)]
    power_microwatts: u32,
    /// The power in watts.
    #[pyo3(get)]
    power_watts: f32,
    /// Whether the part's arithmetic overflowed, leaving current and power meaningless.
    #[pyo3(get)]
    math_overflow: bool,
}

/// One ADS1115 conversion.
#[gen_stub_pyclass]
#[pyclass]
pub struct Ads1115Sample {
    /// The conversion register, two's complement.
    #[pyo3(get)]
    raw: i16,
    /// The gain code the conversion ran at.
    #[pyo3(get)]
    pga: u8,
    /// The voltage in nanovolts, exact in integer arithmetic.
    #[pyo3(get)]
    nanovolts: i64,
    /// The voltage in volts.
    #[pyo3(get)]
    volts: f32,
}

/// A Bosch BME280 driven over an I2C bus, measuring on demand in forced mode.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Bme280 {
    inner: Mutex<Bme280Driver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Bme280 {
    /// A driver for the part at `address` on `bus`. The oversampling codes default to x1 and
    /// the filter to off. Nothing is sent until `init` or the first `measure`.
    #[new]
    #[pyo3(signature = (bus, address, temperature = 1, pressure = 1, humidity = 1, filter = 0))]
    fn new(
        bus: PyRef<'_, I2cBus>,
        address: u8,
        temperature: u8,
        pressure: u8,
        humidity: u8,
        filter: u8,
    ) -> Self {
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
            inner: Mutex::new(driver),
        }
    }

    /// Resets the part, checks it is a BME280, reads its calibration, and writes the settings,
    /// leaving the part asleep. Raises `PamojaError` when nothing answers or another part does.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Runs one forced measurement and returns the compensated reading, initializing the part
    /// first if `init` has not run.
    fn measure(&self, py: Python<'_>) -> PyResult<Bme280Measurement> {
        drive(&self.inner, py, |driver| driver.measure()).map(Bme280Measurement::from)
    }
}

/// A Bosch BMP280 driven over an I2C bus, measuring on demand in forced mode.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Bmp280 {
    inner: Mutex<Bmp280Driver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Bmp280 {
    /// A driver for the part at `address` on `bus`. The oversampling codes default to x1 and
    /// the filter code to off. Nothing is sent until `init` or the first `measure`.
    #[new]
    #[pyo3(signature = (bus, address, temperature = 1, pressure = 1, filter = 0))]
    fn new(bus: PyRef<'_, I2cBus>, address: u8, temperature: u8, pressure: u8, filter: u8) -> Self {
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = bmp280::Bmp280::i2c(bus, address, delay)
            .with_oversampling(
                bmp280::Oversampling::from_code(temperature),
                bmp280::Oversampling::from_code(pressure),
            )
            .with_filter(filter);
        Bmp280 {
            inner: Mutex::new(driver),
        }
    }

    /// Resets the part, checks it is a BMP280, reads its trimming, and writes the settings,
    /// leaving the part asleep. Raises `PamojaError` when nothing answers or another part does.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Runs one forced measurement and returns the compensated reading, initializing the part
    /// first if `init` has not run.
    fn measure(&self, py: Python<'_>) -> PyResult<Bmp280Reading> {
        drive(&self.inner, py, |driver| driver.measure()).map(Bmp280Reading::from)
    }

    /// The trimming coefficients read at initialization, or `None` before it.
    #[getter]
    fn coefficients(&self) -> Option<Bmp280Coefficients> {
        held(&self.inner, |driver| {
            driver
                .calibration()
                .map(|calibration| (*calibration).into())
        })
    }
}

/// A Texas Instruments TMP117 driven over an I2C bus, converting on demand in one-shot mode.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Tmp117 {
    inner: Mutex<Tmp117Driver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Tmp117 {
    /// A driver for the part at `address` on `bus`, averaging eight conversions into each
    /// result unless given another code. Nothing is sent until `init` or the first `measure`.
    #[new]
    #[pyo3(signature = (bus, address, averaging = 1))]
    fn new(bus: PyRef<'_, I2cBus>, address: u8, averaging: u8) -> Self {
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = tmp117::Tmp117::new(bus, address, delay)
            .with_averaging(tmp117::Averaging::from_code(averaging));
        Tmp117 {
            inner: Mutex::new(driver),
        }
    }

    /// Checks the part is a TMP117, waits for its EEPROM to finish loading, and writes the
    /// settings with the part in shutdown. Raises `PamojaError` when nothing answers or
    /// another part does.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Runs one conversion and returns the temperature, initializing the part first if `init`
    /// has not run.
    fn measure(&self, py: Python<'_>) -> PyResult<Tmp117Reading> {
        let reading = drive(&self.inner, py, |driver| driver.measure())?;
        Ok(Tmp117Reading {
            raw: reading.raw(),
            micro_celsius: reading.micro_celsius(),
            celsius: reading.celsius(),
        })
    }

    /// Writes the high and low limits the part compares each result against; the factory
    /// limits are 192 C and -256 C.
    fn set_alert_limits(
        &self,
        py: Python<'_>,
        high_celsius: f32,
        low_celsius: f32,
    ) -> PyResult<()> {
        drive(&self.inner, py, move |driver| {
            driver.set_alert_limits(high_celsius, low_celsius)
        })
    }

    /// Reads the alert flags: whether a result since the last call was above the high limit
    /// or below the low limit, including results the driver's own reads saw.
    fn alerts(&self, py: Python<'_>) -> PyResult<Tmp117Alerts> {
        let alerts = drive(&self.inner, py, |driver| driver.alerts())?;
        Ok(Tmp117Alerts {
            high: alerts.high,
            low: alerts.low,
        })
    }

    /// The silicon revision read at initialization, or `None` before it.
    #[getter]
    fn silicon_revision(&self) -> Option<u8> {
        held(&self.inner, |driver| driver.revision())
    }
}

/// A Texas Instruments OPT3001 driven over an I2C bus, measuring on demand in single-shot mode.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Opt3001 {
    inner: Mutex<Opt3001Driver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Opt3001 {
    /// A driver for the part at `address` on `bus`, integrating each conversion for 800 ms
    /// and choosing its own range unless told otherwise. Nothing is sent until `init` or the
    /// first `measure`.
    #[new]
    #[pyo3(signature = (bus, address, long_conversion = true, range_number = 12))]
    fn new(bus: PyRef<'_, I2cBus>, address: u8, long_conversion: bool, range_number: u8) -> Self {
        let conversion_time = if long_conversion {
            opt3001::ConversionTime::Ms800
        } else {
            opt3001::ConversionTime::Ms100
        };
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = opt3001::Opt3001::new(bus, address, delay)
            .with_conversion_time(conversion_time)
            .with_range(range_number);
        Opt3001 {
            inner: Mutex::new(driver),
        }
    }

    /// Checks the part is an OPT3001 and writes the settings with the part in shutdown.
    /// Raises `PamojaError` when nothing answers or another part does.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Runs one conversion and returns the illuminance, initializing the part first if `init`
    /// has not run.
    fn measure(&self, py: Python<'_>) -> PyResult<Opt3001Reading> {
        let reading = drive(&self.inner, py, |driver| driver.measure())?;
        Ok(Opt3001Reading {
            raw: reading.raw(),
            milli_lux: reading.milli_lux(),
            lux: reading.lux(),
        })
    }

    /// Writes the low and high limits the part's interrupt pin compares each result against,
    /// in millilux.
    fn set_limits(&self, py: Python<'_>, low_milli_lux: u32, high_milli_lux: u32) -> PyResult<()> {
        drive(&self.inner, py, move |driver| {
            driver.set_limits(low_milli_lux, high_milli_lux)
        })
    }

    /// The configuration the driver writes, with the part in shutdown.
    #[getter]
    fn configuration(&self) -> Opt3001Config {
        held(&self.inner, |driver| driver.configuration().into())
    }
}

/// A Texas Instruments HDC1080 driven over an I2C bus, measuring temperature then humidity from
/// one trigger.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Hdc1080 {
    inner: Mutex<Hdc1080Driver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Hdc1080 {
    /// A driver for the part on `bus`, which has one address, at 14 bits a channel unless
    /// told otherwise. Raises `ValueError` for a resolution the part does not have.
    #[new]
    #[pyo3(signature = (bus, temperature_resolution_bits = 14, humidity_resolution_bits = 14))]
    fn new(
        bus: PyRef<'_, I2cBus>,
        temperature_resolution_bits: u8,
        humidity_resolution_bits: u8,
    ) -> PyResult<Self> {
        let temperature = match temperature_resolution_bits {
            14 => hdc1080::TemperatureResolution::Bits14,
            11 => hdc1080::TemperatureResolution::Bits11,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "HDC1080 temperature resolution must be 14 or 11 bits",
                ))
            }
        };
        let humidity = match humidity_resolution_bits {
            14 => hdc1080::HumidityResolution::Bits14,
            11 => hdc1080::HumidityResolution::Bits11,
            8 => hdc1080::HumidityResolution::Bits8,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "HDC1080 humidity resolution must be 14, 11, or 8 bits",
                ))
            }
        };
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = hdc1080::Hdc1080::new(bus, delay).with_resolutions(temperature, humidity);
        Ok(Hdc1080 {
            inner: Mutex::new(driver),
        })
    }

    /// Checks the part is an HDC1080 and writes the configuration. Raises `PamojaError` when
    /// nothing answers or another part does.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Triggers one acquisition of both channels and returns them, initializing the part first
    /// if `init` has not run. Raises `PamojaError` when the part does not acknowledge the read,
    /// which it refuses until its results are ready.
    fn measure(&self, py: Python<'_>) -> PyResult<Hdc1080Measurement> {
        drive(&self.inner, py, |driver| driver.measure()).map(Hdc1080Measurement::from)
    }

    /// Switches the on-die heater, which runs only during acquisitions, on or off.
    fn heater(&self, py: Python<'_>, on: bool) -> PyResult<()> {
        drive(&self.inner, py, move |driver| driver.heater(on))
    }

    /// The configuration the driver writes.
    #[getter]
    fn configuration(&self) -> Hdc1080Config {
        held(&self.inner, |driver| driver.configuration().into())
    }
}

/// A Texas Instruments INA219 driven over an I2C bus, measuring shunt and bus voltage, current,
/// and power on demand.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Ina219 {
    inner: Mutex<Ina219Driver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Ina219 {
    /// A driver for the part at `address` on `bus`: a 100 milliohm shunt sized for 3.2 A and
    /// the power-on register settings unless told otherwise. Nothing is sent until `init` or
    /// the first `measure`.
    #[new]
    #[pyo3(signature = (
        bus,
        address,
        shunt_milliohms = 100,
        max_microamps = 3_200_000,
        current_lsb_microamps = None,
        config = None,
    ))]
    fn new(
        bus: PyRef<'_, I2cBus>,
        address: u8,
        shunt_milliohms: u32,
        max_microamps: u32,
        current_lsb_microamps: Option<u32>,
        config: Option<Ina219Config>,
    ) -> Self {
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let mut driver = ina219::Ina219::new(bus, address, delay)
            .with_shunt(shunt_milliohms, max_microamps)
            .with_configuration(config.map_or_else(ina219::Configuration::default, Into::into));
        if let Some(lsb) = current_lsb_microamps {
            driver = driver.with_current_lsb(lsb);
        }
        Ina219 {
            inner: Mutex::new(driver),
        }
    }

    /// Resets the part, writes the configuration and the calibration, and reads the
    /// calibration back. Raises `PamojaError` when nothing answers or it does not hold.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Triggers one shunt and bus conversion and returns every result, initializing the part
    /// first if `init` has not run.
    fn measure(&self, py: Python<'_>) -> PyResult<Ina219Reading> {
        let reading = drive(&self.inner, py, |driver| driver.measure())?;
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
    #[getter]
    fn current_lsb_microamps(&self) -> u32 {
        held(&self.inner, |driver| driver.current_lsb_microamps())
    }

    /// The calibration word the driver programs.
    #[getter]
    fn calibration_word(&self) -> u16 {
        held(&self.inner, |driver| driver.calibration())
    }
}

/// A Texas Instruments INA226 driven over an I2C bus, measuring shunt and bus voltage, current,
/// and power on demand.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Ina226 {
    inner: Mutex<Ina226Driver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Ina226 {
    /// A driver for the part at `address` on `bus`: a 100 milliohm shunt sized for 3.2 A and
    /// the power-on register settings unless told otherwise. Nothing is sent until `init` or
    /// the first `measure`.
    #[new]
    #[pyo3(signature = (
        bus,
        address,
        shunt_milliohms = 100,
        max_microamps = 3_200_000,
        current_lsb_microamps = None,
        config = None,
    ))]
    fn new(
        bus: PyRef<'_, I2cBus>,
        address: u8,
        shunt_milliohms: u32,
        max_microamps: u32,
        current_lsb_microamps: Option<u32>,
        config: Option<Ina226Config>,
    ) -> Self {
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let mut driver = ina226::Ina226::new(bus, address, delay)
            .with_shunt(shunt_milliohms, max_microamps)
            .with_configuration(config.map_or(ina226::Configuration::RESET, Into::into));
        if let Some(lsb) = current_lsb_microamps {
            driver = driver.with_current_lsb(lsb);
        }
        Ina226 {
            inner: Mutex::new(driver),
        }
    }

    /// Resets the part, checks it is an INA226, writes the configuration and the calibration,
    /// and reads the calibration back. Raises `PamojaError` when nothing answers or another
    /// part does.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Triggers one shunt and bus conversion and returns every result, initializing the part
    /// first if `init` has not run.
    fn measure(&self, py: Python<'_>) -> PyResult<Ina226Reading> {
        let reading = drive(&self.inner, py, |driver| driver.measure())?;
        Ok(Ina226Reading {
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
            math_overflow: reading.math_overflow,
        })
    }

    /// Programs the alert pin: which limit it watches, one function at a time, and the limit,
    /// in the units of the register the function watches.
    fn set_alert(&self, py: Python<'_>, mask: Ina226MaskEnable, limit: u16) -> PyResult<()> {
        let mask = ina226::MaskEnable::from(mask);
        drive(&self.inner, py, move |driver| driver.set_alert(mask, limit))
    }

    /// The current step the driver programs, in microamps per count.
    #[getter]
    fn current_lsb_microamps(&self) -> u32 {
        held(&self.inner, |driver| driver.current_lsb_microamps())
    }

    /// The calibration word the driver programs.
    #[getter]
    fn calibration_word(&self) -> u16 {
        held(&self.inner, |driver| driver.calibration())
    }

    /// The die id read at initialization, or `None` before it.
    #[getter]
    fn identity(&self) -> Option<Ina226DieId> {
        held(&self.inner, |driver| driver.die_id().map(Into::into))
    }
}

/// A Texas Instruments ADS1115 driven over an I2C bus, converting one input on demand.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Ads1115 {
    inner: Mutex<Ads1115Driver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Ads1115 {
    /// A driver for the part at `address` on `bus`, converting AIN0 against AIN1 at the
    /// 2.048 V range and 128 samples per second, the part's reset settings, unless told
    /// otherwise by code. Nothing is sent until `init` or the first `sample`.
    #[new]
    #[pyo3(signature = (bus, address, mux = 0, pga = 2, data_rate = 4))]
    fn new(bus: PyRef<'_, I2cBus>, address: u8, mux: u8, pga: u8, data_rate: u8) -> Self {
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = ads1115::Ads1115::new(bus, address, delay)
            .with_input(ads1115::Mux::from_code(mux))
            .with_gain(ads1115::Pga::from_code(pga))
            .with_data_rate(ads1115::DataRate::from_code(data_rate));
        Ads1115 {
            inner: Mutex::new(driver),
        }
    }

    /// Writes the input, range, and data rate, and reads the configuration back. Raises
    /// `PamojaError` when nothing answers or the configuration reads back differently.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Runs one conversion of the configured input and returns it, initializing the part first
    /// if `init` has not run.
    fn sample(&self, py: Python<'_>) -> PyResult<Ads1115Sample> {
        drive(&self.inner, py, |driver| driver.sample()).map(sample_of)
    }

    /// Converts another input once, by its multiplexer code, leaving the configured input as
    /// it was.
    fn sample_input(&self, py: Python<'_>, mux: u8) -> PyResult<Ads1115Sample> {
        let mux = ads1115::Mux::from_code(mux);
        drive(&self.inner, py, move |driver| driver.sample_input(mux)).map(sample_of)
    }

    /// The configuration the driver writes.
    #[getter]
    fn config(&self) -> Ads1115Config {
        held(&self.inner, |driver| driver.config().into())
    }
}

/// A Sensirion SHT3x driven over an I2C bus, measuring on demand in single-shot mode.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Sht3x {
    inner: Mutex<Sht3xDriver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Sht3x {
    /// A driver for the part at `address` on `bus`, at `"High"` repeatability unless given
    /// `"Low"` or `"Medium"`. Nothing is sent until `init` or the first `measure`.
    #[new]
    #[pyo3(signature = (bus, address, repeatability = "High"))]
    fn new(bus: PyRef<'_, I2cBus>, address: u8, repeatability: &str) -> PyResult<Self> {
        let repeatability = read_repeatability(repeatability)?;
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = sht3x::Sht3x::new(bus, address, delay).with_repeatability(repeatability);
        Ok(Sht3x {
            inner: Mutex::new(driver),
        })
    }

    /// Soft-resets the part and reads its status; a status word whose checksum holds is what
    /// confirms an SHT3x answers. Raises `PamojaError` when nothing answers or the word fails.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Runs one single-shot measurement and returns it, initializing the part first if `init`
    /// has not run. Raises `PamojaError` when a data word fails its checksum.
    fn measure(&self, py: Python<'_>) -> PyResult<Sht3xMeasurement> {
        drive(&self.inner, py, |driver| driver.measure()).map(Sht3xMeasurement::from)
    }

    /// Reads the status register, which `last_status` keeps as well.
    fn read_status(&self, py: Python<'_>) -> PyResult<Sht3xStatus> {
        drive(&self.inner, py, |driver| driver.read_status()).map(Sht3xStatus::from)
    }

    /// Switches the plausibility-check heater on, initializing the part first if needed.
    fn heater_on(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.heater_on())
    }

    /// Switches the heater off, which is its state after any reset.
    fn heater_off(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.heater_off())
    }

    /// The status register as it was last read, or `None` before it has been.
    #[getter]
    fn last_status(&self) -> Option<Sht3xStatus> {
        held(&self.inner, |driver| driver.status().map(Into::into))
    }
}

/// A Sensirion SCD40 or SCD41 driven over an I2C bus in periodic measurement, a result every
/// five seconds.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Scd4x {
    inner: Mutex<Scd4xDriver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Scd4x {
    /// A driver for the part on `bus`, which has one address. Nothing is sent until `init` or
    /// the first `measure`.
    #[new]
    fn new(bus: PyRef<'_, I2cBus>) -> Self {
        let bus = bus.inner.clone();
        let delay = bus.delay();
        Scd4x {
            inner: Mutex::new(scd4x::Scd4x::new(bus, delay)),
        }
    }

    /// Stops any running measurement, reads the serial number, and starts periodic
    /// measurement. Raises `PamojaError` when nothing answers or the serial number fails its
    /// checksum.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Waits for the next periodic result and returns it, initializing the part first if
    /// `init` has not run.
    fn measure(&self, py: Python<'_>) -> PyResult<Scd4xMeasurement> {
        drive(&self.inner, py, |driver| driver.measure()).map(Scd4xMeasurement::from)
    }

    /// Runs one on-demand measurement on an SCD41, which takes five seconds. The part must not
    /// be measuring periodically: call `stop` first, or use this in place of `init`.
    fn measure_single_shot(&self, py: Python<'_>) -> PyResult<Scd4xMeasurement> {
        drive(&self.inner, py, |driver| driver.measure_single_shot()).map(Scd4xMeasurement::from)
    }

    /// Asks the part whether a periodic result is waiting, so `measure` would read at once.
    fn data_ready(&self, py: Python<'_>) -> PyResult<bool> {
        drive(&self.inner, py, |driver| driver.data_ready())
    }

    /// Stops periodic measurement, after which the part takes its settings commands.
    fn stop(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.stop())
    }

    /// Starts periodic measurement.
    fn start(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.start())
    }

    /// Sets the temperature offset that compensates the part's own warmth, in millidegrees,
    /// until power is lost.
    fn set_temperature_offset(&self, py: Python<'_>, milli_celsius: u32) -> PyResult<()> {
        drive(&self.inner, py, move |driver| {
            driver.set_temperature_offset(milli_celsius)
        })
    }

    /// Sets the altitude the part corrects its carbon dioxide reading for, in meters.
    fn set_sensor_altitude(&self, py: Python<'_>, meters: u16) -> PyResult<()> {
        drive(&self.inner, py, move |driver| {
            driver.set_sensor_altitude(meters)
        })
    }

    /// The 48-bit serial number read at initialization, or `None` before it.
    #[getter]
    fn serial(&self) -> Option<u64> {
        held(&self.inner, |driver| driver.serial())
    }
}

/// A DS18B20 the Linux kernel serves as a `w1_slave` file under `/sys/bus/w1/devices`.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Ds18b20Thermometer {
    inner: ds18b20::linux::Thermometer,
}

#[gen_stub_pymethods]
#[pymethods]
impl Ds18b20Thermometer {
    /// A thermometer named by the serial in its directory name, the twelve hex digits after
    /// `28-`.
    #[staticmethod]
    fn for_serial(serial: &str) -> Self {
        Ds18b20Thermometer {
            inner: ds18b20::linux::Thermometer::new(serial),
        }
    }

    /// A thermometer named by the path of its `w1_slave` file.
    #[staticmethod]
    fn at(path: String) -> Self {
        Ds18b20Thermometer {
            inner: ds18b20::linux::Thermometer::at(path),
        }
    }

    /// Every DS18B20 the kernel has found, one per `28-` directory under `devices`, which is
    /// `/sys/bus/w1/devices` unless given. Raises `PamojaError` when the directory cannot be
    /// listed, which usually means the 1-Wire overlay is off.
    #[staticmethod]
    #[pyo3(signature = (devices = None))]
    fn discover(devices: Option<String>) -> PyResult<Vec<Ds18b20Thermometer>> {
        let devices = devices.unwrap_or_else(|| ds18b20::linux::DEVICES.to_owned());
        ds18b20::linux::Thermometer::discover_in(Path::new(&devices))
            .map(|found| {
                found
                    .into_iter()
                    .map(|inner| Ds18b20Thermometer { inner })
                    .collect()
            })
            .map_err(|error| PamojaError::new_err(format!("{devices}: {error}")))
    }

    /// The path of the file the thermometer reads.
    #[getter]
    fn path(&self) -> String {
        self.inner.path().to_string_lossy().into_owned()
    }

    /// Reads the file, which makes the kernel run a conversion, and returns the decoded
    /// reading. Raises `PamojaError` when the file cannot be read or the kernel or this decoder
    /// rejects the checksum.
    fn read(&self, py: Python<'_>) -> PyResult<Ds18b20Reading> {
        py.detach(|| self.inner.read_scratchpad())
            .map(Ds18b20Reading::from)
            .map_err(|error| PamojaError::new_err(error.to_string()))
    }
}

/// A simulated BME280 holding a real part's calibration and one measurement it took, which
/// compensate to 20.44 C, 848.05 hPa, and 44.65 %.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bme280_sim_part(address: u8) -> I2cPart {
    I2cPart {
        inner: bme280::sim::part(address),
    }
}

/// A simulated BME280 that reads what it is asked to, to within what its converter can
/// represent.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bme280_sim_reporting(
    address: u8,
    celsius: f32,
    hectopascals: f32,
    relative_humidity: f32,
) -> I2cPart {
    I2cPart {
        inner: bme280::sim::reporting(address, celsius, hectopascals, relative_humidity),
    }
}

/// The 26-byte temperature and pressure calibration block a simulated BME280 holds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bme280_sim_calibration(py: Python<'_>) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &bme280::sim::CALIBRATION)
}

/// The 7-byte humidity calibration block a simulated BME280 holds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bme280_sim_calibration_humidity(py: Python<'_>) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &bme280::sim::CALIBRATION_HUMIDITY)
}

/// The eight data registers a simulated BME280 holds: one measurement a real part took.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bme280_sim_burst(py: Python<'_>) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &bme280::sim::BURST)
}

/// The eight data registers that compensate to a reading against the simulated calibration.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bme280_sim_burst_for(
    py: Python<'_>,
    celsius: f32,
    hectopascals: f32,
    relative_humidity: f32,
) -> Bound<'_, PyBytes> {
    PyBytes::new(
        py,
        &bme280::sim::burst_for(celsius, hectopascals, relative_humidity),
    )
}

/// A simulated BMP280 holding a real part's trimming and one measurement it took, which
/// compensate to 20.44 C and 848.05 hPa.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_sim_part(address: u8) -> I2cPart {
    I2cPart {
        inner: bmp280::sim::part(address),
    }
}

/// A simulated BMP280 that reads what it is asked to, within a hundredth of a degree and of a
/// hectopascal.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_sim_reporting(address: u8, celsius: f32, hectopascals: f32) -> I2cPart {
    I2cPart {
        inner: bmp280::sim::reporting(address, celsius, hectopascals),
    }
}

/// The 24 trimming bytes a simulated BMP280 holds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_sim_calibration(py: Python<'_>) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &bmp280::sim::CALIBRATION)
}

/// The six data registers a simulated BMP280 holds: one measurement a real part took.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_sim_burst(py: Python<'_>) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &bmp280::sim::BURST)
}

/// The six data registers that compensate to a reading against the simulated trimming.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bmp280_sim_burst_for(py: Python<'_>, celsius: f32, hectopascals: f32) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &bmp280::sim::burst_for(celsius, hectopascals))
}

/// A simulated TMP117 reading 21.25 C, its configuration register keeping the flags the part
/// sets for itself, with the data-ready flag set.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_sim_part(address: u8) -> WordPart {
    WordPart {
        inner: tmp117::sim::part(address),
    }
}

/// A simulated TMP117 that reads what it is asked to, to the nearest 7.8125 millidegrees.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn tmp117_sim_reporting(address: u8, celsius: f32) -> WordPart {
    WordPart {
        inner: tmp117::sim::reporting(address, celsius),
    }
}

/// A simulated OPT3001 reading 380 lux, its conversion-ready flag set.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_sim_part(address: u8) -> WordPart {
    WordPart {
        inner: opt3001::sim::part(address),
    }
}

/// A simulated OPT3001 that reads what it is asked to, to the nearest step its exponent and
/// mantissa represent.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn opt3001_sim_reporting(address: u8, lux: f32) -> WordPart {
    WordPart {
        inner: opt3001::sim::reporting(address, lux),
    }
}

/// A simulated HDC1080 reading 22.5 C and 45 %.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_sim_part() -> WordPart {
    WordPart {
        inner: hdc1080::sim::part(),
    }
}

/// A simulated HDC1080 that reads what it is asked to, within three thousandths of a degree
/// and two thousandths of a percent.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hdc1080_sim_reporting(celsius: f32, relative_humidity: f32) -> WordPart {
    WordPart {
        inner: hdc1080::sim::reporting(celsius, relative_humidity),
    }
}

/// A simulated INA219 carrying 500 mA at 12 V through the 100 milliohm shunt a driver starts
/// with.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina219_sim_part(address: u8) -> WordPart {
    WordPart {
        inner: ina219::sim::part(address),
    }
}

/// A simulated INA219 that reads what it is asked to, calibrated for the same shunt and largest
/// current a driver is given.
#[gen_stub_pyfunction]
#[pyfunction]
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
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ina226_sim_part(address: u8) -> WordPart {
    WordPart {
        inner: ina226::sim::part(address),
    }
}

/// A simulated INA226 that reads what it is asked to, calibrated for the same shunt and largest
/// current a driver is given.
#[gen_stub_pyfunction]
#[pyfunction]
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
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ads1115_sim_part(address: u8) -> WordPart {
    WordPart {
        inner: ads1115::sim::part(address),
    }
}

/// A simulated ADS1115 that reads what it is asked to at the gain code a driver converts at.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn ads1115_sim_reporting(address: u8, pga: u8, volts: f32) -> WordPart {
    WordPart {
        inner: ads1115::sim::reporting(address, ads1115::Pga::from_code(pga), volts),
    }
}

/// A simulated SHT3x reading 22.5 C and 45 %.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_sim_part(address: u8) -> CommandPart {
    CommandPart {
        inner: sht3x::sim::part(address),
    }
}

/// A simulated SHT3x that reads what it is asked to, within three thousandths of a degree and
/// two thousandths of a percent.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sht3x_sim_reporting(address: u8, celsius: f32, relative_humidity: f32) -> CommandPart {
    CommandPart {
        inner: sht3x::sim::reporting(address, celsius, relative_humidity),
    }
}

/// A simulated SCD4x reading 800 ppm, 22.5 C, and 45 %, always with a result waiting.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_sim_part() -> CommandPart {
    CommandPart {
        inner: scd4x::sim::part(),
    }
}

/// A simulated SCD4x that reads what it is asked to.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn scd4x_sim_reporting(co2_ppm: u16, celsius: f32, relative_humidity: f32) -> CommandPart {
    CommandPart {
        inner: scd4x::sim::reporting(co2_ppm, celsius, relative_humidity),
    }
}

/// An ADS1115 sample as Python holds it.
fn sample_of(sample: ads1115::Sample) -> Ads1115Sample {
    Ads1115Sample {
        raw: sample.raw,
        pga: sample.pga.code(),
        nanovolts: sample.nanovolts(),
        volts: sample.volts(),
    }
}

/// Reads something a driver holds without touching the bus.
fn held<D, T>(inner: &Mutex<D>, read: impl FnOnce(&D) -> T) -> T {
    let driver = inner.lock().unwrap_or_else(PoisonError::into_inner);
    read(&driver)
}

/// Runs a driver call with the interpreter released.
fn drive<D: Send, T: Send>(
    inner: &Mutex<D>,
    py: Python<'_>,
    call: impl FnOnce(&mut D) -> Result<T, DriverError<BusError>> + Send,
) -> PyResult<T> {
    py.detach(|| {
        let mut driver = inner.lock().unwrap_or_else(PoisonError::into_inner);
        call(&mut driver)
    })
    .map_err(driver_error)
}

/// A driver failure as a Python exception, in the bus's own words when the bus was the cause.
fn driver_error(error: DriverError<BusError>) -> PyErr {
    let reason = match error {
        DriverError::Bus(error) => error.to_string(),
        DriverError::Sensor(error) => error.to_string(),
        DriverError::Timeout => DriverError::<BusError>::Timeout.to_string(),
    };
    PamojaError::new_err(reason)
}
