//! Python bindings for the actuator drivers that run over an I2C bus.
//!
//! A `Pca9685` drives the part's sixteen PWM channels over an `I2cBus`: it programs the
//! prescale for its frequency with the oscillator asleep, wakes it, waits the 500 us it
//! needs, and loads a channel's four registers in one transfer. Every call releases the
//! interpreter while the bus is busy. `pca9685_sim_part` is a PCA9685 that is not there,
//! keeping its datasheet's rules, so what a driver wrote reads back off a simulated bus.

use std::sync::{Mutex, PoisonError};

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_actuators::pca9685::{self, Outputs, Pwm};
use pamoja_actuators::DriverError;
use pamoja_hal::bus::{BusDelay, BusError, I2cBus as Bus};

use crate::hal::{I2cBus, I2cPart};
use crate::PamojaError;

type Pca9685Driver = pca9685::Pca9685<Bus, BusDelay>;

/// An NXP PCA9685 on an I2C bus, driving sixteen PWM outputs.
#[gen_stub_pyclass]
#[pyclass]
pub struct Pca9685 {
    inner: Mutex<Pca9685Driver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Pca9685 {
    /// A driver for the part at `address` on `bus`, at the part's own 200 Hz on its internal
    /// oscillator with totem-pole outputs unless given otherwise. Nothing is sent until `init`
    /// or the first channel is loaded.
    #[new]
    #[pyo3(signature = (
        bus,
        address,
        frequency_hz = 200,
        oscillator_hz = 25_000_000,
        totem_pole = true,
        inverted = false,
        change_on_ack = false
    ))]
    fn new(
        bus: PyRef<'_, I2cBus>,
        address: u8,
        frequency_hz: u32,
        oscillator_hz: u32,
        totem_pole: bool,
        inverted: bool,
        change_on_ack: bool,
    ) -> Self {
        let bus = bus.inner.clone();
        let delay = bus.delay();
        let driver = pca9685::Pca9685::new(bus, address, delay)
            .with_frequency(frequency_hz)
            .with_oscillator(oscillator_hz)
            .with_outputs(Outputs {
                totem_pole,
                inverted,
                change_on_ack,
            });
        Pca9685 {
            inner: Mutex::new(driver),
        }
    }

    /// Programs the prescale and the output wiring with the oscillator asleep, wakes it,
    /// waits the 500 us it needs, and restarts the channels.
    fn init(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.init())
    }

    /// Loads one channel with the four register bytes the `pwm` builders make, initializing
    /// the part first if `init` has not run. Raises for a channel past 15.
    fn set_channel(&self, py: Python<'_>, channel: u8, pwm: Vec<u8>) -> PyResult<()> {
        let pwm = pwm_of(&pwm)?;
        drive(&self.inner, py, move |driver| {
            driver.set_channel(channel, pwm)
        })
    }

    /// Reads one channel's four register bytes back from the part, one register a
    /// transfer, so the read works whatever MODE1 holds and changes nothing on the part.
    fn channel<'py>(&self, py: Python<'py>, channel: u8) -> PyResult<Bound<'py, PyBytes>> {
        let pwm = drive(&self.inner, py, move |driver| driver.channel(channel))?;
        Ok(PyBytes::new(py, &pwm.bytes()))
    }

    /// Loads every channel with the same four bytes in one transfer.
    fn set_all(&self, py: Python<'_>, pwm: Vec<u8>) -> PyResult<()> {
        let pwm = pwm_of(&pwm)?;
        drive(&self.inner, py, move |driver| driver.set_all(pwm))
    }

    /// Stops the oscillator; the channels keep their settings.
    fn sleep(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.sleep())
    }

    /// Wakes the oscillator, waits the 500 us it needs, and restarts the channels that were
    /// running before the sleep.
    fn wake(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.wake())
    }

    /// Sends the general-call software reset, which returns every PCA9685 on the bus to its
    /// power-on state. It goes to address 0x00, which nothing on a simulated bus answers.
    fn software_reset(&self, py: Python<'_>) -> PyResult<()> {
        drive(&self.inner, py, |driver| driver.software_reset())
    }

    /// The prescale value the driver writes for its frequency.
    #[getter]
    fn prescale(&self) -> u8 {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .prescale()
    }

    /// The frequency the part runs at once the prescaler has rounded the one asked for, in
    /// hertz.
    #[getter]
    fn frequency(&self) -> f32 {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .frequency()
    }
}

/// A simulated PCA9685 as it powers up: asleep at 200 Hz with every output off, keeping its
/// datasheet's rules for writes, reads, and its register pointer.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pca9685_sim_part(address: u8) -> I2cPart {
    I2cPart {
        inner: pca9685::sim::part(address),
    }
}

fn pwm_of(bytes: &[u8]) -> PyResult<Pwm> {
    let registers: [u8; 4] = bytes
        .try_into()
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("pwm must be exactly 4 bytes"))?;
    Ok(Pwm::from_bytes(&registers))
}

/// Runs a driver call with the interpreter released.
fn drive<T: Send>(
    inner: &Mutex<Pca9685Driver>,
    py: Python<'_>,
    call: impl FnOnce(&mut Pca9685Driver) -> Result<T, DriverError<BusError>> + Send,
) -> PyResult<T> {
    py.detach(|| {
        let mut driver = inner.lock().unwrap_or_else(PoisonError::into_inner);
        call(&mut driver)
    })
    .map_err(|error| match error {
        DriverError::Bus(error) => PamojaError::new_err(error.to_string()),
        DriverError::Command(reason) => pyo3::exceptions::PyValueError::new_err(reason),
    })
}
