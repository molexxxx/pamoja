//! Generated Node bindings for the sensor drivers that run over an I2C bus.
//!
//! A driver runs the datasheet's whole conversation with a part over an `I2cBus`: reset,
//! identify, read the calibration, configure, measure. It holds its own share of the bus, and
//! each call runs on a worker thread and resolves when the part has answered, since a
//! measurement waits the datasheet's time before it reads. The simulated parts answer a driver
//! with nothing plugged in.

use std::sync::{Arc, Mutex, PoisonError};

use napi::bindgen_prelude::{spawn_blocking, Buffer};
use napi_derive::napi;
use pamoja_hal::bus::{BusDelay, BusError, I2cBus as Bus};
use pamoja_sensors::bme280::{self, sim};
use pamoja_sensors::driver::I2cRegisters;
use pamoja_sensors::DriverError;

use crate::hal::{I2cBus, I2cPart};
use crate::sensors::Bme280Measurement;

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

type Driver = bme280::Bme280<I2cRegisters<Bus>, BusDelay>;

/// A Bosch BME280 driven over an I2C bus, measuring on demand in forced mode.
#[napi]
pub struct Bme280 {
    inner: Arc<Mutex<Driver>>,
}

#[napi]
impl Bme280 {
    /// A driver for the part at `address` on `bus`. Nothing is sent until `init` or the first
    /// `measure`.
    #[napi(constructor)]
    pub fn new(bus: &I2cBus, address: u8, settings: Option<Bme280Settings>) -> Self {
        let x1 = bme280::Oversampling::X1.code();
        let (temperature, pressure, humidity, filter) = match settings {
            Some(settings) => (
                settings.temperature.unwrap_or(x1),
                settings.pressure.unwrap_or(x1),
                settings.humidity.unwrap_or(x1),
                settings.filter.unwrap_or(bme280::Filter::Off.code()),
            ),
            None => (x1, x1, x1, bme280::Filter::Off.code()),
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
            inner: Arc::new(Mutex::new(driver)),
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

/// A simulated BME280 holding a real part's calibration and one measurement it took, which
/// compensate to 20.44 C, 848.05 hPa, and 44.65 %.
#[napi]
pub fn bme280_sim_part(address: u8) -> I2cPart {
    I2cPart {
        inner: sim::part(address),
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
        inner: sim::reporting(
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
    Buffer::from(sim::CALIBRATION.to_vec())
}

/// The 7-byte humidity calibration block a simulated BME280 holds.
#[napi]
pub fn bme280_sim_calibration_humidity() -> Buffer {
    Buffer::from(sim::CALIBRATION_HUMIDITY.to_vec())
}

/// The eight data registers a simulated BME280 holds: one measurement a real part took.
#[napi]
pub fn bme280_sim_burst() -> Buffer {
    Buffer::from(sim::BURST.to_vec())
}

/// The eight data registers that compensate to a reading against the simulated calibration.
#[napi]
pub fn bme280_sim_burst_for(celsius: f64, hectopascals: f64, relative_humidity: f64) -> Buffer {
    Buffer::from(
        sim::burst_for(
            celsius as f32,
            hectopascals as f32,
            relative_humidity as f32,
        )
        .to_vec(),
    )
}

/// Runs a driver call on a blocking worker thread.
async fn drive<T: Send + 'static>(
    inner: &Arc<Mutex<Driver>>,
    call: impl FnOnce(&mut Driver) -> Result<T, DriverError<BusError>> + Send + 'static,
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
