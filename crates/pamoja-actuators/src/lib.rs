#![cfg_attr(not(feature = "std"), no_std)]

//! Concrete actuator drivers for the pamoja SDK.
//!
//! Where [`pamoja-sensors`](https://docs.rs/pamoja-sensors) decodes what a part
//! reports, this crate encodes what a part should do: it turns a desired output (a
//! PWM frequency and duty, a servo angle, a motor step) into the exact register bytes
//! or coil pattern a driver writes to the hardware. Each module is two layers. The
//! encode layer is pure logic with no I/O, so it runs on a microcontroller, on a
//! gateway, and in a test with nothing plugged in. The driver layer (the
//! `embedded-hal` feature, on by default) is a type named after the part that owns
//! a bus or a set of pins from [`pamoja-hal`](https://docs.rs/pamoja-hal), performs
//! the datasheet's transfer sequence, and implements the core
//! [`Actuator`](https://docs.rs/pamoja-core/latest/pamoja_core/trait.Actuator.html)
//! trait, so a `Pca9685` drives its channels through any `embedded-hal` I2C
//! implementation and a stepper steps through any four output pins. Every driver is
//! tested against its datasheet's own sequence over a scripted bus, and a driver's
//! error converts into the core error so the part slots into a profile or a node
//! unchanged.
//!
//! - [`pca9685`] - the NXP PCA9685 16-channel 12-bit PWM controller, the common way
//!   to drive servos, dimmable LEDs, and motor-driver inputs over I2C. Its register
//!   map, prescale formula, and channel words follow the datasheet.
//! - [`stepper`] - coil sequencing for four-wire stepper motors: the wave, full-step,
//!   and half-step drive patterns, plus a step-and-direction position model for
//!   driver chips that take a step pulse and a direction level.
//!
//! Simple on/off actuators (relays, solenoid valves, a pump switched through a
//! transistor) need no driver of their own: they are a GPIO line, modeled by the pin
//! and logic-level types in [`pamoja-gpio`](https://docs.rs/pamoja-gpio).
//!
//! # Examples
//!
//! The encode layer works out what a part is to be told, with nothing plugged in: the
//! prescale that runs a PCA9685 at the 50 Hz a servo wants, the pulse that centers the
//! servo, and the steps a geared stepper takes to swing a greenhouse vent a quarter turn.
//!
//! ```
//! use pamoja_actuators::pca9685::{
//!     frequency_for_prescale, prescale_for_frequency, Pwm, INTERNAL_OSC_HZ,
//! };
//! use pamoja_actuators::stepper::{steps_for_degrees, Direction, Drive, Sequencer};
//!
//! // The PCA9685 divides its 25 MHz clock by a prescale, so 50 Hz is as close as it gets.
//! let prescale = prescale_for_frequency(50, INTERNAL_OSC_HZ);
//! assert!((frequency_for_prescale(prescale, INTERNAL_OSC_HZ) - 50.0).abs() < 0.5);
//!
//! // A 1.5 ms pulse in each 20 ms period centers a hobby servo: 307 of the 4096 counts.
//! let center = Pwm::servo(1_500, 50);
//! assert_eq!(center.off() - center.on(), 307);
//!
//! // A stepper geared to 2048 full steps a turn needs 512 for the quarter, a whole number
//! // of coil cycles, so the coils end where they started.
//! let steps = steps_for_degrees(90.0, 2_048);
//! assert_eq!(steps, 512);
//! let mut coils = Sequencer::new(Drive::FullStep);
//! for _ in 0..steps {
//!     coils.step(Direction::Forward);
//! }
//! assert_eq!(coils.coils(), Drive::FullStep.pattern()[0]);
//! ```

#[cfg(feature = "embedded-hal")]
extern crate alloc;

#[cfg(feature = "embedded-hal")]
mod error;
pub mod pca9685;
pub mod stepper;

#[cfg(feature = "embedded-hal")]
pub use error::DriverError;
