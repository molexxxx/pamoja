//! The rules an update passes before a device will run it.
//!
//! The order matters and is deliberate. The signature is checked before the
//! manifest is interpreted, the manifest is checked before a byte of image is
//! accepted, and the image is checked before the slot is marked bootable. At no
//! point is something unverified recorded as usable, so a device interrupted at
//! any moment comes back up on the last image it confirmed.

use pamoja_security::PublicIdentity;

use crate::error::{Refusal, Result};
use crate::manifest::{Envelope, Manifest, ID_LEN};
use crate::slots::{SlotRecord, SlotState, SlotStore};
use crate::verify::ImageVerifier;

/// What the bootloader should do with this boot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boot {
    /// Nothing new to try; run the confirmed image in this slot.
    Confirmed(u8),
    /// A staged image is being tried for the first time. It is now pending, so if
    /// it does not confirm itself, the next boot will revert.
    Trying(u8),
    /// A pending image never confirmed, so it has been failed and will not be
    /// tried again. Run the fallback.
    Reverted {
        /// The slot whose image did not confirm.
        failed: u8,
        /// The confirmed slot to run instead.
        fallback: u8,
    },
}

/// Who this device is, and who it trusts to update it.
#[derive(Clone, Copy, Debug)]
pub struct Device {
    /// Who built this device's firmware.
    pub vendor_id: [u8; ID_LEN],
    /// What kind of device this is.
    pub class_id: [u8; ID_LEN],
    /// The public key of the only author whose updates it will take.
    pub author: PublicIdentity,
}

/// Applies the update rules against a device's slots.
pub struct Updater<S> {
    device: Device,
    store: S,
}

impl<S: SlotStore> Updater<S> {
    /// Creates an updater over a device's slots.
    ///
    /// # Arguments
    ///
    /// * `device` - the device's identity and trusted author.
    /// * `store` - where its images live.
    ///
    /// # Returns
    ///
    /// The updater.
    pub fn new(device: Device, store: S) -> Self {
        Self { device, store }
    }

    /// Borrows the underlying slot store.
    ///
    /// # Returns
    ///
    /// The store, for inspecting slot records.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Returns the highest sequence number any slot holds.
    ///
    /// A new manifest must beat this, not merely the running image, so an older
    /// release cannot be slipped in alongside a newer one that is already staged.
    /// Failed slots count too: re-releasing a sequence that already failed would
    /// let a captured image be replayed.
    ///
    /// # Returns
    ///
    /// The highest sequence number present, or `0` if every slot is empty.
    ///
    /// # Errors
    ///
    /// Returns a refusal if a slot record cannot be read.
    pub fn installed_sequence(&self) -> Result<u64> {
        let mut highest = 0;
        for slot in 0..self.store.slot_count() {
            let record = self.store.record(slot)?;
            if record.state != SlotState::Empty {
                highest = highest.max(record.sequence);
            }
        }
        Ok(highest)
    }

    /// Checks a manifest and opens the slot it names for writing.
    ///
    /// # Arguments
    ///
    /// * `envelope` - the signed manifest offered to this device.
    ///
    /// # Returns
    ///
    /// A [`Staging`] ready to take the image, once every check that can be made
    /// without the image has passed.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Signature`] if the envelope is not from the trusted
    /// author, [`Refusal::WrongDevice`] if it is for a different vendor or class,
    /// [`Refusal::Rollback`] if it would not move the device forward,
    /// [`Refusal::SlotTooSmall`] if the image cannot fit, or
    /// [`Refusal::WrongState`] if it names the slot the device would fall back to.
    pub fn begin(&mut self, envelope: &[u8]) -> Result<Staging<'_, S>> {
        let manifest = Envelope::decode(envelope)?.verify(&self.device.author)?;

        if manifest.vendor_id != self.device.vendor_id || manifest.class_id != self.device.class_id
        {
            return Err(Refusal::WrongDevice);
        }

        if manifest.sequence <= self.installed_sequence()? {
            return Err(Refusal::Rollback);
        }

        let slot = manifest.storage;
        if manifest.size > self.store.capacity(slot)? {
            return Err(Refusal::SlotTooSmall);
        }

        // Writing over the confirmed image would leave nothing to fall back to, so
        // an update that names that slot is refused rather than obeyed.
        if self.store.record(slot)?.state == SlotState::Confirmed {
            return Err(Refusal::WrongState);
        }

