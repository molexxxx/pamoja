//! Generated Node bindings for signed firmware updates.
//!
//! These mirror the `pamoja-update` Rust API. Two audiences meet here: a build
//! server signs a manifest and a delegation, and a device decides what to accept.
//!
//! The updater is built over the in-memory slot store, because the Rust crate
//! takes any store through a trait and a trait cannot cross to JavaScript. What
//! does cross is the whole of the decision logic, which is the part that has to
//! be right.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_update::{
    Boot as CoreBoot, Delegation as CoreDelegation, Device, Envelope,
    ImageVerifier as CoreVerifier, Manifest as CoreManifest, MemoryStore, PayloadFormat, Refusal,
    SlotState as CoreSlotState, SlotStore, Updater as CoreUpdater, DELEGATION_MAX, DIGEST_LEN,
    ENVELOPE_MAX, ID_LEN, MANIFEST_MAX, STRUCTURE_VERSION,
};

use crate::security::DeviceIdentity;

/// The length in bytes of a public key.
const KEY_LEN: usize = 32;

/// The manifest structure version this build writes.
#[napi]
pub const UPDATE_STRUCTURE_VERSION: u8 = STRUCTURE_VERSION;

/// The payload format meaning the payload is the image itself, byte for byte.
#[napi]
pub const UPDATE_FORMAT_RAW: u8 = 1;

/// What a device believes about one slot.
#[napi(string_enum)]
pub enum SlotState {
    /// Nothing has been written here.
    Empty,
    /// An image is arriving, and `written` says how much of it has.
    Receiving,
    /// A complete image that matched its manifest, not yet tried.
    Staged,
    /// Being tried for the first time; it reverts unless it confirms.
    Pending,
    /// Tried and confirmed working.
    Confirmed,
    /// Tried and did not confirm, so it will not be tried again.
    Failed,
}

/// What a bootloader should do with what it found.
#[napi(string_enum)]
pub enum BootAction {
    /// Nothing new to try; run the confirmed image.
    Confirmed,
    /// A staged image is being tried for the first time.
    Trying,
    /// A pending image never confirmed, so it was failed.
    Reverted,
}

/// What a release says about itself, and what a device checks it against.
#[napi(object)]
pub struct Manifest {
    /// Which iteration of the manifest format this is.
    pub structure_version: u8,
    /// Rises with every release, which is what stops an older image being
    /// replayed at a device.
    pub sequence: f64,
    /// Who built the image, as 16 bytes.
    pub vendor_id: Buffer,
    /// Which kind of device it is for, as 16 bytes.
    pub class_id: Buffer,
    /// How the payload is encoded, currently only `UPDATE_FORMAT_RAW`.
    pub format: u8,
    /// Which slot the payload belongs in.
    pub storage: u8,
    /// The SHA-256 of the payload, which every other guarantee rests on.
    pub digest: Buffer,
    /// The payload length in bytes, known before a single byte is accepted.
    pub size: u32,
    /// When this release stops being offered, in seconds since the Unix epoch,
    /// or `0` to never expire.
    pub expires: f64,
}

/// A statement, signed by the anchor, that a second key may sign releases.
#[napi(object)]
pub struct Delegation {
    /// Rises with every rotation, so a retired key cannot be reinstated by
    /// replaying the statement that once authorised it.
    pub epoch: f64,
    /// The public key that may sign manifests while this delegation stands.
    pub release_key: Buffer,
    /// When the delegation stops being honoured, in seconds since the Unix
    /// epoch, or `0` to never expire.
    pub expires: f64,
}

/// The record a device keeps about one slot, durable across a reboot.
#[napi(object)]
pub struct SlotRecord {
    /// The state of the slot.
    pub state: SlotState,
    /// The sequence number of the image in the slot.
    pub sequence: f64,
    /// The length of the image in bytes.
    pub size: u32,
    /// The digest of the image.
    pub digest: Buffer,
    /// How many bytes have been stored, which is where a resumed transfer picks
    /// up.
    pub written: u32,
}

