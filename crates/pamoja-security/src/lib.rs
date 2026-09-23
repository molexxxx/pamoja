#![cfg_attr(not(test), no_std)]

//! Device identity and signed telemetry for the pamoja SDK.
//!
//! Many of the deployments this SDK is built for - vaccine fridges, clinic
//! telemetry, water and energy metering - need their data to be trustworthy, not
//! just delivered. A reading that drives a health or billing decision has to be
//! provably from the device that claims to have sent it, and provably unaltered on
//! the way. This crate provides that foundation with ed25519 signatures:
//!
//! - [`DeviceIdentity`] - a device's private key, built from a provisioned 32-byte
//!   seed, that signs the payloads the device emits.
//! - [`PublicIdentity`] - the matching public key, safe to share, that a gateway or
//!   auditor uses to verify a payload is authentic and unaltered.
//! - [`Signature`] - the 64-byte detached signature carried alongside a payload.
//!
//! Signing and verifying are deterministic and need no randomness, so the crate is
//! `no_std` and runs unchanged on a microcontroller that signs its own telemetry.
//! It is the groundwork the security pillar builds on, ahead of transport-level
//! TLS/DTLS and signed over-the-air updates.
//!
//! # Examples
//!
//! Sign a reading on the device, then verify it as an auditor would:
//!
//! ```
//! use pamoja_security::DeviceIdentity;
//!
//! // A device is provisioned with a 32-byte secret seed.
//! let device = DeviceIdentity::from_seed(&[42u8; 32]);
//! let public = device.public();
//!
//! // A water meter signs its reading; the signature travels with the data.
//! let reading = b"meter-12: 1043.7 m3 @ 1700";
//! let signature = device.sign(reading);
//!
//! // The utility, holding the meter's public identity, confirms it is authentic.
//! assert!(public.verify(reading, &signature).is_ok());
//!
//! // A reading edited on the way does not verify.
//! assert!(public.verify(b"meter-12: 943.7 m3 @ 1700", &signature).is_err());
//! ```

extern crate alloc;

mod identity;
mod signature;

pub use identity::{DeviceIdentity, PublicIdentity};
pub use signature::Signature;
