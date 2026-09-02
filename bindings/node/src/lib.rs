//! Node.js bindings for the pamoja SDK, generated with napi-rs.
//!
//! This crate is the generated low-level surface (the contract tier): it exposes
//! the Rust core and capability crates to JavaScript and TypeScript one-to-one. A
//! hand-written, idiomatic facade wraps it for everyday use; see the package's
//! TypeScript entry point.

use napi_derive::napi;

/// Returns the version of the native pamoja module.
#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// The capability modules are public so their free functions stay reachable from
// the crate root. In a cdylib this adds no Rust-visible API, but a `#[napi]`
// function in a private module reads as dead code to the lint pass that runs
// over the test target.
#[cfg(feature = "actuators")]
pub mod actuators;
#[cfg(feature = "can")]
pub mod can;
#[cfg(feature = "codec")]
pub mod codec;
#[cfg(feature = "gpio")]
pub mod gpio;
#[cfg(feature = "kit")]
pub mod kit;
#[cfg(feature = "lora")]
pub mod lora;
#[cfg(feature = "lorawan")]
pub mod lorawan;
#[cfg(feature = "mesh")]
pub mod mesh;
#[cfg(feature = "modbus")]
pub mod modbus;
#[cfg(feature = "mqtt")]
pub mod mqtt;
#[cfg(feature = "routing")]
pub mod routing;
#[cfg(feature = "security")]
pub mod security;
#[cfg(feature = "sensors")]
pub mod sensors;
#[cfg(feature = "serial")]
pub mod serial;