/// The decision a device made at boot, already recorded before it was returned.
#[napi(object)]
pub struct Boot {
    /// What the bootloader should do.
    pub action: BootAction,
    /// The image the decision is about, which for `Reverted` is the one that
    /// failed.
    pub slot: u8,
    /// The slot to run. It is the same as `slot` for anything but `Reverted`.
    pub fallback: u8,
}

/// How much of an image has arrived.
#[napi(object)]
pub struct Progress {
    /// The bytes stored so far.
    pub written: u32,
    /// The total the manifest declares.
    pub total: u32,
}

/// Encodes the body of a manifest, which is the part a signature covers.
#[napi]
pub fn encode_manifest(manifest: Manifest) -> napi::Result<Buffer> {
    let manifest = core_manifest(&manifest)?;
    let mut buf = [0u8; MANIFEST_MAX];
    let written = manifest.encode(&mut buf).map_err(refusal)?;
    Ok(buf[..written].to_vec().into())
}

/// Reads a manifest body back from its bytes.
///
/// This reads what a manifest claims; it proves nothing about who wrote it. Use
/// {@link verifyEnvelope} to read one whose signature has been checked.
#[napi]
pub fn decode_manifest(bytes: Buffer) -> napi::Result<Manifest> {
    CoreManifest::decode(bytes.as_ref())
        .map(|manifest| js_manifest(&manifest))
        .map_err(refusal)
}

/// Hashes a complete image, for a publisher filling in a manifest.
#[napi]
pub fn image_digest(image: Buffer) -> Buffer {
    pamoja_update::image_digest(image.as_ref()).to_vec().into()
}

/// Signs a manifest into the envelope that is offered to a device.
#[napi]
pub fn sign_manifest(manifest: Manifest, author: &DeviceIdentity) -> napi::Result<Buffer> {
    let manifest = core_manifest(&manifest)?;
    let mut buf = [0u8; ENVELOPE_MAX];
    let written = manifest.sign(&author.inner, &mut buf).map_err(refusal)?;
    Ok(buf[..written].to_vec().into())
}

/// Verifies an envelope against a key and reads the manifest inside it.
///
/// Throws when the signature is not from that key.
#[napi]
pub fn verify_envelope(bytes: Buffer, public_key: Buffer) -> napi::Result<Manifest> {
    let public = public(public_key.as_ref())?;
    let envelope = Envelope::decode(bytes.as_ref()).map_err(refusal)?;
    envelope
        .verify(&public)
        .map(|manifest| js_manifest(&manifest))
        .map_err(refusal)
}

/// Copies out the signed body of an envelope, without checking the signature.
///
/// This is what a gateway relays onward unchanged.
#[napi]
pub fn envelope_body(bytes: Buffer) -> napi::Result<Buffer> {
    Envelope::decode(bytes.as_ref())
        .map(|envelope| envelope.body().to_vec().into())
        .map_err(refusal)
}

/// Signs a delegation, naming a release key the anchor stands behind.
#[napi]
pub fn sign_delegation(delegation: Delegation, anchor: &DeviceIdentity) -> napi::Result<Buffer> {
    let delegation = core_delegation(&delegation)?;
    let mut buf = [0u8; DELEGATION_MAX];
    let written = delegation.sign(&anchor.inner, &mut buf).map_err(refusal)?;
    Ok(buf[..written].to_vec().into())
}

/// Opens a signed delegation against the anchor that should have signed it.
#[napi]
pub fn open_delegation(bytes: Buffer, anchor_public_key: Buffer) -> napi::Result<Delegation> {
    let anchor = public(anchor_public_key.as_ref())?;
    CoreDelegation::open(bytes.as_ref(), &anchor)
        .map(|delegation| js_delegation(&delegation))
        .map_err(refusal)
}

/// Hashes an image as it arrives and settles it against its manifest.
#[napi]
pub struct ImageVerifier {
    inner: Option<CoreVerifier>,
}

