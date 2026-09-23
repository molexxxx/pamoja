//! The C ABI for the actuator drivers.
//!
//! These functions wrap [`pamoja_actuators`] for callers that reach the SDK
//! through the flat C boundary: the command-encode half of a PCA9685 PWM
//! controller and of a stepper motor, turning a desired output into the bytes and
//! coil patterns a driver applies.
//!
//! A PWM setting is four register bytes, so it crosses by value as a
//! `#[repr(C)]` struct the caller writes straight to the channel's registers. A
//! stepper sequencer and position both carry state across calls, so they cross as
//! handles.

use pamoja_actuators::{pca9685, stepper};

use crate::{set_last_error, PamojaStatus};

/// The PCA9685's internal oscillator frequency, in hertz.
pub const PAMOJA_PCA9685_INTERNAL_OSC_HZ: u32 = 25_000_000;

/// How many PWM channels a PCA9685 drives.
pub const PAMOJA_PCA9685_CHANNELS: u8 = 16;

/// How many counts a PCA9685 period is divided into.
pub const PAMOJA_PCA9685_COUNTS: u16 = 4096;

/// The address a PCA9685 answers at with its six address pins low.
pub const PAMOJA_PCA9685_DEFAULT_ADDRESS: u8 = 0x40;

/// How long the oscillator takes to run once woken, in microseconds.
pub const PAMOJA_PCA9685_OSCILLATOR_STARTUP_MICROS: u32 = 500;

/// Mode register 1: restart, clock, auto-increment, sleep, and the addresses answered.
pub const PAMOJA_PCA9685_REGISTER_MODE1: u8 = 0x00;
/// Mode register 2: how the outputs are wired and when they change.
pub const PAMOJA_PCA9685_REGISTER_MODE2: u8 = 0x01;
/// The first of channel 0's four registers; channel n starts four registers on per channel.
pub const PAMOJA_PCA9685_REGISTER_LED0_ON_L: u8 = 0x06;
/// The first of the four ALL_LED registers, which load every channel at once.
pub const PAMOJA_PCA9685_REGISTER_ALL_LED_ON_L: u8 = 0xFA;
/// The prescaler that sets the PWM frequency, writable only while the part sleeps.
pub const PAMOJA_PCA9685_REGISTER_PRE_SCALE: u8 = 0xFE;

/// MODE1's RESTART bit: set when the part slept with a channel running, cleared by a written 1.
pub const PAMOJA_PCA9685_MODE1_RESTART: u8 = 0x80;
/// MODE1's EXTCLK bit: the prescaler divides the EXTCLK pin rather than the oscillator.
pub const PAMOJA_PCA9685_MODE1_EXTCLK: u8 = 0x40;
/// MODE1's auto-increment bit: the register pointer moves on after each byte.
pub const PAMOJA_PCA9685_MODE1_AUTO_INCREMENT: u8 = 0x20;
/// MODE1's SLEEP bit: the oscillator is off and PRE_SCALE takes a write.
pub const PAMOJA_PCA9685_MODE1_SLEEP: u8 = 0x10;

/// The power-on value of MODE1: asleep, answering the All Call address.
pub const PAMOJA_PCA9685_MODE1_RESET: u8 = 0x11;
/// The power-on value of MODE2: totem-pole outputs.
pub const PAMOJA_PCA9685_MODE2_RESET: u8 = 0x04;
/// The power-on value of PRE_SCALE: 200 Hz on the internal oscillator.
pub const PAMOJA_PCA9685_PRE_SCALE_RESET: u8 = 0x1E;
/// The smallest value the part loads into PRE_SCALE, about 1526 Hz.
pub const PAMOJA_PCA9685_PRE_SCALE_MIN: u8 = 3;

// The header generator does not read the crates this one depends on, so these
// carry their value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_PCA9685_INTERNAL_OSC_HZ == pca9685::INTERNAL_OSC_HZ);
const _: () = assert!(PAMOJA_PCA9685_CHANNELS == pca9685::CHANNELS);
const _: () = assert!(PAMOJA_PCA9685_COUNTS == pca9685::COUNTS);
const _: () = assert!(PAMOJA_PCA9685_DEFAULT_ADDRESS == pca9685::DEFAULT_I2C_ADDRESS);
const _: () =
    assert!(PAMOJA_PCA9685_OSCILLATOR_STARTUP_MICROS == pca9685::OSCILLATOR_STARTUP_MICROS);
