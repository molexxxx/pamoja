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

#[cfg(feature = "embedded-hal")]
extern crate alloc;

#[cfg(feature = "embedded-hal")]
mod error;
pub mod pca9685;
pub mod stepper;

#[cfg(feature = "embedded-hal")]
pub use error::DriverError;