#[napi]
impl ImageVerifier {
    /// Creates a verifier for the image a manifest describes.
    #[napi(constructor)]
    pub fn new(manifest: Manifest) -> napi::Result<Self> {
        Ok(Self {
            inner: Some(CoreVerifier::new(&core_manifest(&manifest)?)),
        })
    }

    /// Takes the next piece of the image, in order.
    #[napi]
    pub fn update(&mut self, chunk: Buffer) -> napi::Result<()> {
        self.verifier()?.update(chunk.as_ref()).map_err(refusal)
    }

    /// Settles the image, returning its digest, and spends this verifier.
    ///
    /// Throws if the image is not the one the manifest described.
    #[napi]
    pub fn finish(&mut self) -> napi::Result<Buffer> {
        let verifier = self
            .inner
            .take()
            .ok_or_else(|| napi::Error::from_reason("this verifier has already been settled"))?;
        verifier
            .finish()
            .map(|verified| verified.digest().to_vec().into())
            .map_err(refusal)
    }

    /// Borrows the verifier, refusing one that has already been settled.
    fn verifier(&mut self) -> napi::Result<&mut CoreVerifier> {
        self.inner
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("this verifier has already been settled"))
    }
}

/// A device slots, and the rules applied to what is offered for them.
#[napi]
pub struct Updater {
    inner: CoreUpdater<MemoryStore>,
    staging: Option<Staging>,
}

/// The transfer an updater is part-way through, remembered between calls.
struct Staging {
    envelope: Vec<u8>,
    now: Option<u64>,
}

#[napi]
impl Updater {
    /// Creates an updater for a device with `slotCount` slots of `slotCapacity`
    /// bytes each, trusting `anchorPublicKey` as the root of every decision about
    /// who may update it.
    #[napi(constructor)]
    pub fn new(
        vendor_id: Buffer,
        class_id: Buffer,
        anchor_public_key: Buffer,
        slot_count: u8,
        slot_capacity: u32,
    ) -> napi::Result<Self> {
        let device = Device {
            vendor_id: fixed::<ID_LEN>(vendor_id.as_ref(), "vendorId")?,
            class_id: fixed::<ID_LEN>(class_id.as_ref(), "classId")?,
            anchor: public(anchor_public_key.as_ref())?,
        };
        Ok(Self {
            inner: CoreUpdater::new(device, MemoryStore::new(slot_count, slot_capacity)),
            staging: None,
        })
    }

    /// How many slots this device has.
    #[napi(getter)]
    pub fn slot_count(&self) -> u8 {
        self.inner.store().slot_count()
    }

    /// Reads what the device believes about one slot.
    #[napi]
    pub fn slot_record(&self, slot: u8) -> napi::Result<SlotRecord> {
        self.inner
            .store()
            .record(slot)
            .map(|record| SlotRecord {
                state: slot_state(record.state),
                sequence: record.sequence as f64,
                size: record.size,
                digest: record.digest.to_vec().into(),
                written: record.written,
            })
            .map_err(refusal)
    }

    /// The highest sequence number the device already holds.
    #[napi(getter)]
    pub fn installed_sequence(&self) -> napi::Result<f64> {
        self.inner
            .installed_sequence()
            .map(|sequence| sequence as f64)
            .map_err(refusal)
    }

    /// Records that a slot already holds a confirmed image at a sequence number.
    ///
    /// This is how a device that shipped with firmware says what it is running,
    /// so the rollback rule has something to compare against.
    #[napi]
    pub fn provision(&mut self, slot: u8, sequence: f64) -> napi::Result<()> {
        self.inner.provision(slot, sequence as u64).map_err(refusal)
    }

    /// Adopts a delegation, so releases signed by the key it names are accepted.
    #[napi]
    pub fn adopt(&mut self, envelope: Buffer, now: Option<f64>) -> napi::Result<Delegation> {
        self.inner
            .adopt(envelope.as_ref(), clock(now))
            .map(|delegation| js_delegation(&delegation))
            .map_err(refusal)
    }

