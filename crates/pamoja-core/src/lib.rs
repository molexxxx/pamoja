//! Core abstractions for the pamoja device SDK.
//!
//! This crate defines the traits that every capability crate (for example
//! `pamoja-mqtt`, `pamoja-serial`, or `pamoja-ros2`) implements. The
//! core is protocol-agnostic: it models devices, sensors, actuators, transports,
//! durable storage, and an event bus, and leaves concrete protocol support to the
//! capability crates so that an application depends only on what it uses.
//!
//! The primary abstractions are:
//!
//! - [`Device`] - a connectable physical or virtual device.
//! - [`Sensor`] - a source of typed readings.
//! - [`Actuator`] - a sink for typed commands.
//! - [`Telemetry`] - a stream of telemetry frames.
//! - [`Transport`] - the outbound half of a link: connect, send, subscribe.
//! - [`Receive`] - the inbound half: the messages a subscription delivers.
//! - [`Store`] - a durable store-and-forward queue.
//! - [`EventBus`] - a typed publish/subscribe channel.
//! - [`Map`] and [`MapCommand`] - adapters that reshape a reading or a command, so a
//!   multi-channel driver feeds a consumer that wants one number.
//! - [`Error`] and [`Result`] - the shared error model.
//!
//! # Examples
//!
//! Implementing [`Sensor`] for a temperature probe:
//!
//! ```
//! use pamoja_core::{Result, Sensor};
//!
//! struct Thermometer {
//!     celsius: f32,
//! }
//!
//! impl Sensor for Thermometer {
//!     type Reading = f32;
//!
//!     async fn read(&mut self) -> Result<Self::Reading> {
//!         Ok(self.celsius)
//!     }
//! }
//!
//! let _probe = Thermometer { celsius: 20.5 };
//! ```

// The core is `no_std` unless the default `std` feature is on, so it fits a
// microcontroller. The owned types it needs (`String`, `Vec`) come from `alloc`.
#![cfg_attr(not(feature = "std"), no_std)]
// The public traits use `async fn`, which is intentional for this statically
// dispatched SDK; the associated lint is therefore allowed crate-wide.
#![allow(async_fn_in_trait)]

extern crate alloc;

pub mod adapt;
pub mod bus;
pub mod device;
pub mod error;
pub mod store;
pub mod transport;

pub use adapt::{Map, MapCommand};
pub use bus::EventBus;
pub use device::{Actuator, Device, Sensor, Telemetry};
pub use error::{Error, Result};
pub use store::Store;
pub use transport::{topic_matches, Message, Receive, Transport};
