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
use crate::trust::Delegation;
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
    /// The key this device anchors its trust in.
    ///
    /// It is the root of every decision about who may update the device, so it is
    /// used almost never: either to sign releases directly, or to sign a
    /// [`Delegation`] naming a release key that does. The second arrangement is
    /// the one to prefer, because it lets the anchor stay somewhere it is hard to
    /// steal.
    pub anchor: PublicIdentity,
}

/// Applies the update rules against a device's slots.
pub struct Updater<S> {
    device: Device,
    store: S,
    delegation: Option<Delegation>,
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
        Self {
            device,
            store,
            delegation: None,
        }
    }

    /// Adopts a delegation the device already held, after a restart.
    ///
    /// A delegation is small and its envelope is self-authenticating, so the
    /// simplest place to keep one is wherever the caller already keeps device
    /// settings. Hand it back here on the way up.
    ///
    /// # Arguments
    ///
    /// * `envelope` - the stored delegation envelope.
    /// * `now` - seconds since the Unix epoch, or `None` on a device with no clock.
    ///
    /// # Returns
    ///
    /// The updater, now accepting releases signed by the delegated key.
    ///
    /// # Errors
    ///
    /// Returns whatever [`adopt`](Self::adopt) refuses.
    pub fn with_delegation(mut self, envelope: &[u8], now: Option<u64>) -> Result<Self> {
        self.adopt(envelope, now)?;
        Ok(self)
    }

    /// Takes on a delegation, moving which key may sign this device's updates.
    ///
    /// The caller should persist the envelope it just passed, so the device comes
    /// back up trusting the same key.
    ///
    /// # Arguments
    ///
    /// * `envelope` - a delegation signed by the device's trust anchor.
    /// * `now` - seconds since the Unix epoch, or `None` on a device with no clock.
    ///
    /// # Returns
    ///
    /// The delegation now in force.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Signature`] if it was not signed by the trust anchor,
    /// [`Refusal::Rollback`] if its epoch does not rise above the one already
    /// held, so a retired key cannot be reinstated by replay, and
    /// [`Refusal::Expired`] or [`Refusal::NoClock`] on the same terms as a
    /// manifest.
    pub fn adopt(&mut self, envelope: &[u8], now: Option<u64>) -> Result<Delegation> {
        let delegation = Delegation::open(envelope, &self.device.anchor)?;

        if let Some(held) = self.delegation {
            if delegation.epoch <= held.epoch {
                return Err(Refusal::Rollback);
            }
        }

        if delegation.expires != 0 {
            match now {
                Some(now) if now < delegation.expires => {}
                Some(_) => return Err(Refusal::Expired),
                None => return Err(Refusal::NoClock),
            }
        }

        // Refuse a delegation naming something that is not a usable key, rather
        // than adopting it and discovering at the next release that nothing can
        // sign for this device any more.
        delegation.signer()?;

        self.delegation = Some(delegation);
        Ok(delegation)
    }

    /// Returns the delegation in force, if the device holds one.
    ///
    /// # Returns
    ///
    /// The delegation, or `None` when releases are signed by the anchor itself.
    pub fn delegation(&self) -> Option<Delegation> {
        self.delegation
    }

    /// Returns the key a manifest must be signed by right now.
    ///
    /// # Returns
    ///
    /// The delegated release key when one is in force, and the trust anchor
    /// otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Signature`] if a held delegation names an unusable key.
    fn signing_key(&self) -> Result<PublicIdentity> {
        match self.delegation {
            Some(delegation) => delegation.signer(),
            None => Ok(self.device.anchor),
        }
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
            if !matches!(record.state, SlotState::Empty | SlotState::Receiving) {
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
    /// Returns whatever [`begin_at`](Self::begin_at) refuses. A manifest that
    /// carries an expiry is refused, because a device with no clock cannot honour
    /// one; call [`begin_at`](Self::begin_at) with the time if it has one.
    pub fn begin(&mut self, envelope: &[u8]) -> Result<Staging<'_, S>> {
        self.begin_at(envelope, None)
    }

    /// Checks a manifest against the current time and opens the slot it names.
    ///
    /// # Arguments
    ///
    /// * `envelope` - the signed manifest offered to this device.
    /// * `now` - seconds since the Unix epoch, or `None` on a device with no
    ///   clock.
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
    /// [`Refusal::Expired`] if its expiry has passed, [`Refusal::NoClock`] if it
    /// expires and `now` is `None`, [`Refusal::Rollback`] if it would not move the
    /// device forward, [`Refusal::SlotTooSmall`] if the image cannot fit, or
    /// [`Refusal::WrongState`] if it names the slot the device would fall back to.
    pub fn begin_at(&mut self, envelope: &[u8], now: Option<u64>) -> Result<Staging<'_, S>> {
        let manifest = Envelope::decode(envelope)?.verify(&self.signing_key()?)?;
        self.check(&manifest, now)?;
        self.open(manifest)
    }

    /// Runs every check that can be made before a byte of image arrives.
    fn check(&self, manifest: &Manifest, now: Option<u64>) -> Result<()> {
        if manifest.vendor_id != self.device.vendor_id || manifest.class_id != self.device.class_id
        {
            return Err(Refusal::WrongDevice);
        }

        // A sequence number cannot protect a device that has been offline a long
        // time: an attacker can offer it a release genuinely newer than the one it
        // runs, but old enough to have a known flaw. An expiry bounds that window.
        if manifest.expires != 0 {
            match now {
                Some(now) if now < manifest.expires => {}
                Some(_) => return Err(Refusal::Expired),
                None => return Err(Refusal::NoClock),
            }
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

        Ok(())
    }

    /// Clears the target slot and opens it for a transfer starting at zero.
    fn open(&mut self, manifest: Manifest) -> Result<Staging<'_, S>> {
        let slot = manifest.storage;
        self.store.erase(slot)?;
        // Recording the target before any bytes arrive is what makes the transfer
        // resumable: after a reset the device can tell what it was receiving.
        self.store.set_record(
            slot,
            SlotRecord {
                state: SlotState::Receiving,
                sequence: manifest.sequence,
                size: manifest.size,
                digest: manifest.digest,
                written: 0,
            },
        )?;
        Ok(Staging {
            store: &mut self.store,
            slot,
            verifier: ImageVerifier::new(&manifest),
            manifest,
            offset: 0,
        })
    }

    /// Opens a slot for an image, continuing a transfer that was cut off.
    ///
    /// A slow radio can spend half an hour on a single image, so a link that drops
    /// near the end must not mean starting again. If the slot already holds part of
    /// exactly this image, the transfer picks up where it stopped; anything else
    /// starts over, because mixing two images produces neither. An image whose last
    /// byte arrived but which was never settled counts as picking up where it
    /// stopped, so a reset in that gap costs nothing.
    ///
    /// # Arguments
    ///
    /// * `envelope` - the signed manifest offered to this device.
    /// * `now` - seconds since the Unix epoch, or `None` on a device with no clock.
    ///
    /// # Returns
    ///
    /// A [`Staging`] positioned after whatever already arrived, which
    /// [`progress`](Staging::progress) reports.
    ///
    /// # Errors
    ///
    /// Returns whatever [`begin_at`](Self::begin_at) refuses.
    pub fn resume_at(&mut self, envelope: &[u8], now: Option<u64>) -> Result<Staging<'_, S>> {
        let manifest = Envelope::decode(envelope)?.verify(&self.signing_key()?)?;
        self.check(&manifest, now)?;

        let slot = manifest.storage;
        let record = self.store.record(slot)?;
        let resumable = record.state == SlotState::Receiving
            && record.digest == manifest.digest
            && record.size == manifest.size
            && record.written <= manifest.size;

        if !resumable {
            return self.open(manifest);
        }

        // The hash cannot be carried across a reset, so it is rebuilt by reading
        // back what the slot already holds. Those bytes are still unproven; the
        // digest check at the end settles them, exactly as for a fresh transfer.
        let mut verifier = ImageVerifier::new(&manifest);
        let mut buf = [0u8; 256];
        let mut at = 0u32;
        while at < record.written {
            let want = buf.len().min((record.written - at) as usize);
            let read = self.store.read(slot, at, &mut buf[..want])?;
            if read == 0 {
                return Err(Refusal::Malformed);
            }
            verifier.update(&buf[..read])?;
            at += read as u32;
        }

        Ok(Staging {
            store: &mut self.store,
            slot,
            verifier,
            manifest,
            offset: record.written,
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
        self.stage_at(envelope, image, None)
    }

    /// Checks a manifest against the current time and stages an image held whole.
    ///
    /// # Arguments
    ///
    /// * `envelope` - the signed manifest.
    /// * `image` - the whole image.
    /// * `now` - seconds since the Unix epoch, or `None` on a device with no clock.
    ///
    /// # Returns
    ///
    /// The slot the image was staged into.
    ///
    /// # Errors
    ///
    /// Returns whatever [`begin_at`](Self::begin_at) or [`Staging::finish`]
    /// refuses.
    pub fn stage_at(&mut self, envelope: &[u8], image: &[u8], now: Option<u64>) -> Result<u8> {
        let mut staging = self.begin_at(envelope, now)?;
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

        // Progress is recorded as it is made, so a reset costs at most the chunk in
        // flight. How much that costs is the caller's to choose: a larger chunk
        // means fewer record writes and so less flash wear, but more to redo.
        let mut record = self.store.record(self.slot)?;
        record.written = self.offset;
        self.store.set_record(self.slot, record)
    }

    /// Reports how much of the image has arrived.
    ///
    /// # Returns
    ///
    /// The bytes stored so far and the total the manifest declares.
    pub fn progress(&self) -> (u32, u32) {
        (self.offset, self.manifest.size)
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
                written: verified.size(),
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
