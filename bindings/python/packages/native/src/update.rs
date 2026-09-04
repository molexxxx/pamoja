//! Generated Python bindings for signed firmware updates.
//!
//! These mirror the `pamoja-update` Rust API. Two audiences meet here: a build
//! server signs a manifest and a delegation, and a device decides what to
//! accept.
//!
//! The updater is built over the in-memory slot store, because the Rust crate
//! takes any store through a trait and a trait cannot reach Python. What does
//! cross is the whole of the decision logic, which is the part that has to be
//! right.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_update::{
    Boot, Delegation as CoreDelegation, Device, Envelope, ImageVerifier as CoreVerifier,
    Manifest as CoreManifest, MemoryStore, PayloadFormat, Refusal, SlotState, SlotStore,
    Updater as CoreUpdater, DELEGATION_MAX, DIGEST_LEN, ENVELOPE_MAX, ID_LEN, MANIFEST_MAX,
    STRUCTURE_VERSION,
};

use crate::security::DeviceIdentity;
use crate::PamojaError;

/// The length in bytes of a public key.
const KEY_LEN: usize = 32;

/// The manifest structure version this build writes.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn update_structure_version() -> u8 {
    STRUCTURE_VERSION
}

/// The payload format meaning the payload is the image itself, byte for byte.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn update_format_raw() -> u8 {
    1
}

/// What a release says about itself, and what a device checks it against.
#[gen_stub_pyclass]
#[pyclass]
pub struct Manifest {
    /// Which iteration of the manifest format this is.
    #[pyo3(get)]
    structure_version: u8,
    /// Rises with every release, which is what stops an older image being
    /// replayed at a device.
    #[pyo3(get)]
    sequence: u64,
    /// Who built the image, as 16 bytes.
    #[pyo3(get)]
    vendor_id: Vec<u8>,
    /// Which kind of device it is for, as 16 bytes.
    #[pyo3(get)]
    class_id: Vec<u8>,
    /// How the payload is encoded, currently only the raw format.
    #[pyo3(get)]
    format: u8,
    /// Which slot the payload belongs in.
    #[pyo3(get)]
    storage: u8,
    /// The SHA-256 of the payload, which every other guarantee rests on.
    #[pyo3(get)]
    digest: Vec<u8>,
    /// The payload length in bytes, known before a single byte is accepted.
    #[pyo3(get)]
    size: u32,
    /// When this release stops being offered, in seconds since the Unix epoch,
    /// or `0` to never expire.
    #[pyo3(get)]
    expires: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl Manifest {
    /// Describes a release.
    #[new]
    #[pyo3(signature = (sequence, vendor_id, class_id, storage, digest, size, expires = 0, format = 1, structure_version = STRUCTURE_VERSION))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        sequence: u64,
        vendor_id: Vec<u8>,
        class_id: Vec<u8>,
        storage: u8,
        digest: Vec<u8>,
        size: u32,
        expires: u64,
        format: u8,
        structure_version: u8,
    ) -> Self {
        Manifest {
            structure_version,
            sequence,
            vendor_id,
            class_id,
            format,
            storage,
            digest,
            size,
            expires,
        }
    }
}

/// A statement, signed by the anchor, that a second key may sign releases.
#[gen_stub_pyclass]
#[pyclass]
pub struct Delegation {
    /// Rises with every rotation, so a retired key cannot be reinstated by
    /// replaying the statement that once authorised it.
    #[pyo3(get)]
    epoch: u64,
    /// The public key that may sign manifests while this delegation stands.
    #[pyo3(get)]
    release_key: Vec<u8>,
    /// When the delegation stops being honoured, in seconds since the Unix
    /// epoch, or `0` to never expire.
    #[pyo3(get)]
    expires: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl Delegation {
    /// Names a release key the anchor stands behind.
    #[new]
    #[pyo3(signature = (epoch, release_key, expires = 0))]
    fn new(epoch: u64, release_key: Vec<u8>, expires: u64) -> Self {
        Delegation {
            epoch,
            release_key,
            expires,
        }
    }
}

/// The record a device keeps about one slot, durable across a reboot.
#[gen_stub_pyclass]
#[pyclass]
pub struct SlotRecord {
    /// The state of the slot, by name.
    #[pyo3(get)]
    state: String,
    /// The sequence number of the image in the slot.
    #[pyo3(get)]
    sequence: u64,
    /// The length of the image in bytes.
    #[pyo3(get)]
    size: u32,
    /// The digest of the image.
    #[pyo3(get)]
    digest: Vec<u8>,
    /// How many bytes have been stored, which is where a resumed transfer picks
    /// up.
    #[pyo3(get)]
    written: u32,
}

/// The decision a device made at boot, already recorded before it was returned.
#[gen_stub_pyclass]
#[pyclass]
pub struct BootDecision {
    /// What the bootloader should do: `Confirmed`, `Trying`, or `Reverted`.
    #[pyo3(get)]
    action: String,
    /// The image the decision is about, which for `Reverted` is the one that
    /// failed.
    #[pyo3(get)]
    slot: u8,
    /// The slot to run. It is the same as `slot` for anything but `Reverted`.
    #[pyo3(get)]
    fallback: u8,
}

/// How much of an image has arrived.
#[gen_stub_pyclass]
#[pyclass]
pub struct Progress {
    /// The bytes stored so far.
    #[pyo3(get)]
    written: u32,
    /// The total the manifest declares.
    #[pyo3(get)]
    total: u32,
}

/// Encodes the body of a manifest, which is the part a signature covers.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn encode_manifest(manifest: &Manifest) -> PyResult<Vec<u8>> {
    let manifest = core_manifest(manifest)?;
    let mut buf = [0u8; MANIFEST_MAX];
    let written = manifest.encode(&mut buf).map_err(refusal)?;
    Ok(buf[..written].to_vec())
}

