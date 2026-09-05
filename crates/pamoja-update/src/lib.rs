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
//! - [`Delegation`] - the anchor's signed statement of which key may sign
//!   releases, so that key can be rotated without visiting the devices.
//! - [`Updater`] - the rules: verify, then stage, then try, then confirm or fall
//!   back. A transfer cut off by a dead link resumes where it stopped rather than
//!   starting over, which is what makes a large image installable over a slow
//!   radio at all.
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
//! # What it defends against
//!
//! RFC 9124 enumerates the threats a firmware update mechanism has to answer.
//! Each one this crate answers is answered by a rule with a test naming it:
//!
//! | Threat | Answered by |
//! | --- | --- |
//! | `THREAT.IMG.NON_AUTH`, unauthorised firmware | the author's signature, checked before the manifest is parsed |
//! | `THREAT.IMG.EXPIRED`, a replayed older release | a sequence number that must beat every slot, failed ones included |
//! | `THREAT.IMG.EXPIRED.OFFLINE`, a stale release aimed at a device that has been out of contact | [`Manifest::expires`], which bounds how long a release stays usable |
//! | `THREAT.IMG.INCOMPATIBLE`, firmware for another device | authenticated vendor and class identifiers |
//! | `THREAT.IMG.FORMAT`, a misread payload type | the payload format sits inside the signed body |
//!
//! Two it does not answer. `THREAT.IMG.DISCLOSURE`, an attacker reading the
//! firmware to hunt for flaws, wants payload encryption. `THREAT.UPD.WRONG_PRECURSOR`
//! only arises for differential updates, which this crate does not do.
//!
//! # Known limits
//!
//! **A retired key stays trusted until the device hears otherwise.** Rotation
//! takes effect when a device adopts the new delegation, and a device that has
//! been out of contact since a key was compromised still honours that key until
//! it is reached. There is no way to revoke faster than you can deliver, which
//! RFC 9124 acknowledges by leaving revocation outside the manifest format.
//! Setting an expiry on a delegation bounds the exposure for devices that have a
//! clock.
//!
//! **Delegation is one level deep.** The anchor appoints a release key, and the
//! chain stops there. RFC 9124 allows longer chains for delegated authority
//! between several parties; that is not implemented, and a release key cannot
//! appoint a successor.
//!
//! **The sequence number is only as trustworthy as the slot records.** It is
//! derived from what [`SlotStore`] reports, so an implementation that loses or
//! exposes those records weakens rollback protection. Hardware that can keep a
//! monotonic counter should be used where it exists.
//!
//! **It does not fetch, and it does not write to flash.** There is no transport
//! and no driver: the image arrives however the caller arranges, and
//! [`SlotStore`] is the seam to real storage.
//!
//! Also absent: attestation and secure boot, delta updates, encrypted payloads,
//! multi-payload dependency manifests, and the optional RFC 9124 elements for
//! multi-component devices, payload URIs, and execute-in-place metadata.
//!
//! # Resuming an interrupted transfer
//!
//! A slow radio can spend half an hour on one image, so a link that drops near the
//! end must not mean starting again. Progress is recorded as it is made, and
//! [`Updater::resume_at`] continues from there when the slot already holds part of
//! exactly the same image. Anything else starts over, because two images spliced
//! together are neither.
//!
//! Resuming does not make the earlier bytes trusted. A hash cannot be carried
//! across a reset, so it is rebuilt by reading back what the slot holds, and the
//! whole image is still settled by the digest check at the end. A resumed transfer
//! that completes with the wrong bytes fails exactly as a fresh one would.
//!
//! How often progress is recorded is the caller's to choose through its chunk
//! size: larger chunks mean fewer writes and less flash wear, but more to redo
//! after a reset.
//!
//! # Who may sign
//!
//! A device anchors its trust in one key. That anchor can sign releases itself,
//! which is the simple arrangement, or it can sign a [`Delegation`] naming a
//! separate release key and then stay somewhere hard to reach.
//!
//! The second is worth the extra step. The key that signs releases has to be
//! available every time you cut one, and availability is what eventually gets a
//! key stolen; an anchor that only comes out to authorise a rotation can live in a
//! safe. Rotating means issuing a delegation with a higher epoch, which retires
//! the previous key rather than adding to it.
//!
//! # How it boots
//!
//! An image is run from whichever slot holds it, and slots are never swapped.
//! That is the model MCUboot calls direct-XIP, chosen because a swap can be
//! interrupted halfway and then has to be recovered; here there is nothing to
//! recover, because nothing moves.
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
//!     expires: 0,
//! };
//! let mut envelope = [0u8; ENVELOPE_MAX];
//! let written = manifest.sign(&author, &mut envelope).unwrap();
//!
//! // The device trusts one author and knows what it is.
//! let device = Device {
//!     vendor_id: [0xab; 16],
//!     class_id: [0xcd; 16],
//!     anchor: author.public(),
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
mod trust;
mod update;
mod verify;

pub use error::{Refusal, Result};
pub use manifest::{
    Envelope, Manifest, PayloadFormat, DIGEST_LEN, ENVELOPE_MAX, ID_LEN, MANIFEST_MAX,
    STRUCTURE_VERSION,
};
pub use slots::{MemoryStore, SlotRecord, SlotState, SlotStore};
pub use trust::{Delegation, DELEGATION_MAX};
pub use update::{Boot, Device, Staging, Updater};
pub use verify::{image_digest, ImageVerifier, Verified};
