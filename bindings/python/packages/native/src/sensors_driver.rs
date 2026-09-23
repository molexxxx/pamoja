//! Generated Python bindings for the sensor drivers that run over an I2C bus.
//!
//! A driver runs the datasheet's whole conversation with a part over an `I2cBus`: reset,
//! identify, read the calibration, configure, measure. It holds its own share of the bus, and
//! each call releases the interpreter while the part answers, since a measurement waits the
//! datasheet's time before it reads. The simulated parts answer a driver with nothing plugged
//! in.

use std::sync::{Mutex, PoisonError};

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_hal::bus::{BusDelay, BusError, I2cBus as Bus};
use pamoja_sensors::bme280::{self, sim};
use pamoja_sensors::driver::I2cRegisters;
use pamoja_sensors::DriverError;

use crate::hal::{I2cBus, I2cPart};
use crate::sensors::Bme280Measurement;
use crate::PamojaError;

type Driver = bme280::Bme280<I2cRegisters<Bus>, BusDelay>;

/// A Bosch BME280 driven over an I2C bus, measuring on demand in forced mode.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct Bme280 {
    inner: Mutex<Driver>,
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
        self.drive(py, |driver| driver.init())
    }

    /// Runs one forced measurement and returns the compensated reading, initializing the part
    /// first if `init` has not run.
    fn measure(&self, py: Python<'_>) -> PyResult<Bme280Measurement> {
        self.drive(py, |driver| driver.measure())
            .map(Bme280Measurement::from)
    }
}

impl Bme280 {
    fn drive<T: Send>(
        &self,
        py: Python<'_>,
        call: impl FnOnce(&mut Driver) -> Result<T, DriverError<BusError>> + Send,
    ) -> PyResult<T> {
        py.detach(|| {
            let mut driver = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            call(&mut driver)
        })
        .map_err(driver_error)
    }
}

/// A simulated BME280 holding a real part's calibration and one measurement it took, which
/// compensate to 20.44 C, 848.05 hPa, and 44.65 %.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bme280_sim_part(address: u8) -> I2cPart {
    I2cPart {
        inner: sim::part(address),
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
        inner: sim::reporting(address, celsius, hectopascals, relative_humidity),
    }
}

/// The 26-byte temperature and pressure calibration block a simulated BME280 holds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bme280_sim_calibration(py: Python<'_>) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &sim::CALIBRATION)
}

/// The 7-byte humidity calibration block a simulated BME280 holds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bme280_sim_calibration_humidity(py: Python<'_>) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &sim::CALIBRATION_HUMIDITY)
}

/// The eight data registers a simulated BME280 holds: one measurement a real part took.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn bme280_sim_burst(py: Python<'_>) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &sim::BURST)
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
        &sim::burst_for(celsius, hectopascals, relative_humidity),
    )
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
