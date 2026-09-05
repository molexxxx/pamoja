//! The whole pamoja device SDK in one crate.
//!
//! pamoja is one memory-safe Rust core with a crate per capability, so a build
//! carries only the crates it names. This crate is the other way in: every
//! capability sits behind a feature, all on by default, so `cargo add pamoja` is
//! the whole framework, the way `npm install pamoja`, `pip install pamoja`, and
//! `dotnet add package Pamoja` are in the bindings.
//!
//! Each module re-exports the crate of the same name: `pamoja::codec` is
//! `pamoja-codec`, `pamoja::mqtt` is `pamoja-mqtt`, and `pamoja::core` is
//! `pamoja-core`, the traits every capability implements. The types, the
//! documentation, and the examples are those of the crate, so code moves between
//! `use pamoja::codec::CborCodec` and `use pamoja_codec::CborCodec` with no other change.
//!
//! ```toml
//! [dependencies]
//! pamoja = "0.1"
//! ```
//!
//! A build that needs only some capabilities names them, and takes on only
//! their dependencies:
//!
//! ```toml
//! [dependencies]
//! pamoja = { version = "0.1", default-features = false, features = ["std", "codec", "security"] }
//! ```
//!
//! # Example
//!
//! A reading taken off a wire, smoothed, packed for a metered link, and signed so
//! the gateway that receives it can tell which device sent it, with nothing plugged
//! in:
//!
//! ```
//! use pamoja::codec::{decode_deltas, encode_deltas};
//! use pamoja::kit::Smoother;
//! use pamoja::security::{DeviceIdentity, PublicIdentity};
//! use pamoja::sensors::ds18b20::{temperature_from_celsius, Resolution, Scratchpad};
//!
//! // A stand-in for the thermometer. On a running node these nine bytes arrive from
//! // the 1-Wire bus; here the library builds what a part at 25.0625 C would send.
//! let off_the_bus = Scratchpad::new(
//!     temperature_from_celsius(25.0625, Resolution::Bits12),
//!     Resolution::Bits12,
//!     75,
//!     -10,
//! )
//! .to_bytes();
//!
//! // The part checksums every read, so a value mangled on a long run is an error
//! // rather than a plausible temperature a couple of degrees off.
//! let celsius = Scratchpad::parse(&off_the_bus)
//!     .expect("the checksum matches")
//!     .temperature_celsius();
//! assert_eq!(celsius, 25.0625);
//!
//! // Readings jitter, so smooth them and send a batch rather than one at a time.
//! let mut smoother = Smoother::new(0.5);
//! let batch: Vec<i64> = [celsius, celsius + 0.5, celsius + 0.4]
//!     .into_iter()
//!     .map(|sample| (smoother.update(sample) * 100.0).round() as i64)
//!     .collect();
//! let packed = encode_deltas(&batch);
//! assert!(packed.len() < batch.len() * 8);
//!
//! // Sign the batch. The signature travels with the payload as one message, so a
//! // gateway holding only the public key gets the payload back once it checks out.
//! let device = DeviceIdentity::from_seed(&[7u8; 32]);
//! let message = device.sign_message(&packed);
//!
//! let known = PublicIdentity::from_bytes(&device.public().to_bytes())?;
//! let payload = known.verify_message(&message)?;
//! assert_eq!(decode_deltas(payload).expect("a valid batch"), batch);
//! # Ok::<(), pamoja::core::Error>(())
//! ```
//!
//! # Features
//!
//! One feature per capability, named as its crate is without the prefix, and all
//! on by default:
//!
//! | Feature | Module | Crate |
//! | --- | --- | --- |
//! | (always) | `pamoja::core` | [pamoja-core](https://docs.rs/pamoja-core) |
//! | `security` | `pamoja::security` | [pamoja-security](https://docs.rs/pamoja-security) |
//! | `codec` | `pamoja::codec` | [pamoja-codec](https://docs.rs/pamoja-codec) |
//! | `kit` | `pamoja::kit` | [pamoja-kit](https://docs.rs/pamoja-kit) |
//! | `serial` | `pamoja::serial` | [pamoja-serial](https://docs.rs/pamoja-serial) |
//! | `modbus` | `pamoja::modbus` | [pamoja-modbus](https://docs.rs/pamoja-modbus) |
//! | `can` | `pamoja::can` | [pamoja-can](https://docs.rs/pamoja-can) |
//! | `gpio` | `pamoja::gpio` | [pamoja-gpio](https://docs.rs/pamoja-gpio) |
//! | `sensors` | `pamoja::sensors` | [pamoja-sensors](https://docs.rs/pamoja-sensors) |
//! | `actuators` | `pamoja::actuators` | [pamoja-actuators](https://docs.rs/pamoja-actuators) |
//! | `lora` | `pamoja::lora` | [pamoja-lora](https://docs.rs/pamoja-lora) |
//! | `lorawan` | `pamoja::lorawan` | [pamoja-lorawan](https://docs.rs/pamoja-lorawan) |
//! | `mesh` | `pamoja::mesh` | [pamoja-mesh](https://docs.rs/pamoja-mesh) |
//! | `routing` | `pamoja::routing` | [pamoja-routing](https://docs.rs/pamoja-routing) |
//! | `mavlink` | `pamoja::mavlink` | [pamoja-mavlink](https://docs.rs/pamoja-mavlink) |
//! | `audit` | `pamoja::audit` | [pamoja-audit](https://docs.rs/pamoja-audit) |
//! | `session` | `pamoja::session` | [pamoja-session](https://docs.rs/pamoja-session) |
//! | `update` | `pamoja::update` | [pamoja-update](https://docs.rs/pamoja-update) |
//! | `power` | `pamoja::power` | [pamoja-power](https://docs.rs/pamoja-power) |
//! | `telemetry` | `pamoja::telemetry` | [pamoja-telemetry](https://docs.rs/pamoja-telemetry) |
//! | `mqtt` | `pamoja::mqtt` | [pamoja-mqtt](https://docs.rs/pamoja-mqtt) |
//! | `coap` | `pamoja::coap` | [pamoja-coap](https://docs.rs/pamoja-coap) |
//! | `loopback` | `pamoja::loopback` | [pamoja-loopback](https://docs.rs/pamoja-loopback) |
//! | `sync` | `pamoja::sync` | [pamoja-sync](https://docs.rs/pamoja-sync) |
//! | `ladder` | `pamoja::ladder` | [pamoja-ladder](https://docs.rs/pamoja-ladder) |
//! | `bus` | `pamoja::bus` | [pamoja-bus](https://docs.rs/pamoja-bus) |
//! | `sim` | `pamoja::sim` | [pamoja-sim](https://docs.rs/pamoja-sim) |
//! | `profile` | `pamoja::profile` | [pamoja-profile](https://docs.rs/pamoja-profile) |
//! | `ros2` | `pamoja::ros2` | [pamoja-ros2](https://docs.rs/pamoja-ros2) |
//! | `zenoh` | `pamoja::zenoh` | [pamoja-zenoh](https://docs.rs/pamoja-zenoh) |
//! | `dashboard` (off by default) | `pamoja::dashboard` | [pamoja-dashboard](https://docs.rs/pamoja-dashboard) |
//!
//! Six of those capabilities' chapters hold more than one capability, and each has a
//! feature that turns on exactly its own, so a build can name a domain instead of listing
//! its parts. They are checked against the capability map, so a new capability cannot fall
//! out of its group:
//!
//! | Group feature | Turns on |
//! | --- | --- |
//! | `field-io` | `serial`, `modbus`, `can`, `gpio` |
//! | `sensing` | `sensors`, `actuators` |
//! | `radio` | `lora`, `lorawan`, `mesh`, `routing` |
//! | `trust` | `audit`, `session`, `update`, `power`, `telemetry` |
//! | `transports` | `mqtt`, `coap`, `loopback`, `sync`, `ladder`, `bus`, `sim` |
//! | `profiles` | `profile`, `ros2`, `zenoh` |
//!
//! ```toml
//! [dependencies]
//! pamoja = { version = "0.1", default-features = false, features = ["std", "field-io"] }
//! ```
//!
//! `std`, on by default, turns on the standard-library layer of the crates that
//! have one (`pamoja-core`, `pamoja-lora`, `pamoja-mavlink`) and implies `alloc`,
//! which adds the owned channel plans, tables, and message shapes of `pamoja-lora`,
//! `pamoja-mesh`, `pamoja-routing`, and `pamoja-mavlink`. With both off and only
//! `no_std` capabilities named, the crate builds for a bare-metal target; CI
//! compiles it for `thumbv7em-none-eabihf`. The crates keep their finer switches
//! (the LoRa region set, the kit's helper groups, the MAVLink serial driver), so
//! depend on the crate itself when you need one of those. `dashboard` adds the
//! fleet dashboard, a web server, and is off by default.