        self.store.erase(slot)?;
        Ok(Staging {
            store: &mut self.store,
            slot,
            verifier: ImageVerifier::new(&manifest),
            manifest,
            offset: 0,
        })
    }

    /// Checks a manifest and stages an image already held whole.
    ///
    /// # Arguments
    ///
    /// * `envelope` - the signed manifest.
    /// * `image` - the whole image.
    ///
    /// # Returns
    ///
    /// The slot the image was staged into.
    ///
    /// # Errors
    ///
    /// Returns whatever [`begin`](Self::begin) or [`Staging::finish`] refuses.
    pub fn stage(&mut self, envelope: &[u8], image: &[u8]) -> Result<u8> {
        let mut staging = self.begin(envelope)?;
        staging.write(image)?;
        staging.finish()
    }

    /// Decides what to run, and records that decision before returning it.
    ///
    /// Call this once per boot, before jumping to an image. A staged image
    /// becomes pending here, so if the device resets before confirming, the next
    /// call sees a pending slot and reverts.
    ///
    /// # Returns
    ///
    /// What the bootloader should run.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::NothingToRevert`] if there is no image to fall back to.
    pub fn on_boot(&mut self) -> Result<Boot> {
        if let Some(pending) = self.find(SlotState::Pending)? {
            // It was booted last time and never said it was healthy.
            self.set_state(pending, SlotState::Failed)?;
            let fallback = self
                .find(SlotState::Confirmed)?
                .ok_or(Refusal::NothingToRevert)?;
            return Ok(Boot::Reverted {
                failed: pending,
                fallback,
            });
        }

        if let Some(staged) = self.find(SlotState::Staged)? {
            self.set_state(staged, SlotState::Pending)?;
            return Ok(Boot::Trying(staged));
        }

        self.find(SlotState::Confirmed)?
            .map(Boot::Confirmed)
            .ok_or(Refusal::NothingToRevert)
    }

    /// Reports the running image healthy, making it the one to fall back to.
    ///
    /// The slot the device previously fell back to is erased, which is what frees
    /// it to receive the next update.
    ///
    /// # Returns
    ///
    /// The slot that is now confirmed.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::WrongState`] if no image is pending, so a confirmation
    /// that arrives twice, or from an image nobody is trying, does nothing.
    pub fn confirm(&mut self) -> Result<u8> {
        let pending = self.find(SlotState::Pending)?.ok_or(Refusal::WrongState)?;

        if let Some(previous) = self.find(SlotState::Confirmed)? {
            self.store.erase(previous)?;
        }
        self.set_state(pending, SlotState::Confirmed)?;
        Ok(pending)
    }

    /// Gives up on the pending image and goes back to the confirmed one.
    ///
    /// # Returns
    ///
    /// The slot the device falls back to.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::WrongState`] if no image is pending, or
    /// [`Refusal::NothingToRevert`] if there is nothing to fall back to.
    pub fn revert(&mut self) -> Result<u8> {
        let pending = self.find(SlotState::Pending)?.ok_or(Refusal::WrongState)?;
        let fallback = self
            .find(SlotState::Confirmed)?
            .ok_or(Refusal::NothingToRevert)?;
        self.set_state(pending, SlotState::Failed)?;
        Ok(fallback)
    }

    /// Marks a slot confirmed at first provisioning, when nothing was staged.
    ///
    /// A device leaves the factory already running an image that no update
    /// installed. Without this there is no fallback for the first update to
    /// return to.
    ///
    /// # Arguments
    ///
    /// * `slot` - the slot the factory image occupies.
    /// * `sequence` - the sequence number of that image.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the slot is confirmed.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::WrongState`] if any slot is already confirmed.
    pub fn provision(&mut self, slot: u8, sequence: u64) -> Result<()> {
        if self.find(SlotState::Confirmed)?.is_some() {
            return Err(Refusal::WrongState);
        }
        let mut record = self.store.record(slot)?;
        record.state = SlotState::Confirmed;
        record.sequence = sequence;
        self.store.set_record(slot, record)
    }

    /// Returns the first slot in the given state.
    fn find(&self, state: SlotState) -> Result<Option<u8>> {
        for slot in 0..self.store.slot_count() {
            if self.store.record(slot)?.state == state {
                return Ok(Some(slot));
            }
        }
        Ok(None)
    }

    /// Changes a slot's state, leaving the rest of its record alone.
    fn set_state(&mut self, slot: u8, state: SlotState) -> Result<()> {
        let mut record = self.store.record(slot)?;
        record.state = state;
        self.store.set_record(slot, record)
    }
}

/// A slot open for an image, with the manifest's promises still to be met.
///
/// The slot's record is only written once the image has been verified whole, so
/// an interrupted transfer leaves a slot that is never bootable rather than one
/// holding half an image.
pub struct Staging<'a, S: SlotStore> {
    store: &'a mut S,
    slot: u8,
    manifest: Manifest,
    verifier: ImageVerifier,
    offset: u32,
}

impl<S: SlotStore> Staging<'_, S> {
    /// Takes the next piece of the image.
    ///
    /// # Arguments
    ///
    /// * `chunk` - the next bytes of the image, in order.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the chunk is hashed and stored.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Size`] if more bytes arrive than the manifest declared,
    /// or [`Refusal::SlotTooSmall`] if the slot cannot take them.
    pub fn write(&mut self, chunk: &[u8]) -> Result<()> {
        self.verifier.update(chunk)?;
        self.store.write(self.slot, self.offset, chunk)?;
        self.offset += chunk.len() as u32;
        Ok(())
    }

    /// Finishes the image and marks the slot bootable if it matched.
    ///
    /// # Returns
    ///
    /// The slot now holding a staged image.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Size`] or [`Refusal::Digest`] if the image is not the
    /// one the manifest described, leaving the slot unbootable.
    pub fn finish(self) -> Result<u8> {
        let verified = self.verifier.finish()?;
        self.store.set_record(
            self.slot,
            SlotRecord {
                state: SlotState::Staged,
                sequence: self.manifest.sequence,
                size: verified.size(),
                digest: verified.digest(),
            },
        )?;
        Ok(self.slot)
    }

    /// Returns the manifest this staging is fulfilling.
    ///
    /// # Returns
    ///
    /// The verified manifest.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
}