/// Reads a manifest body back from its bytes.
///
/// This reads what a manifest claims; it proves nothing about who wrote it. Use
/// `verify_envelope` to read one whose signature has been checked.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn decode_manifest(data: Vec<u8>) -> PyResult<Manifest> {
    CoreManifest::decode(&data)
        .map(|manifest| py_manifest(&manifest))
        .map_err(refusal)
}

/// Signs a manifest into the envelope that is offered to a device.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sign_manifest(manifest: &Manifest, author: &DeviceIdentity) -> PyResult<Vec<u8>> {
    let manifest = core_manifest(manifest)?;
    let mut buf = [0u8; ENVELOPE_MAX];
    let written = manifest.sign(&author.inner, &mut buf).map_err(refusal)?;
    Ok(buf[..written].to_vec())
}

/// Verifies an envelope against a key and reads the manifest inside it.
///
/// Raises when the signature is not from that key.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn verify_envelope(data: Vec<u8>, public_key: Vec<u8>) -> PyResult<Manifest> {
    let public = public(&public_key)?;
    let envelope = Envelope::decode(&data).map_err(refusal)?;
    envelope
        .verify(&public)
        .map(|manifest| py_manifest(&manifest))
        .map_err(refusal)
}

/// Copies out the signed body of an envelope, without checking the signature.
///
/// This is what a gateway relays onward unchanged.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn envelope_body(data: Vec<u8>) -> PyResult<Vec<u8>> {
    Envelope::decode(&data)
        .map(|envelope| envelope.body().to_vec())
        .map_err(refusal)
}

/// Signs a delegation, naming a release key the anchor stands behind.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn sign_delegation(delegation: &Delegation, anchor: &DeviceIdentity) -> PyResult<Vec<u8>> {
    let delegation = core_delegation(delegation)?;
    let mut buf = [0u8; DELEGATION_MAX];
    let written = delegation.sign(&anchor.inner, &mut buf).map_err(refusal)?;
    Ok(buf[..written].to_vec())
}

/// Opens a signed delegation against the anchor that should have signed it.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn open_delegation(data: Vec<u8>, anchor_public_key: Vec<u8>) -> PyResult<Delegation> {
    let anchor = public(&anchor_public_key)?;
    CoreDelegation::open(&data, &anchor)
        .map(|delegation| py_delegation(&delegation))
        .map_err(refusal)
}