#![no_std]

pub use pamoja_core as core;

#[cfg(feature = "actuators")]
pub use pamoja_actuators as actuators;
#[cfg(feature = "audit")]
pub use pamoja_audit as audit;
#[cfg(feature = "bus")]
pub use pamoja_bus as bus;
#[cfg(feature = "can")]
pub use pamoja_can as can;
#[cfg(feature = "coap")]
pub use pamoja_coap as coap;
#[cfg(feature = "codec")]
pub use pamoja_codec as codec;
#[cfg(feature = "dashboard")]
pub use pamoja_dashboard as dashboard;
#[cfg(feature = "gpio")]
pub use pamoja_gpio as gpio;
#[cfg(feature = "kit")]
pub use pamoja_kit as kit;
#[cfg(feature = "ladder")]
pub use pamoja_ladder as ladder;
#[cfg(feature = "loopback")]
pub use pamoja_loopback as loopback;
#[cfg(feature = "lora")]
pub use pamoja_lora as lora;
#[cfg(feature = "lorawan")]
pub use pamoja_lorawan as lorawan;
#[cfg(feature = "mavlink")]
pub use pamoja_mavlink as mavlink;
#[cfg(feature = "mesh")]
pub use pamoja_mesh as mesh;
#[cfg(feature = "modbus")]
pub use pamoja_modbus as modbus;
#[cfg(feature = "mqtt")]
pub use pamoja_mqtt as mqtt;
#[cfg(feature = "power")]
pub use pamoja_power as power;
#[cfg(feature = "profile")]
pub use pamoja_profile as profile;
#[cfg(feature = "ros2")]
pub use pamoja_ros2 as ros2;
#[cfg(feature = "routing")]
pub use pamoja_routing as routing;
#[cfg(feature = "security")]
pub use pamoja_security as security;
#[cfg(feature = "sensors")]
pub use pamoja_sensors as sensors;
#[cfg(feature = "serial")]
pub use pamoja_serial as serial;
#[cfg(feature = "session")]
pub use pamoja_session as session;
#[cfg(feature = "sim")]
pub use pamoja_sim as sim;
#[cfg(feature = "sync")]
pub use pamoja_sync as sync;
#[cfg(feature = "telemetry")]
pub use pamoja_telemetry as telemetry;
#[cfg(feature = "update")]
pub use pamoja_update as update;
#[cfg(feature = "zenoh")]
pub use pamoja_zenoh as zenoh;