const _: () = assert!(PAMOJA_PCA9685_REGISTER_MODE1 == pca9685::register::MODE1);
const _: () = assert!(PAMOJA_PCA9685_REGISTER_MODE2 == pca9685::register::MODE2);
const _: () = assert!(PAMOJA_PCA9685_REGISTER_LED0_ON_L == pca9685::register::LED0_ON_L);
const _: () = assert!(PAMOJA_PCA9685_REGISTER_ALL_LED_ON_L == pca9685::register::ALL_LED_ON_L);
const _: () = assert!(PAMOJA_PCA9685_REGISTER_PRE_SCALE == pca9685::register::PRE_SCALE);
const _: () = assert!(PAMOJA_PCA9685_MODE1_RESTART == pca9685::mode1::RESTART);
const _: () = assert!(PAMOJA_PCA9685_MODE1_EXTCLK == pca9685::mode1::EXTCLK);
const _: () = assert!(PAMOJA_PCA9685_MODE1_AUTO_INCREMENT == pca9685::mode1::AUTO_INCREMENT);
const _: () = assert!(PAMOJA_PCA9685_MODE1_SLEEP == pca9685::mode1::SLEEP);
const _: () = assert!(PAMOJA_PCA9685_MODE1_RESET == pca9685::MODE1_RESET);
const _: () = assert!(PAMOJA_PCA9685_MODE2_RESET == pca9685::MODE2_RESET);
const _: () = assert!(PAMOJA_PCA9685_PRE_SCALE_RESET == pca9685::PRE_SCALE_RESET);
const _: () = assert!(PAMOJA_PCA9685_PRE_SCALE_MIN == pca9685::PRE_SCALE_MIN);

/// A PCA9685 channel's four register bytes.
///
/// The order matches the channel's four consecutive registers, so the whole
/// struct can be written in one bus transaction.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaPwm {
    /// The low byte of the count at which the output goes high.
    pub on_low: u8,
    /// The high byte of that count; bit 4 is the full-on flag.
    pub on_high: u8,
    /// The low byte of the count at which the output goes low.
    pub off_low: u8,
    /// The high byte of that count; bit 4 is the full-off flag.
    pub off_high: u8,
}

/// Which way to step a motor.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaStepDirection {
    /// Advance the sequence, turning the shaft one way.
    Forward = 0,
    /// Reverse the sequence, turning the shaft the other way.
    Backward = 1,
}

/// A stepper drive pattern, trading torque, smoothness, and resolution.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaStepDrive {
    /// One coil energized at a time: four steps, least torque and least power.
    Wave = 0,
    /// Two adjacent coils at a time: four steps, most torque.
    FullStep = 1,
    /// Alternating one and two coils: eight steps, double resolution.
    HalfStep = 2,
}

/// An opaque handle to a position in a stepper drive sequence.
///
/// Release it with [`pamoja_stepper_free`].
pub struct PamojaStepper {
    sequencer: stepper::Sequencer,
    position: stepper::Position,
}