    /// The delegation this updater currently honours, or `null` when releases
    /// must be signed by the anchor itself.
    #[napi(getter)]
    pub fn delegation(&self) -> Option<Delegation> {
        self.inner
            .delegation()
            .map(|delegation| js_delegation(&delegation))
    }

    /// Checks a manifest and stages an image that is already held whole.
    ///
    /// Returns the slot the image was staged into.
    #[napi]
    pub fn stage(&mut self, envelope: Buffer, image: Buffer, now: Option<f64>) -> napi::Result<u8> {
        self.inner
            .stage_at(envelope.as_ref(), image.as_ref(), clock(now))
            .map_err(refusal)
    }

    /// Checks a manifest and opens the slot it names for a transfer in pieces.
    ///
    /// Every check that can be made without the image runs here, so a release
    /// that is not for this device, would roll it back, or does not fit is
    /// refused before a byte of it is accepted. The envelope is remembered until
    /// {@link finish}, and each call after this one reopens the transfer from
    /// what the slot records, which is the same path a device takes after a
    /// reset.
    #[napi]
    pub fn begin(&mut self, envelope: Buffer, now: Option<f64>) -> napi::Result<u8> {
        let envelope = envelope.to_vec();
        let now = clock(now);
        let slot = self
            .inner
            .begin_at(&envelope, now)
            .map(|staging| staging.manifest().storage)
            .map_err(refusal)?;
        self.staging = Some(Staging { envelope, now });
        Ok(slot)
    }

    /// Takes the next piece of an image opened with {@link begin}.
    #[napi]
    pub fn write(&mut self, chunk: Buffer) -> napi::Result<()> {
        let (envelope, now) = self.open_transfer()?;
        let mut staging = self.inner.resume_at(&envelope, now).map_err(refusal)?;
        staging.write(chunk.as_ref()).map_err(refusal)
    }

    /// Reports how much of an opened image has arrived.
    #[napi]
    pub fn progress(&mut self) -> napi::Result<Progress> {
        let (envelope, now) = self.open_transfer()?;
        let staging = self.inner.resume_at(&envelope, now).map_err(refusal)?;
        let (written, total) = staging.progress();
        Ok(Progress { written, total })
    }

    /// Finishes an opened image and marks the slot bootable if it matched.
    ///
    /// Returns the slot now holding a staged image.
    #[napi]
    pub fn finish(&mut self) -> napi::Result<u8> {
        let (envelope, now) = self.open_transfer()?;
        let slot = self
            .inner
            .resume_at(&envelope, now)
            .and_then(|staging| staging.finish())
            .map_err(refusal)?;
        self.staging = None;
        Ok(slot)
    }

    /// Decides what to run, and records that decision before returning it.
    ///
    /// Call this once per boot, before jumping to an image. A staged image
    /// becomes pending here, so a device that resets before confirming reverts on
    /// the next call rather than trying a broken image forever.
    #[napi]
    pub fn on_boot(&mut self) -> napi::Result<Boot> {
        self.inner.on_boot().map(js_boot).map_err(refusal)
    }

    /// Confirms the pending image, so it will be run from now on.
    #[napi]
    pub fn confirm(&mut self) -> napi::Result<u8> {
        self.inner.confirm().map_err(refusal)
    }

    /// Fails the pending image and goes back to the confirmed one.
    #[napi]
    pub fn revert(&mut self) -> napi::Result<u8> {
        self.inner.revert().map_err(refusal)
    }

    /// Borrows the open transfer, refusing when none has been opened.
    fn open_transfer(&self) -> napi::Result<(Vec<u8>, Option<u64>)> {
        match &self.staging {
            Some(staging) => Ok((staging.envelope.clone(), staging.now)),
            None => Err(napi::Error::from_reason(
                "no transfer is open; call begin() first",
            )),
        }
    }
}

/// Turns an optional JavaScript timestamp into the optional the crate takes.
fn clock(now: Option<f64>) -> Option<u64> {
    now.map(|now| now as u64)
}