/// Hashes an image as it arrives and settles it against its manifest.
#[gen_stub_pyclass]
#[pyclass]
pub struct ImageVerifier {
    inner: Option<CoreVerifier>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ImageVerifier {
    /// Creates a verifier for the image a manifest describes.
    #[new]
    fn new(manifest: &Manifest) -> PyResult<Self> {
        Ok(ImageVerifier {
            inner: Some(CoreVerifier::new(&core_manifest(manifest)?)),
        })
    }

    /// Takes the next piece of the image, in order.
    fn update(&mut self, chunk: Vec<u8>) -> PyResult<()> {
        self.verifier()?.update(&chunk).map_err(refusal)
    }

    /// Settles the image, returning its digest, and spends this verifier.
    ///
    /// Raises if the image is not the one the manifest described.
    fn finish(&mut self) -> PyResult<Vec<u8>> {
        let verifier = self
            .inner
            .take()
            .ok_or_else(|| PamojaError::new_err("this verifier has already been settled"))?;
        verifier
            .finish()
            .map(|verified| verified.digest().to_vec())
            .map_err(refusal)
    }
}

impl ImageVerifier {
    /// Borrows the verifier, refusing one that has already been settled.
    fn verifier(&mut self) -> PyResult<&mut CoreVerifier> {
        self.inner
            .as_mut()
            .ok_or_else(|| PamojaError::new_err("this verifier has already been settled"))
    }
}

/// A device slots, and the rules applied to what is offered for them.
#[gen_stub_pyclass]
#[pyclass]
pub struct Updater {
    inner: CoreUpdater<MemoryStore>,
    staging: Option<(Vec<u8>, Option<u64>)>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Updater {
    /// Creates an updater for a device with `slot_count` slots of
    /// `slot_capacity` bytes each, trusting `anchor_public_key` as the root of
    /// every decision about who may update it.
    #[new]
    fn new(
        vendor_id: Vec<u8>,
        class_id: Vec<u8>,
        anchor_public_key: Vec<u8>,
        slot_count: u8,
        slot_capacity: u32,
    ) -> PyResult<Self> {
        let device = Device {
            vendor_id: fixed::<ID_LEN>(&vendor_id, "vendor_id")?,
            class_id: fixed::<ID_LEN>(&class_id, "class_id")?,
            anchor: public(&anchor_public_key)?,
        };
        Ok(Updater {
            inner: CoreUpdater::new(device, MemoryStore::new(slot_count, slot_capacity)),
            staging: None,
        })
    }

    /// How many slots this device has.
    #[getter]
    fn slot_count(&self) -> u8 {
        self.inner.store().slot_count()
    }

    /// Reads what the device believes about one slot.
    fn slot_record(&self, slot: u8) -> PyResult<SlotRecord> {
        self.inner
            .store()
            .record(slot)
            .map(|record| SlotRecord {
                state: slot_state(record.state),
                sequence: record.sequence,
                size: record.size,
                digest: record.digest.to_vec(),
                written: record.written,
            })
            .map_err(refusal)
    }

    /// The highest sequence number the device already holds.
    #[getter]
    fn installed_sequence(&self) -> PyResult<u64> {
        self.inner.installed_sequence().map_err(refusal)
    }

    /// Records that a slot already holds a confirmed image at a sequence number.
    ///
    /// This is how a device that shipped with firmware says what it is running,
    /// so the rollback rule has something to compare against.
    fn provision(&mut self, slot: u8, sequence: u64) -> PyResult<()> {
        self.inner.provision(slot, sequence).map_err(refusal)
    }

    /// Adopts a delegation, so releases signed by the key it names are accepted.
    #[pyo3(signature = (envelope, now = None))]
    fn adopt(&mut self, envelope: Vec<u8>, now: Option<u64>) -> PyResult<Delegation> {
        self.inner
            .adopt(&envelope, now)
            .map(|delegation| py_delegation(&delegation))
            .map_err(refusal)
    }

    /// The delegation this updater currently honours, or `None` when releases
    /// must be signed by the anchor itself.
    #[getter]
    fn delegation(&self) -> Option<Delegation> {
        self.inner
            .delegation()
            .map(|delegation| py_delegation(&delegation))
    }

    /// Checks a manifest and stages an image that is already held whole,
    /// returning the slot it went into.
    #[pyo3(signature = (envelope, image, now = None))]
    fn stage(&mut self, envelope: Vec<u8>, image: Vec<u8>, now: Option<u64>) -> PyResult<u8> {
        self.inner.stage_at(&envelope, &image, now).map_err(refusal)
    }

    /// Checks a manifest and opens the slot it names for a transfer in pieces.
    ///
    /// Every check that can be made without the image runs here, so a release
    /// that is not for this device, would roll it back, or does not fit is
    /// refused before a byte of it is accepted. The envelope is remembered until
    /// `finish`, and each call after this one reopens the transfer from what the
    /// slot records, which is the same path a device takes after a reset.
    #[pyo3(signature = (envelope, now = None))]
    fn begin(&mut self, envelope: Vec<u8>, now: Option<u64>) -> PyResult<u8> {
        let slot = self
            .inner
            .begin_at(&envelope, now)
            .map(|staging| staging.manifest().storage)
            .map_err(refusal)?;
        self.staging = Some((envelope, now));
        Ok(slot)
    }

    /// Takes the next piece of an image opened with `begin`.
    fn write(&mut self, chunk: Vec<u8>) -> PyResult<()> {
        let (envelope, now) = self.open_transfer()?;
        let mut staging = self.inner.resume_at(&envelope, now).map_err(refusal)?;
        staging.write(&chunk).map_err(refusal)
    }

    /// Reports how much of an opened image has arrived.
    fn progress(&mut self) -> PyResult<Progress> {
        let (envelope, now) = self.open_transfer()?;
        let staging = self.inner.resume_at(&envelope, now).map_err(refusal)?;
        let (written, total) = staging.progress();
        Ok(Progress { written, total })
    }

    /// Finishes an opened image and marks the slot bootable if it matched,
    /// returning the slot now holding it.
    fn finish(&mut self) -> PyResult<u8> {
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
    /// becomes pending here, so a device that resets before confirming reverts
    /// on the next call rather than trying a broken image forever.
    fn on_boot(&mut self) -> PyResult<BootDecision> {
        self.inner.on_boot().map(py_boot).map_err(refusal)
    }

    /// Confirms the pending image, so it will be run from now on.
    fn confirm(&mut self) -> PyResult<u8> {
        self.inner.confirm().map_err(refusal)
    }

    /// Fails the pending image and goes back to the confirmed one.
    fn revert(&mut self) -> PyResult<u8> {
        self.inner.revert().map_err(refusal)
    }
}

impl Updater {
    /// Borrows the open transfer, refusing when none has been opened.
    fn open_transfer(&self) -> PyResult<(Vec<u8>, Option<u64>)> {
        self.staging
            .clone()
            .ok_or_else(|| PamojaError::new_err("no transfer is open; call begin() first"))
    }
}

/// Maps a refusal onto the error Python sees, naming the rule it broke.
fn refusal(refusal: Refusal) -> PyErr {
    PamojaError::new_err(refusal.reason())
}

/// Reads a fixed-width argument, naming it in the error when the length is wrong.
fn fixed<const N: usize>(bytes: &[u8], name: &str) -> PyResult<[u8; N]> {
    <[u8; N]>::try_from(bytes)
        .map_err(|_| PamojaError::new_err(format!("{name} must be exactly {N} bytes")))
}

/// Reads a 32-byte public key, rejecting one that is not a valid key.
fn public(bytes: &[u8]) -> PyResult<pamoja_security::PublicIdentity> {
    let key = fixed::<KEY_LEN>(bytes, "public_key")?;
    pamoja_security::PublicIdentity::from_bytes(&key)
        .map_err(|error| PamojaError::new_err(error.to_string()))
}

/// Rebuilds the core manifest from what crossed from Python.
fn core_manifest(manifest: &Manifest) -> PyResult<CoreManifest> {
    if manifest.format != 1 {
        return Err(refusal(Refusal::UnsupportedVersion));
    }
    Ok(CoreManifest {
        structure_version: manifest.structure_version,
        sequence: manifest.sequence,
        vendor_id: fixed::<ID_LEN>(&manifest.vendor_id, "vendor_id")?,
        class_id: fixed::<ID_LEN>(&manifest.class_id, "class_id")?,
        format: PayloadFormat::Raw,
        storage: manifest.storage,
        digest: fixed::<DIGEST_LEN>(&manifest.digest, "digest")?,
        size: manifest.size,
        expires: manifest.expires,
    })
}

/// Maps a core manifest onto the value that crosses to Python.
fn py_manifest(manifest: &CoreManifest) -> Manifest {
    Manifest {
        structure_version: manifest.structure_version,
        sequence: manifest.sequence,
        vendor_id: manifest.vendor_id.to_vec(),
        class_id: manifest.class_id.to_vec(),
        format: manifest.format as u8,
        storage: manifest.storage,
        digest: manifest.digest.to_vec(),
        size: manifest.size,
        expires: manifest.expires,
    }
}

/// Rebuilds the core delegation from what crossed from Python.
fn core_delegation(delegation: &Delegation) -> PyResult<CoreDelegation> {
    Ok(CoreDelegation {
        epoch: delegation.epoch,
        release_key: fixed::<KEY_LEN>(&delegation.release_key, "release_key")?,
        expires: delegation.expires,
    })
}

/// Maps a core delegation onto the value that crosses to Python.
fn py_delegation(delegation: &CoreDelegation) -> Delegation {
    Delegation {
        epoch: delegation.epoch,
        release_key: delegation.release_key.to_vec(),
        expires: delegation.expires,
    }
}

/// Names a slot state for Python.
fn slot_state(state: SlotState) -> String {
    match state {
        SlotState::Empty => "Empty",
        SlotState::Receiving => "Receiving",
        SlotState::Staged => "Staged",
        SlotState::Pending => "Pending",
        SlotState::Confirmed => "Confirmed",
        SlotState::Failed => "Failed",
    }
    .to_owned()
}

/// Maps a core boot decision onto the value that crosses to Python.
fn py_boot(boot: Boot) -> BootDecision {
    match boot {
        Boot::Confirmed(slot) => BootDecision {
            action: "Confirmed".to_owned(),
            slot,
            fallback: slot,
        },
        Boot::Trying(slot) => BootDecision {
            action: "Trying".to_owned(),
            slot,
            fallback: slot,
        },
        Boot::Reverted { failed, fallback } => BootDecision {
            action: "Reverted".to_owned(),
            slot: failed,
            fallback,
        },
    }
}