/// Returns the first of a PCA9685 channel's four consecutive registers.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_register` set, or
/// [`PamojaStatus::InvalidArgument`] if `channel` is 16 or above.
///
/// # Safety
///
/// `out_register` must point to a writable `uint8_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pca9685_channel_register(
    channel: u8,
    out_register: *mut u8,
) -> PamojaStatus {
    if out_register.is_null() {
        set_last_error("out_register must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    if channel >= pca9685::CHANNELS {
        set_last_error(format!("channel must be below {}", pca9685::CHANNELS));
        return PamojaStatus::InvalidArgument;
    }
    *out_register = pca9685::channel_register(channel);
    PamojaStatus::Ok
}

/// Returns the prescale value that sets a PCA9685 update rate.
///
/// # Returns
///
/// The prescale register value, clamped to what the part accepts.
#[no_mangle]
pub extern "C" fn pamoja_pca9685_prescale_for_frequency(update_rate_hz: u32, osc_hz: u32) -> u8 {
    pca9685::prescale_for_frequency(update_rate_hz, osc_hz)
}

/// Returns the update rate a PCA9685 prescale value produces.
///
/// # Returns
///
/// The frequency in hertz.
#[no_mangle]
pub extern "C" fn pamoja_pca9685_frequency_for_prescale(prescale: u8, osc_hz: u32) -> f32 {
    pca9685::frequency_for_prescale(prescale, osc_hz)
}

/// Builds a PWM setting from explicit on and off counts.
///
/// # Returns
///
/// The four register bytes; counts are masked to 12 bits.
#[no_mangle]
pub extern "C" fn pamoja_pwm_from_counts(on: u16, off: u16) -> PamojaPwm {
    pca9685::Pwm::from_counts(on, off).into()
}

/// Builds a PWM setting with no phase delay: on at count 0, off at `off`.
///
/// # Returns
///
/// The four register bytes.
#[no_mangle]
pub extern "C" fn pamoja_pwm_duty(off: u16) -> PamojaPwm {
    pca9685::Pwm::duty(off).into()
}

/// Builds the setting that drives a hobby servo to a given pulse width.
///
/// Typical travel is about 1000 to 2000 microseconds at a 50 Hz update rate.
///
/// # Returns
///
/// The four register bytes for that pulse width.
#[no_mangle]
pub extern "C" fn pamoja_pwm_servo(pulse_micros: u32, update_rate_hz: u32) -> PamojaPwm {
    pca9685::Pwm::servo(pulse_micros, update_rate_hz).into()
}

/// Reads a PCA9685 setting back from the four register bytes a channel holds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_on` and `*out_off` set to the counts
/// the registers hold, the full-on and full-off flags included.
///
/// # Safety
///
/// `out_on` and `out_off` must each point to a writable `uint16_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_pwm_counts(
    pwm: PamojaPwm,
    out_on: *mut u16,
    out_off: *mut u16,
) -> PamojaStatus {
    if out_on.is_null() || out_off.is_null() {
        set_last_error("out_on and out_off must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let setting = pca9685::Pwm::from_bytes(&[pwm.on_low, pwm.on_high, pwm.off_low, pwm.off_high]);
    *out_on = setting.on();
    *out_off = setting.off();
    PamojaStatus::Ok
}

/// The setting that holds a channel continuously high.
///
/// # Returns
///
/// The four register bytes.
#[no_mangle]
pub extern "C" fn pamoja_pwm_full_on() -> PamojaPwm {
    pca9685::Pwm::full_on().into()
}

/// The setting that holds a channel continuously low, the power-on state.
///
/// # Returns
///
/// The four register bytes.
#[no_mangle]
pub extern "C" fn pamoja_pwm_full_off() -> PamojaPwm {
    pca9685::Pwm::full_off().into()
}

/// Creates a stepper at the start of a drive pattern, with its position at zero.
///
/// # Returns
///
/// A new stepper the caller must release with [`pamoja_stepper_free`].
#[no_mangle]
pub extern "C" fn pamoja_stepper_new(drive: PamojaStepDrive) -> *mut PamojaStepper {
    Box::into_raw(Box::new(PamojaStepper {
        sequencer: stepper::Sequencer::new(drive.into()),
        position: stepper::Position::new(),
    }))
}

/// Advances a stepper one step and returns the coil pattern to apply.
///
/// # Returns
///
/// The four-bit coil pattern, or 0 if `stepper` is null. The most significant of
/// the four bits is the first coil.
///
/// # Safety
///
/// `stepper` must be a live handle from [`pamoja_stepper_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_stepper_step(
    stepper: *mut PamojaStepper,
    direction: PamojaStepDirection,
) -> u8 {
    if stepper.is_null() {
        return 0;
    }
    let stepper = &mut *stepper;
    stepper.position.step(direction.into());
    stepper.sequencer.step(direction.into())
}

/// Returns the coil pattern a stepper currently holds, without advancing it.
///
/// # Returns
///
/// The four-bit coil pattern, or 0 if `stepper` is null.
///
/// # Safety
///
/// `stepper` must be a live handle from [`pamoja_stepper_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_stepper_coils(stepper: *const PamojaStepper) -> u8 {
    if stepper.is_null() {
        return 0;
    }
    (*stepper).sequencer.coils()
}

/// Returns how many steps a stepper has taken, signed by direction.
///
/// # Returns
///
/// The net step count, or 0 if `stepper` is null.
///
/// # Safety
///
/// `stepper` must be a live handle from [`pamoja_stepper_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_stepper_steps(stepper: *const PamojaStepper) -> i32 {
    if stepper.is_null() {
        return 0;
    }
    (*stepper).position.steps()
}

/// Returns how many steps make up one electrical cycle of a drive pattern.
///
/// # Returns
///
/// `4` for wave and full-step, `8` for half-step.
#[no_mangle]
pub extern "C" fn pamoja_stepper_step_count(drive: PamojaStepDrive) -> usize {
    stepper::Drive::from(drive).step_count()
}

/// Releases a stepper handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `stepper` must be a handle from [`pamoja_stepper_new`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_stepper_free(stepper: *mut PamojaStepper) {
    if !stepper.is_null() {
        drop(Box::from_raw(stepper));
    }
}

/// Returns how many steps a rotation of `degrees` takes on a given motor.
///
/// # Returns
///
/// The step count, negative for a negative angle.
#[no_mangle]
pub extern "C" fn pamoja_stepper_steps_for_degrees(degrees: f32, steps_per_revolution: u32) -> i32 {
    stepper::steps_for_degrees(degrees, steps_per_revolution)
}