/// Maps a refusal onto the error JavaScript sees, naming the rule it broke.
fn refusal(refusal: Refusal) -> napi::Error {
    napi::Error::from_reason(refusal.reason())
}

/// Reads a fixed-width argument, naming it in the error when the length is wrong.
fn fixed<const N: usize>(bytes: &[u8], name: &str) -> napi::Result<[u8; N]> {
    <[u8; N]>::try_from(bytes)
        .map_err(|_| napi::Error::from_reason(format!("{name} must be exactly {N} bytes")))
}

/// Reads a 32-byte public key, rejecting one that is not a valid key.
fn public(bytes: &[u8]) -> napi::Result<pamoja_security::PublicIdentity> {
    let key = fixed::<KEY_LEN>(bytes, "publicKey")?;
    pamoja_security::PublicIdentity::from_bytes(&key)
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Rebuilds the core manifest from what crossed from JavaScript.
fn core_manifest(manifest: &Manifest) -> napi::Result<CoreManifest> {
    if manifest.format != UPDATE_FORMAT_RAW {
        return Err(refusal(Refusal::UnsupportedVersion));
    }
    Ok(CoreManifest {
        structure_version: manifest.structure_version,
        sequence: manifest.sequence as u64,
        vendor_id: fixed::<ID_LEN>(manifest.vendor_id.as_ref(), "vendorId")?,
        class_id: fixed::<ID_LEN>(manifest.class_id.as_ref(), "classId")?,
        format: PayloadFormat::Raw,
        storage: manifest.storage,
        digest: fixed::<DIGEST_LEN>(manifest.digest.as_ref(), "digest")?,
        size: manifest.size,
        expires: manifest.expires as u64,
    })
}

/// Maps a core manifest onto the value that crosses to JavaScript.
fn js_manifest(manifest: &CoreManifest) -> Manifest {
    Manifest {
        structure_version: manifest.structure_version,
        sequence: manifest.sequence as f64,
        vendor_id: manifest.vendor_id.to_vec().into(),
        class_id: manifest.class_id.to_vec().into(),
        format: manifest.format as u8,
        storage: manifest.storage,
        digest: manifest.digest.to_vec().into(),
        size: manifest.size,
        expires: manifest.expires as f64,
    }
}

/// Rebuilds the core delegation from what crossed from JavaScript.
fn core_delegation(delegation: &Delegation) -> napi::Result<CoreDelegation> {
    Ok(CoreDelegation {
        epoch: delegation.epoch as u64,
        release_key: fixed::<KEY_LEN>(delegation.release_key.as_ref(), "releaseKey")?,
        expires: delegation.expires as u64,
    })
}

/// Maps a core delegation onto the value that crosses to JavaScript.
fn js_delegation(delegation: &CoreDelegation) -> Delegation {
    Delegation {
        epoch: delegation.epoch as f64,
        release_key: delegation.release_key.to_vec().into(),
        expires: delegation.expires as f64,
    }
}

/// Maps a core slot state onto the value that crosses to JavaScript.
fn slot_state(state: CoreSlotState) -> SlotState {
    match state {
        CoreSlotState::Empty => SlotState::Empty,
        CoreSlotState::Receiving => SlotState::Receiving,
        CoreSlotState::Staged => SlotState::Staged,
        CoreSlotState::Pending => SlotState::Pending,
        CoreSlotState::Confirmed => SlotState::Confirmed,
        CoreSlotState::Failed => SlotState::Failed,
    }
}

/// Maps a core boot decision onto the value that crosses to JavaScript.
fn js_boot(boot: CoreBoot) -> Boot {
    match boot {
        CoreBoot::Confirmed(slot) => Boot {
            action: BootAction::Confirmed,
            slot,
            fallback: slot,
        },
        CoreBoot::Trying(slot) => Boot {
            action: BootAction::Trying,
            slot,
            fallback: slot,
        },
        CoreBoot::Reverted { failed, fallback } => Boot {
            action: BootAction::Reverted,
            slot: failed,
            fallback,
        },
    }
}
