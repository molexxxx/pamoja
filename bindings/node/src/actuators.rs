//! Generated Node bindings for the actuator drivers.
//!
//! These mirror the `pamoja-actuators` Rust API: the command-encode half of a
//! PCA9685 PWM controller and of a stepper motor, turning a desired output into
//! the bytes and coil patterns a driver applies.
//!
//! A PWM setting is the four register bytes for one channel, so it comes back as
//! a buffer ready to write. A stepper carries its position across calls, so it is
//! a class.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_actuators::{pca9685, stepper};

/// A stepper drive pattern, trading torque, smoothness, and resolution.
#[napi(string_enum)]
pub enum StepDrive {
    /// One coil energised at a time: four steps, least torque and least power.
    Wave,
    /// Two adjacent coils at a time: four steps, most torque.
    FullStep,
    /// Alternating one and two coils: eight steps, double resolution.
    HalfStep,
}

/// Which way to step a motor.
#[napi(string_enum)]
pub enum StepDirection {
    /// Advance the sequence, turning the shaft one way.
    Forward,
    /// Reverse the sequence, turning the shaft the other way.
    Backward,
}

/// The PCA9685's internal oscillator frequency, in hertz.
#[napi]
pub const PCA9685_INTERNAL_OSC_HZ: u32 = pca9685::INTERNAL_OSC_HZ;

/// How many PWM channels a PCA9685 drives.
#[napi]
pub const PCA9685_CHANNELS: u8 = pca9685::CHANNELS;

/// How many counts a PCA9685 period is divided into.
#[napi]
pub const PCA9685_COUNTS: u16 = pca9685::COUNTS;

/// Returns the first of a PCA9685 channel's four consecutive registers.
#[napi]
pub fn pca9685_channel_register(channel: u8) -> napi::Result<u8> {
    if channel >= pca9685::CHANNELS {
        return Err(napi::Error::from_reason(format!(
            "channel must be below {}",
            pca9685::CHANNELS
        )));
    }
    Ok(pca9685::channel_register(channel))
}

/// Returns the prescale value that sets a PCA9685 update rate.
#[napi]
pub fn pca9685_prescale_for_frequency(update_rate_hz: u32, osc_hz: u32) -> u8 {
    pca9685::prescale_for_frequency(update_rate_hz, osc_hz)
}

/// Returns the update rate a PCA9685 prescale value produces, in hertz.
#[napi]
pub fn pca9685_frequency_for_prescale(prescale: u8, osc_hz: u32) -> f64 {
    f64::from(pca9685::frequency_for_prescale(prescale, osc_hz))
}

/// Builds a channel's four register bytes from explicit on and off counts.
#[napi]
pub fn pwm_from_counts(on: u16, off: u16) -> Buffer {
    pca9685::Pwm::from_counts(on, off).bytes().to_vec().into()
}

/// Builds a channel's register bytes with no phase delay: on at 0, off at `off`.
#[napi]
pub fn pwm_duty(off: u16) -> Buffer {
    pca9685::Pwm::duty(off).bytes().to_vec().into()
}

/// Builds the register bytes that drive a hobby servo to a given pulse width.
///
/// Typical travel is about 1000 to 2000 microseconds at a 50 Hz update rate.
#[napi]
pub fn pwm_servo(pulse_micros: u32, update_rate_hz: u32) -> Buffer {
    pca9685::Pwm::servo(pulse_micros, update_rate_hz)
        .bytes()
        .to_vec()
        .into()
}

/// The register bytes that hold a channel continuously high.
#[napi]
pub fn pwm_full_on() -> Buffer {
    pca9685::Pwm::full_on().bytes().to_vec().into()
}

/// The register bytes that hold a channel continuously low, the power-on state.
#[napi]
pub fn pwm_full_off() -> Buffer {
    pca9685::Pwm::full_off().bytes().to_vec().into()
}

/// Returns how many steps make up one electrical cycle of a drive pattern.
#[napi]
pub fn stepper_step_count(drive: StepDrive) -> u32 {
    stepper::Drive::from(drive).step_count() as u32
}

/// Returns how many steps a rotation of `degrees` takes on a given motor.
#[napi]
pub fn stepper_steps_for_degrees(degrees: f64, steps_per_revolution: u32) -> i32 {
    stepper::steps_for_degrees(degrees as f32, steps_per_revolution)
}

/// A stepper motor's place in its drive sequence, and how far it has turned.
#[napi]
pub struct Stepper {
    sequencer: stepper::Sequencer,
    position: stepper::Position,
}

#[napi]
impl Stepper {
    /// Creates a stepper at the start of a drive pattern, with its position at zero.
    #[napi(constructor)]
    pub fn new(drive: StepDrive) -> Self {
        Self {
            sequencer: stepper::Sequencer::new(drive.into()),
            position: stepper::Position::new(),
        }
    }

    /// Advances one step and returns the four-bit coil pattern to apply.
    ///
    /// The most significant of the four bits is the first coil.
    #[napi]
    pub fn step(&mut self, direction: StepDirection) -> u8 {
        let direction = stepper::Direction::from(direction);
        self.position.step(direction);
        self.sequencer.step(direction)
    }

    /// The coil pattern currently held, without advancing.
    #[napi(getter)]
    pub fn coils(&self) -> u8 {
        self.sequencer.coils()
    }

    /// How many steps have been taken, signed by direction.
    #[napi(getter)]
    pub fn steps(&self) -> i32 {
        self.position.steps()
    }
}

impl From<StepDrive> for stepper::Drive {
    fn from(value: StepDrive) -> Self {
        match value {
            StepDrive::Wave => stepper::Drive::Wave,
            StepDrive::FullStep => stepper::Drive::FullStep,
            StepDrive::HalfStep => stepper::Drive::HalfStep,
        }
    }
}

impl From<StepDirection> for stepper::Direction {
    fn from(value: StepDirection) -> Self {
        match value {
            StepDirection::Forward => stepper::Direction::Forward,
            StepDirection::Backward => stepper::Direction::Backward,
        }
    }
}
