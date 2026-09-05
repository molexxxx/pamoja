//! Generated Python bindings for the actuator drivers.
//!
//! These mirror the `pamoja-actuators` Rust API: the command-encode half of a
//! PCA9685 PWM controller and of a stepper motor, turning a desired output into
//! the bytes and coil patterns a driver applies.
//!
//! A PWM setting is the four register bytes for one channel, so it comes back as
//! `bytes` ready to write. A stepper carries its position across calls, so it is
//! a class.
//!
//! The drive pattern and direction cross as plain strings, which the facade turns
//! back into Python enum members.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_actuators::{pca9685, stepper};

/// The PCA9685's internal oscillator frequency, in hertz.
const INTERNAL_OSC_HZ: u32 = pca9685::INTERNAL_OSC_HZ;

/// A stepper motor's place in its drive sequence, and how far it has turned.
#[gen_stub_pyclass]
#[pyclass]
pub struct Stepper {
    sequencer: stepper::Sequencer,
    position: stepper::Position,
}

#[gen_stub_pymethods]
#[pymethods]
impl Stepper {
    /// Creates a stepper at the start of a drive pattern, with its position at zero.
    #[new]
    fn new(drive: &str) -> PyResult<Self> {
        Ok(Self {
            sequencer: stepper::Sequencer::new(read_drive(drive)?),
            position: stepper::Position::new(),
        })
    }

    /// Advances one step and returns the four-bit coil pattern to apply.
    ///
    /// The most significant of the four bits is the first coil.
    fn step(&mut self, direction: &str) -> PyResult<u8> {
        let direction = read_direction(direction)?;
        self.position.step(direction);
        Ok(self.sequencer.step(direction))
    }

    /// The coil pattern currently held, without advancing.
    #[getter]
    fn coils(&self) -> u8 {
        self.sequencer.coils()
    }

    /// How many steps have been taken, signed by direction.
    #[getter]
    fn steps(&self) -> i32 {
        self.position.steps()
    }
}

/// Returns the PCA9685 constants a caller sizes its arithmetic against.
///
/// The tuple is the internal oscillator frequency in hertz, the channel count,
/// and the counts per period.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pca9685_limits() -> (u32, u8, u16) {
    (INTERNAL_OSC_HZ, pca9685::CHANNELS, pca9685::COUNTS)
}

/// Returns the first of a PCA9685 channel's four consecutive registers.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pca9685_channel_register(channel: u8) -> PyResult<u8> {
    if channel >= pca9685::CHANNELS {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "channel must be below {}",
            pca9685::CHANNELS
        )));
    }
    Ok(pca9685::channel_register(channel))
}

/// Returns the prescale value that sets a PCA9685 update rate.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pca9685_prescale_for_frequency(update_rate_hz: u32, osc_hz: u32) -> u8 {
    pca9685::prescale_for_frequency(update_rate_hz, osc_hz)
}

/// Returns the update rate a PCA9685 prescale value produces, in hertz.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pca9685_frequency_for_prescale(prescale: u8, osc_hz: u32) -> f32 {
    pca9685::frequency_for_prescale(prescale, osc_hz)
}

/// Builds a channel's four register bytes from explicit on and off counts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pwm_from_counts<'py>(py: Python<'py>, on: u16, off: u16) -> Bound<'py, PyBytes> {
    PyBytes::new(py, &pca9685::Pwm::from_counts(on, off).bytes())
}

/// Builds a channel's register bytes with no phase delay: on at 0, off at `off`.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pwm_duty<'py>(py: Python<'py>, off: u16) -> Bound<'py, PyBytes> {
    PyBytes::new(py, &pca9685::Pwm::duty(off).bytes())
}

/// Builds the register bytes that drive a hobby servo to a given pulse width.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pwm_servo<'py>(
    py: Python<'py>,
    pulse_micros: u32,
    update_rate_hz: u32,
) -> Bound<'py, PyBytes> {
    PyBytes::new(
        py,
        &pca9685::Pwm::servo(pulse_micros, update_rate_hz).bytes(),
    )
}

/// Reads a PCA9685 setting back from the four register bytes a channel holds.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pwm_counts(data: Vec<u8>) -> PyResult<(u16, u16)> {
    let registers: [u8; 4] = data
        .as_slice()
        .try_into()
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("pwm must be exactly 4 bytes"))?;
    let pwm = pca9685::Pwm::from_bytes(&registers);
    Ok((pwm.on(), pwm.off()))
}

/// The register bytes that hold a channel continuously high.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pwm_full_on(py: Python<'_>) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &pca9685::Pwm::full_on().bytes())
}

/// The register bytes that hold a channel continuously low, the power-on state.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pwm_full_off(py: Python<'_>) -> Bound<'_, PyBytes> {
    PyBytes::new(py, &pca9685::Pwm::full_off().bytes())
}

/// Returns how many steps make up one electrical cycle of a drive pattern.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn stepper_step_count(drive: &str) -> PyResult<usize> {
    Ok(read_drive(drive)?.step_count())
}

/// Returns how many steps a rotation of `degrees` takes on a given motor.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn stepper_steps_for_degrees(degrees: f32, steps_per_revolution: u32) -> i32 {
    stepper::steps_for_degrees(degrees, steps_per_revolution)
}

/// Reads a drive pattern back from its name.
fn read_drive(drive: &str) -> PyResult<stepper::Drive> {
    match drive {
        "Wave" => Ok(stepper::Drive::Wave),
        "FullStep" => Ok(stepper::Drive::FullStep),
        "HalfStep" => Ok(stepper::Drive::HalfStep),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "drive must be Wave, FullStep, or HalfStep",
        )),
    }
}

/// Reads a step direction back from its name.
fn read_direction(direction: &str) -> PyResult<stepper::Direction> {
    match direction {
        "Forward" => Ok(stepper::Direction::Forward),
        "Backward" => Ok(stepper::Direction::Backward),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "direction must be Forward or Backward",
        )),
    }
}