impl From<pca9685::Pwm> for PamojaPwm {
    fn from(value: pca9685::Pwm) -> Self {
        let [on_low, on_high, off_low, off_high] = value.bytes();
        PamojaPwm {
            on_low,
            on_high,
            off_low,
            off_high,
        }
    }
}

impl From<PamojaStepDirection> for stepper::Direction {
    fn from(value: PamojaStepDirection) -> Self {
        match value {
            PamojaStepDirection::Forward => stepper::Direction::Forward,
            PamojaStepDirection::Backward => stepper::Direction::Backward,
        }
    }
}

impl From<PamojaStepDrive> for stepper::Drive {
    fn from(value: PamojaStepDrive) -> Self {
        match value {
            PamojaStepDrive::Wave => stepper::Drive::Wave,
            PamojaStepDrive::FullStep => stepper::Drive::FullStep,
            PamojaStepDrive::HalfStep => stepper::Drive::HalfStep,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn a_half_brightness_channel_writes_its_midpoint() {
        assert_eq!(
            pamoja_pwm_duty(2048),
            PamojaPwm {
                on_low: 0x00,
                on_high: 0x00,
                off_low: 0x00,
                off_high: 0x08,
            }
        );
    }

    #[test]
    fn fully_off_is_its_own_encoding_not_a_zero_duty() {
        assert_eq!(
            pamoja_pwm_full_off(),
            PamojaPwm {
                on_low: 0x00,
                on_high: 0x00,
                off_low: 0x00,
                off_high: 0x10,
            },
            "a zero duty still glitches high for one count; the flag does not"
        );
        assert_eq!(pamoja_pwm_full_on().on_high, 0x10);
    }

    #[test]
    fn a_servo_pulse_scales_against_its_update_rate() {
        // A 1500 microsecond pulse at 50 Hz is the center of a hobby servo's travel.
        let center = pamoja_pwm_servo(1_500, 50);
        let counts = u16::from(center.off_low) | (u16::from(center.off_high) << 8);
        assert_eq!(counts, 307, "1500 us * 4096 * 50 / 1e6");
    }

    #[test]
    fn a_channel_beyond_the_part_is_refused() {
        let mut register = 0u8;
        // Safety: the out-pointer is writable.
        let status = unsafe { pamoja_pca9685_channel_register(16, &mut register) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
    }

    #[test]
    fn the_update_rate_round_trips_through_its_prescale() {
        let prescale = pamoja_pca9685_prescale_for_frequency(50, PAMOJA_PCA9685_INTERNAL_OSC_HZ);
        let frequency =
            pamoja_pca9685_frequency_for_prescale(prescale, PAMOJA_PCA9685_INTERNAL_OSC_HZ);
        assert!((frequency - 50.0).abs() < 1.0, "got {frequency} Hz");
    }

    #[test]
    fn a_full_electrical_cycle_returns_to_its_first_pattern() {
        let stepper = pamoja_stepper_new(PamojaStepDrive::HalfStep);
        // Safety: the stepper is live.
        unsafe {
            let first = pamoja_stepper_coils(stepper);
            for _ in 0..pamoja_stepper_step_count(PamojaStepDrive::HalfStep) {
                pamoja_stepper_step(stepper, PamojaStepDirection::Forward);
            }
            assert_eq!(pamoja_stepper_coils(stepper), first);
            assert_eq!(pamoja_stepper_steps(stepper), 8);
            pamoja_stepper_free(stepper);
        }
    }

    #[test]
    fn stepping_back_and_forth_returns_the_position_to_zero() {
        let stepper = pamoja_stepper_new(PamojaStepDrive::FullStep);
        // Safety: the stepper is live.
        unsafe {
            pamoja_stepper_step(stepper, PamojaStepDirection::Forward);
            pamoja_stepper_step(stepper, PamojaStepDirection::Backward);
            assert_eq!(pamoja_stepper_steps(stepper), 0);
            pamoja_stepper_free(stepper);
        }
    }

    #[test]
    fn calls_on_a_null_stepper_are_rejected_without_dereferencing() {
        // Safety: passing null is explicitly handled.
        unsafe {
            assert_eq!(
                pamoja_stepper_step(ptr::null_mut(), PamojaStepDirection::Forward),
                0
            );
            assert_eq!(pamoja_stepper_coils(ptr::null()), 0);
            assert_eq!(pamoja_stepper_steps(ptr::null()), 0);
            pamoja_stepper_free(ptr::null_mut());
        }
    }

    #[test]
    fn a_quarter_turn_is_a_quarter_of_the_revolution() {
        assert_eq!(pamoja_stepper_steps_for_degrees(90.0, 200), 50);
        assert_eq!(pamoja_stepper_steps_for_degrees(-90.0, 200), -50);
    }
}
