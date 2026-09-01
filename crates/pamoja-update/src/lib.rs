#![cfg_attr(not(test), no_std)]

//! Signed firmware updates for the pamoja SDK.
//!
//! A device in a clinic, on a pump, or under a solar panel is expensive to reach
//! and sometimes impossible. If it cannot be updated in place, every bug in it is
//! permanent and every fix is a journey. This crate is what makes updating one
//! safe enough to do remotely:
//!
//! - [`Manifest`] - what an update claims about itself: which device it is for,
//!   where it goes, how big it is, what it hashes to, and where it sits in the
//!   release order.
//! - [`Envelope`] - that manifest next to a signature over it, so a device can
//!   tell an author's release from anyone else's bytes.
//! - [`ImageVerifier`] - hashes the image as it arrives, so a device with
//!   kilobytes of memory can check a payload of megabytes.
//! - [`SlotStore`] and [`MemoryStore`] - where images live, with an in-memory
//!   implementation so the whole flow runs in a test with no hardware.
//! - [`Updater`] - the rules: verify, then stage, then try, then confirm or fall
//!   back.
//!
//! # Why this is safe over an untrusted link
//!
//! The signature covers the manifest, and the manifest commits to the image's
//! digest. Authenticity therefore reaches the image without the carrier being
//! trusted at all. An update can ride a LoRa mesh, a passing phone, or a USB stick
//! left in a village, and a device will still only run what its author released.
//! That matters more here than a secure channel would, because the deployments
//! this SDK targets frequently have no certificate authority, and often no
//! internet.
//!
//! # Relationship to the SUIT specifications
//!
//! The information model is the one RFC 9124 defines, and the architecture and
//! terminology are RFC 9019's. Both are published standards. The concrete CBOR
//! serialization, `draft-ietf-suit-manifest`, is not: at the time of writing it
//! remains an Internet-Draft awaiting publication.
//!
//! So this crate implements the settled part and serializes it itself, rather
//! than pinning the SDK to a wire format that can still change. The encoding is
//! deliberately kept separate from the model, so a SUIT reader can later produce
//! the same [`Manifest`] without any of the rules around it moving. This is the same kind of considered deviation as the hand-written
//! MAVLink dialect, and it is recorded here rather than left to be discovered.
//!
//! # What this crate does not do
//!
//! It does not fetch, and it does not write to flash. It has no transport and no
//! driver: the image arrives however the caller arranges, and [`SlotStore`] is the
//! seam to real storage. Attestation, delta updates, encrypted payloads, and
//! multi-payload dependency manifests are not implemented.
//!
//! # Examples
//!
//! An update is released, carried to a device, tried, and confirmed:
//!
//! ```
//! use pamoja_security::DeviceIdentity;
//! use pamoja_update::{
//!     Boot, Device, Manifest, MemoryStore, PayloadFormat, Updater, ENVELOPE_MAX,
//!     STRUCTURE_VERSION,
//! };
//! use sha2::{Digest, Sha256};
//!
//! let author = DeviceIdentity::from_seed(&[1u8; 32]);
//! let image = b"version two of the firmware";
//!
//! let manifest = Manifest {
//!     structure_version: STRUCTURE_VERSION,
//!     sequence: 2,
//!     vendor_id: [0xab; 16],
//!     class_id: [0xcd; 16],
//!     format: PayloadFormat::Raw,
//!     storage: 1,
//!     digest: Sha256::digest(image).into(),
//!     size: image.len() as u32,
//! };
//! let mut envelope = [0u8; ENVELOPE_MAX];
//! let written = manifest.sign(&author, &mut envelope).unwrap();
//!
//! // The device trusts one author and knows what it is.
//! let device = Device {
//!     vendor_id: [0xab; 16],
//!     class_id: [0xcd; 16],
//!     author: author.public(),
//! };
//! let mut updater = Updater::new(device, MemoryStore::new(2, 4096));
//! updater.provision(0, 1).unwrap(); // the image it shipped with
//!
//! updater.stage(&envelope[..written], image).unwrap();
//! assert_eq!(updater.on_boot().unwrap(), Boot::Trying(1));
//! assert_eq!(updater.confirm().unwrap(), 1);
//!
//! // The next boot simply runs it.
//! assert_eq!(updater.on_boot().unwrap(), Boot::Confirmed(1));
//! ```

extern crate alloc;

mod cbor;
mod error;
mod manifest;
mod slots;
mod update;
mod verify;

pub use error::{Refusal, Result};
pub use manifest::{
    Envelope, Manifest, PayloadFormat, DIGEST_LEN, ENVELOPE_MAX, ID_LEN, MANIFEST_MAX,
    STRUCTURE_VERSION,
};
pub use slots::{MemoryStore, SlotRecord, SlotState, SlotStore};
pub use update::{Boot, Device, Staging, Updater};
pub use verify::{ImageVerifier, Verified};
