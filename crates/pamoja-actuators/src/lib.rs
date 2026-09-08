#![no_std]

//! Concrete actuator drivers for the pamoja SDK.
//!
//! Where [`pamoja-sensors`](https://docs.rs/pamoja-sensors) decodes what a part
//! reports, this crate encodes what a part should do: it turns a desired output (a
//! PWM frequency and duty, a servo angle, a motor step) into the exact register bytes
//! or coil pattern a driver writes to the hardware. Like the rest of the SDK's
//! hardware crates, it is the command half ahead of the actual bus driver, pure logic
//! with no I/O, so the same code runs on a microcontroller and in a test.
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

pub mod pca9685;
pub mod stepper;
