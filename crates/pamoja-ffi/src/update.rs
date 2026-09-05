//! The C ABI for signed firmware updates.
//!
//! These functions wrap [`pamoja_update`] for callers that reach the SDK through
//! the flat C boundary. Two audiences meet here. A build server signs a manifest
//! and a delegation, which is all value math over
//! [`PamojaManifest`] and [`PamojaDelegation`]. A device decides what to accept,
//! which needs the slots it keeps images in, so an updater crosses as an opaque
//! handle.
//!
//! The updater is built over the in-memory slot store. The Rust crate takes any
//! store through a trait, and a trait cannot cross a C ABI, so a caller wiring
//! real flash writes that in Rust; what crosses here is the whole of the decision
//! logic, which is the part that has to be right.

use std::ptr;

use pamoja_update::{
    Boot, Delegation, Device, Envelope, Manifest, MemoryStore, PayloadFormat, Refusal, SlotRecord,
    SlotState, SlotStore, Updater, DELEGATION_MAX, DIGEST_LEN, ENVELOPE_MAX, ID_LEN, MANIFEST_MAX,
    STRUCTURE_VERSION,
};
use pamoja_update::{ImageVerifier, Verified};

use crate::security::{identity_handle, read_public, PamojaDeviceIdentity, PAMOJA_KEY_LEN};
use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// The length in bytes of a vendor or device-class identifier.
pub const PAMOJA_UPDATE_ID_LEN: usize = ID_LEN;

/// The length in bytes of an image digest.
pub const PAMOJA_UPDATE_DIGEST_LEN: usize = DIGEST_LEN;

/// The manifest structure version this build writes.
pub const PAMOJA_UPDATE_STRUCTURE_VERSION: u8 = STRUCTURE_VERSION;

/// The payload format meaning the payload is the image itself, byte for byte.
pub const PAMOJA_UPDATE_FORMAT_RAW: u8 = 1;

/// What a release says about itself, and what a device checks it against.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaManifest {
    /// Which iteration of the manifest format this is.
    pub structure_version: u8,
    /// Rises with every release, which is what stops an older image being
    /// replayed at a device.
    pub sequence: u64,
    /// Who built the image.
    pub vendor_id: [u8; PAMOJA_UPDATE_ID_LEN],
    /// Which kind of device it is for.
    pub class_id: [u8; PAMOJA_UPDATE_ID_LEN],
    /// How the payload is encoded, currently only
    /// [`PAMOJA_UPDATE_FORMAT_RAW`].
    pub format: u8,
    /// Which slot the payload belongs in.
    pub storage: u8,
    /// The SHA-256 of the payload, which every other guarantee rests on.
    pub digest: [u8; PAMOJA_UPDATE_DIGEST_LEN],
    /// The payload length in bytes, known before a single byte is accepted.
    pub size: u32,
    /// When this release stops being offered, in seconds since the Unix epoch,
    /// or `0` to never expire.
    pub expires: u64,
}

/// A statement, signed by the anchor, that a second key may sign releases.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaDelegation {
    /// Rises with every rotation, so a retired key cannot be reinstated by
    /// replaying the statement that once authorised it.
    pub epoch: u64,
    /// The public key that may sign manifests while this delegation stands.
    pub release_key: [u8; PAMOJA_KEY_LEN],
    /// When the delegation stops being honoured, in seconds since the Unix
    /// epoch, or `0` to never expire.
    pub expires: u64,
}

/// Who a device is, and whose signature it trusts.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaDevice {
    /// Who built this firmware.
    pub vendor_id: [u8; PAMOJA_UPDATE_ID_LEN],
    /// What kind of device this is.
    pub class_id: [u8; PAMOJA_UPDATE_ID_LEN],
    /// The [`PAMOJA_KEY_LEN`]-byte key this device anchors its trust in.
    pub anchor: [u8; PAMOJA_KEY_LEN],
}

/// What a device believes about one slot.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaSlotState {
    /// Nothing has been written here.
    Empty = 0,
    /// An image is arriving, and `written` says how much of it has.
    Receiving = 1,
    /// A complete image that matched its manifest, not yet tried.
    Staged = 2,
    /// Being tried for the first time; it reverts unless it confirms.
    Pending = 3,
    /// Tried and confirmed working.
    Confirmed = 4,
    /// Tried and did not confirm, so it will not be tried again.
    Failed = 5,
}

/// The record a device keeps about one slot, durable across a reboot.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSlotRecord {
    /// The state of the slot.
    pub state: PamojaSlotState,
    /// The sequence number of the image in the slot.
    pub sequence: u64,
    /// The length of the image in bytes.
    pub size: u32,
    /// The digest of the image.
    pub digest: [u8; PAMOJA_UPDATE_DIGEST_LEN],
    /// How many bytes have been stored, which is where a resumed transfer picks
    /// up.
    pub written: u32,
}

/// What a bootloader should do with what it found.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaBootAction {
    /// Nothing new to try; run the confirmed image.
    Confirmed = 0,
    /// A staged image is being tried for the first time.
    Trying = 1,
    /// A pending image never confirmed, so it was failed.
    Reverted = 2,
}

/// The decision a device made at boot, already recorded before it was returned.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaBoot {
    /// What the bootloader should do.
    pub action: PamojaBootAction,
    /// The image the decision is about, which for
    /// [`PamojaBootAction::Reverted`] is the one that failed.
    pub slot: u8,
    /// The slot to run. It is the same as `slot` for anything but
    /// [`PamojaBootAction::Reverted`].
    pub fallback: u8,
}

/// An opaque handle that hashes an image as it arrives.
///
/// Create it with [`pamoja_image_verifier_new`], feed it with
/// [`pamoja_image_verifier_update`], and settle it with
/// [`pamoja_image_verifier_finish`], which consumes the handle.
pub struct PamojaImageVerifier {
    verifier: ImageVerifier,
}

/// An opaque handle to a device slots and the rules applied to them.
///
/// Create it with [`pamoja_updater_new`] and release it with
/// [`pamoja_updater_free`].
pub struct PamojaUpdater {
    updater: Updater<MemoryStore>,
    staging: Option<Staging>,
}

/// The transfer an updater is part-way through, remembered between calls.
struct Staging {
    envelope: Vec<u8>,
    now: Option<u64>,
}

/// Encodes the body of a manifest, which is the part a signature covers.
///
/// # Arguments
///
/// * `manifest` - the manifest to encode.
///
/// # Returns
///
/// A buffer the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or null if the manifest
/// carries a payload format this build cannot write.
#[no_mangle]
pub extern "C" fn pamoja_manifest_encode(manifest: PamojaManifest) -> *mut PamojaBuffer {
    let Ok(manifest) = rust_manifest(manifest) else {
        return ptr::null_mut();
    };
    let mut buf = [0u8; MANIFEST_MAX];
    match manifest.encode(&mut buf) {
        Ok(written) => PamojaBuffer::into_raw(buf[..written].to_vec()),
        Err(refusal) => {
            refuse(refusal);
            ptr::null_mut()
        }
    }
}

/// Reads a manifest body back from its bytes.
///
/// This reads what a manifest claims; it proves nothing about who wrote it. Use
/// [`pamoja_envelope_verify`] to read one whose signature has been checked.
///
/// # Arguments
///
/// * `bytes` - the encoded manifest body.
/// * `len` - the length of `bytes`.
/// * `out_manifest` - receives the decoded manifest.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `bytes` must point to at least `len` readable bytes, or be null when `len` is
/// 0, and `out_manifest` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_manifest_decode(
    bytes: *const u8,
    len: usize,
    out_manifest: *mut PamojaManifest,
) -> PamojaStatus {
    let bytes = match read_bytes(bytes, len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    if out_manifest.is_null() {
        set_last_error("out_manifest must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match Manifest::decode(&bytes) {
        Ok(manifest) => {
            *out_manifest = boundary_manifest(&manifest);
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Signs a manifest into the envelope that is offered to a device.
///
/// # Arguments
///
/// * `manifest` - the manifest to sign.
/// * `author` - the identity signing the release.
///
/// # Returns
///
/// A buffer the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or null on failure.
///
/// # Safety
///
/// `author` must be a live handle from
/// [`pamoja_device_identity_new`](crate::security::pamoja_device_identity_new),
/// or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_manifest_sign(
    manifest: PamojaManifest,
    author: *const PamojaDeviceIdentity,
) -> *mut PamojaBuffer {
    let Ok(manifest) = rust_manifest(manifest) else {
        return ptr::null_mut();
    };
    let Some(author) = identity_handle(author) else {
        return ptr::null_mut();
    };
    let mut buf = [0u8; ENVELOPE_MAX];
    match manifest.sign(&author.inner, &mut buf) {
        Ok(written) => PamojaBuffer::into_raw(buf[..written].to_vec()),
        Err(refusal) => {
            refuse(refusal);
            ptr::null_mut()
        }
    }
}

/// Verifies an envelope against a key and reads the manifest inside it.
///
/// # Arguments
///
/// * `bytes` - the signed envelope.
/// * `len` - the length of `bytes`.
/// * `public_key` - the [`PAMOJA_KEY_LEN`]-byte key expected to have signed it.
/// * `out_manifest` - receives the verified manifest.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] if the signature is from that key, or
/// [`PamojaStatus::Auth`] if it is not.
///
/// # Safety
///
/// `bytes` must point to at least `len` readable bytes or be null when `len` is
/// 0, `public_key` must point to at least [`PAMOJA_KEY_LEN`] readable bytes, and
/// `out_manifest` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_envelope_verify(
    bytes: *const u8,
    len: usize,
    public_key: *const u8,
    out_manifest: *mut PamojaManifest,
) -> PamojaStatus {
    let bytes = match read_bytes(bytes, len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let public = match read_public(public_key) {
        Ok(public) => public,
        Err(status) => return status,
    };
    if out_manifest.is_null() {
        set_last_error("out_manifest must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }

    let envelope = match Envelope::decode(&bytes) {
        Ok(envelope) => envelope,
        Err(refusal) => return refuse(refusal),
    };
    match envelope.verify(&public) {
        Ok(manifest) => {
            *out_manifest = boundary_manifest(&manifest);
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Copies out the signed body of an envelope, without checking the signature.
///
/// This is what a gateway relays onward unchanged, and what a device hashes when
/// it checks the signature itself.
///
/// # Arguments
///
/// * `bytes` - the signed envelope.
/// * `len` - the length of `bytes`.
///
/// # Returns
///
/// A buffer the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or null if the envelope is
/// malformed.
///
/// # Safety
///
/// `bytes` must point to at least `len` readable bytes, or be null when `len` is
/// 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_envelope_body(bytes: *const u8, len: usize) -> *mut PamojaBuffer {
    let Ok(bytes) = read_bytes(bytes, len) else {
        return ptr::null_mut();
    };
    match Envelope::decode(&bytes) {
        Ok(envelope) => PamojaBuffer::into_raw(envelope.body().to_vec()),
        Err(refusal) => {
            refuse(refusal);
            ptr::null_mut()
        }
    }
}

/// Signs a delegation, naming a release key the anchor stands behind.
///
/// # Arguments
///
/// * `delegation` - the statement to sign.
/// * `anchor` - the anchor identity, which is the root of the trust.
///
/// # Returns
///
/// A buffer the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or null on failure.
///
/// # Safety
///
/// `anchor` must be a live identity handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_delegation_sign(
    delegation: PamojaDelegation,
    anchor: *const PamojaDeviceIdentity,
) -> *mut PamojaBuffer {
    let Some(anchor) = identity_handle(anchor) else {
        return ptr::null_mut();
    };
    let delegation = Delegation {
        epoch: delegation.epoch,
        release_key: delegation.release_key,
        expires: delegation.expires,
    };
    let mut buf = [0u8; DELEGATION_MAX];
    match delegation.sign(&anchor.inner, &mut buf) {
        Ok(written) => PamojaBuffer::into_raw(buf[..written].to_vec()),
        Err(refusal) => {
            refuse(refusal);
            ptr::null_mut()
        }
    }
}

/// Opens a signed delegation against the anchor that should have signed it.
///
/// # Arguments
///
/// * `bytes` - the signed delegation envelope.
/// * `len` - the length of `bytes`.
/// * `anchor_public_key` - the [`PAMOJA_KEY_LEN`]-byte anchor key.
/// * `out_delegation` - receives the verified delegation.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] if the delegation is from the anchor, or
/// [`PamojaStatus::Auth`] if it is not.
///
/// # Safety
///
/// `bytes` must point to at least `len` readable bytes or be null when `len` is
/// 0, `anchor_public_key` must point to at least [`PAMOJA_KEY_LEN`] readable
/// bytes, and `out_delegation` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_delegation_open(
    bytes: *const u8,
    len: usize,
    anchor_public_key: *const u8,
    out_delegation: *mut PamojaDelegation,
) -> PamojaStatus {
    let bytes = match read_bytes(bytes, len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let anchor = match read_public(anchor_public_key) {
        Ok(anchor) => anchor,
        Err(status) => return status,
    };
    if out_delegation.is_null() {
        set_last_error("out_delegation must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match Delegation::open(&bytes, &anchor) {
        Ok(delegation) => {
            *out_delegation = boundary_delegation(&delegation);
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Creates a verifier that hashes an image against what a manifest declares.
///
/// # Arguments
///
/// * `manifest` - the manifest describing the image.
///
/// # Returns
///
/// A handle the caller must settle with [`pamoja_image_verifier_finish`] or
/// abandon with [`pamoja_image_verifier_free`], or null if the manifest carries
/// Hashes a complete image, for a publisher filling in a manifest.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with the 32-byte SHA-256 written to `out_digest`.
///
/// # Safety
///
/// `image` must point to at least `image_len` readable bytes, or be null when
/// `image_len` is 0, and `out_digest` must point to at least 32 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_image_digest(
    image: *const u8,
    image_len: usize,
    out_digest: *mut u8,
) -> PamojaStatus {
    if out_digest.is_null() {
        set_last_error("out_digest must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let image = match read_bytes(image, image_len) {
        Ok(image) => image,
        Err(status) => return status,
    };
    let digest = pamoja_update::image_digest(&image);
    core::ptr::copy_nonoverlapping(digest.as_ptr(), out_digest, digest.len());
    PamojaStatus::Ok
}

/// a payload format this build cannot apply.
#[no_mangle]
pub extern "C" fn pamoja_image_verifier_new(manifest: PamojaManifest) -> *mut PamojaImageVerifier {
    let Ok(manifest) = rust_manifest(manifest) else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaImageVerifier {
        verifier: ImageVerifier::new(&manifest),
    }))
}

/// Takes the next piece of the image.
///
/// # Arguments
///
/// * `verifier` - the verifier.
/// * `chunk` - the next bytes of the image, in order.
/// * `len` - the length of `chunk`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the chunk is hashed, or a failure if more bytes
/// have arrived than the manifest declared.
///
/// # Safety
///
/// `verifier` must be a live handle from [`pamoja_image_verifier_new`], and
/// `chunk` must point to at least `len` readable bytes, or be null when `len` is
/// 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_image_verifier_update(
    verifier: *mut PamojaImageVerifier,
    chunk: *const u8,
    len: usize,
) -> PamojaStatus {
    if verifier.is_null() {
        set_last_error("verifier must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let chunk = match read_bytes(chunk, len) {
        Ok(chunk) => chunk,
        Err(status) => return status,
    };
    match (*verifier).verifier.update(&chunk) {
        Ok(()) => PamojaStatus::Ok,
        Err(refusal) => refuse(refusal),
    }
}

/// Settles an image against its manifest, consuming the verifier.
///
/// The handle is released whether the image matched or not, so it must not be
/// used again after this call and must not also be passed to
/// [`pamoja_image_verifier_free`].
///
/// # Arguments
///
/// * `verifier` - the verifier, consumed by this call.
/// * `out_size` - receives the length of the image that was hashed.
/// * `out_digest` - receives [`PAMOJA_UPDATE_DIGEST_LEN`] bytes of digest.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] if the image is the one the manifest described, or a
/// failure naming the rule it broke.
///
/// # Safety
///
/// `verifier` must be a live handle from [`pamoja_image_verifier_new`] that has
/// not been freed, `out_size` must be writable or null, and `out_digest` must
/// point to at least [`PAMOJA_UPDATE_DIGEST_LEN`] writable bytes or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_image_verifier_finish(
    verifier: *mut PamojaImageVerifier,
    out_size: *mut u32,
    out_digest: *mut u8,
) -> PamojaStatus {
    if verifier.is_null() {
        set_last_error("verifier must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let owned = Box::from_raw(verifier);
    match owned.verifier.finish() {
        Ok(verified) => {
            write_verified(&verified, out_size, out_digest);
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Releases a verifier handle that will not be settled.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `verifier` must be a handle from [`pamoja_image_verifier_new`] that has not
/// already been freed or passed to [`pamoja_image_verifier_finish`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_image_verifier_free(verifier: *mut PamojaImageVerifier) {
    if !verifier.is_null() {
        drop(Box::from_raw(verifier));
    }
}

/// Creates an updater over a device slots.
///
/// # Arguments
///
/// * `device` - who the device is and whose signature it trusts.
/// * `slot_count` - how many slots the device has.
/// * `slot_capacity` - how many bytes each slot holds.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_updater_free`], or null if the
/// anchor is not a valid public key.
#[no_mangle]
pub extern "C" fn pamoja_updater_new(
    device: PamojaDevice,
    slot_count: u8,
    slot_capacity: u32,
) -> *mut PamojaUpdater {
    // Safety: the anchor is a fixed-size array inside a value that crossed by
    // value, so the pointer is always valid for the key length.
    let anchor = unsafe { read_public(device.anchor.as_ptr()) };
    let Ok(anchor) = anchor else {
        return ptr::null_mut();
    };
    let device = Device {
        vendor_id: device.vendor_id,
        class_id: device.class_id,
        anchor,
    };
    Box::into_raw(Box::new(PamojaUpdater {
        updater: Updater::new(device, MemoryStore::new(slot_count, slot_capacity)),
        staging: None,
    }))
}

/// Adopts a delegation, so releases signed by the key it names are accepted.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `bytes` - the signed delegation envelope.
/// * `len` - the length of `bytes`.
/// * `has_now` - `true` if the device has a clock, `false` if it does not.
/// * `now` - seconds since the Unix epoch, read only when `has_now` is `true`.
/// * `out_delegation` - receives the adopted delegation, or may be null.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] if the delegation was signed by the anchor, is newer
/// than the one held, and has not expired.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], `bytes` must
/// point to at least `len` readable bytes or be null when `len` is 0, and
/// `out_delegation` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_adopt(
    updater: *mut PamojaUpdater,
    bytes: *const u8,
    len: usize,
    has_now: bool,
    now: u64,
    out_delegation: *mut PamojaDelegation,
) -> PamojaStatus {
    if updater.is_null() {
        set_last_error("updater must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match read_bytes(bytes, len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match (*updater).updater.adopt(&bytes, clock(has_now, now)) {
        Ok(delegation) => {
            if !out_delegation.is_null() {
                *out_delegation = boundary_delegation(&delegation);
            }
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Reads the delegation an updater currently honours.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `out_delegation` - receives the delegation when there is one.
///
/// # Returns
///
/// `true` if a delegation is held and was written out, or `false` if releases
/// must be signed by the anchor itself.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], and
/// `out_delegation` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_delegation(
    updater: *const PamojaUpdater,
    out_delegation: *mut PamojaDelegation,
) -> bool {
    if updater.is_null() {
        return false;
    }
    match (*updater).updater.delegation() {
        Some(delegation) => {
            if !out_delegation.is_null() {
                *out_delegation = boundary_delegation(&delegation);
            }
            true
        }
        None => false,
    }
}

/// Reads the highest sequence number the device already holds.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `out_sequence` - receives the sequence number.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], and
/// `out_sequence` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_installed_sequence(
    updater: *const PamojaUpdater,
    out_sequence: *mut u64,
) -> PamojaStatus {
    if updater.is_null() || out_sequence.is_null() {
        set_last_error("updater and out_sequence must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match (*updater).updater.installed_sequence() {
        Ok(sequence) => {
            *out_sequence = sequence;
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Reads what a device believes about one slot.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `slot` - the slot to read.
/// * `out_record` - receives the record.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, or a failure if the device has no such slot.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], and `out_record`
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_slot_record(
    updater: *const PamojaUpdater,
    slot: u8,
    out_record: *mut PamojaSlotRecord,
) -> PamojaStatus {
    if updater.is_null() || out_record.is_null() {
        set_last_error("updater and out_record must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match (*updater).updater.store().record(slot) {
        Ok(record) => {
            *out_record = boundary_record(&record);
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Returns how many slots a device has.
///
/// # Arguments
///
/// * `updater` - the updater.
///
/// # Returns
///
/// The slot count, or 0 if `updater` is null.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_slot_count(updater: *const PamojaUpdater) -> u8 {
    if updater.is_null() {
        return 0;
    }
    (*updater).updater.store().slot_count()
}

/// Records that a slot already holds a confirmed image at a sequence number.
///
/// This is how a device that shipped with firmware tells the updater what it is
/// running, so the rollback rule has something to compare against.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `slot` - the slot holding the running image.
/// * `sequence` - the sequence number of that image.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_provision(
    updater: *mut PamojaUpdater,
    slot: u8,
    sequence: u64,
) -> PamojaStatus {
    if updater.is_null() {
        set_last_error("updater must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match (*updater).updater.provision(slot, sequence) {
        Ok(()) => PamojaStatus::Ok,
        Err(refusal) => refuse(refusal),
    }
}

/// Checks a manifest and stages an image that is already held whole.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `envelope` - the signed manifest offered to this device.
/// * `envelope_len` - the length of `envelope`.
/// * `image` - the whole image.
/// * `image_len` - the length of `image`.
/// * `has_now` - `true` if the device has a clock.
/// * `now` - seconds since the Unix epoch, read only when `has_now` is `true`.
/// * `out_slot` - receives the slot the image was staged into.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, or a failure naming the rule that refused
/// the update.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], `envelope` and
/// `image` must point to at least their stated lengths of readable bytes or be
/// null when those lengths are 0, and `out_slot` must be writable or null.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_stage(
    updater: *mut PamojaUpdater,
    envelope: *const u8,
    envelope_len: usize,
    image: *const u8,
    image_len: usize,
    has_now: bool,
    now: u64,
    out_slot: *mut u8,
) -> PamojaStatus {
    if updater.is_null() {
        set_last_error("updater must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let envelope = match read_bytes(envelope, envelope_len) {
        Ok(envelope) => envelope,
        Err(status) => return status,
    };
    let image = match read_bytes(image, image_len) {
        Ok(image) => image,
        Err(status) => return status,
    };
    match (*updater)
        .updater
        .stage_at(&envelope, &image, clock(has_now, now))
    {
        Ok(slot) => {
            write_slot(slot, out_slot);
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Checks a manifest and opens the slot it names for a transfer in pieces.
///
/// Every check that can be made without the image runs here, so a release that
/// is not for this device, would roll it back, or does not fit is refused before
/// a byte of it is accepted.
///
/// The envelope is remembered until [`pamoja_updater_finish`], so the calls that
/// follow do not repeat it. Each of those reopens the transfer from what the
/// slot records, which is the same path a device takes after a reset, and is
/// what lets a transfer survive one.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `envelope` - the signed manifest offered to this device.
/// * `envelope_len` - the length of `envelope`.
/// * `has_now` - `true` if the device has a clock.
/// * `now` - seconds since the Unix epoch, read only when `has_now` is `true`.
/// * `out_slot` - receives the slot the image will be written into.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], `envelope` must
/// point to at least `envelope_len` readable bytes or be null when it is 0, and
/// `out_slot` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_begin(
    updater: *mut PamojaUpdater,
    envelope: *const u8,
    envelope_len: usize,
    has_now: bool,
    now: u64,
    out_slot: *mut u8,
) -> PamojaStatus {
    if updater.is_null() {
        set_last_error("updater must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let envelope = match read_bytes(envelope, envelope_len) {
        Ok(envelope) => envelope,
        Err(status) => return status,
    };
    let now = clock(has_now, now);

    let slot = match (*updater).updater.begin_at(&envelope, now) {
        Ok(staging) => staging.manifest().storage,
        Err(refusal) => return refuse(refusal),
    };
    (*updater).staging = Some(Staging { envelope, now });
    write_slot(slot, out_slot);
    PamojaStatus::Ok
}

/// Takes the next piece of an image opened with [`pamoja_updater_begin`].
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `chunk` - the next bytes of the image, in order.
/// * `len` - the length of `chunk`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the chunk is stored and its progress recorded.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], and `chunk` must
/// point to at least `len` readable bytes, or be null when `len` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_write(
    updater: *mut PamojaUpdater,
    chunk: *const u8,
    len: usize,
) -> PamojaStatus {
    if updater.is_null() {
        set_last_error("updater must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let chunk = match read_bytes(chunk, len) {
        Ok(chunk) => chunk,
        Err(status) => return status,
    };
    let Some((envelope, now)) = open_transfer(&*updater) else {
        return PamojaStatus::InvalidArgument;
    };
    match (*updater).updater.resume_at(&envelope, now) {
        Ok(mut staging) => match staging.write(&chunk) {
            Ok(()) => PamojaStatus::Ok,
            Err(refusal) => refuse(refusal),
        },
        Err(refusal) => refuse(refusal),
    }
}

/// Reports how much of an opened image has arrived.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `out_written` - receives the bytes stored so far.
/// * `out_total` - receives the total the manifest declares.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], and the output
/// pointers must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_progress(
    updater: *mut PamojaUpdater,
    out_written: *mut u32,
    out_total: *mut u32,
) -> PamojaStatus {
    if updater.is_null() {
        set_last_error("updater must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some((envelope, now)) = open_transfer(&*updater) else {
        return PamojaStatus::InvalidArgument;
    };
    match (*updater).updater.resume_at(&envelope, now) {
        Ok(staging) => {
            let (written, total) = staging.progress();
            if !out_written.is_null() {
                *out_written = written;
            }
            if !out_total.is_null() {
                *out_total = total;
            }
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Finishes an opened image and marks the slot bootable if it matched.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `out_slot` - receives the slot now holding a staged image.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, or a failure if the image is not the one the
/// manifest described, which leaves the slot unbootable.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], and `out_slot`
/// must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_finish(
    updater: *mut PamojaUpdater,
    out_slot: *mut u8,
) -> PamojaStatus {
    if updater.is_null() {
        set_last_error("updater must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some((envelope, now)) = open_transfer(&*updater) else {
        return PamojaStatus::InvalidArgument;
    };
    let outcome = match (*updater).updater.resume_at(&envelope, now) {
        Ok(staging) => staging.finish(),
        Err(refusal) => Err(refusal),
    };
    match outcome {
        Ok(slot) => {
            (*updater).staging = None;
            write_slot(slot, out_slot);
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Decides what to run, and records that decision before returning it.
///
/// Call this once per boot, before jumping to an image. A staged image becomes
/// pending here, so a device that resets before confirming reverts on the next
/// call rather than trying a broken image forever.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `out_boot` - receives the decision.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, or a failure if there is nothing to fall
/// back to.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], and `out_boot`
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_on_boot(
    updater: *mut PamojaUpdater,
    out_boot: *mut PamojaBoot,
) -> PamojaStatus {
    if updater.is_null() || out_boot.is_null() {
        set_last_error("updater and out_boot must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match (*updater).updater.on_boot() {
        Ok(boot) => {
            *out_boot = boundary_boot(boot);
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Confirms the pending image, so it will be run from now on.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `out_slot` - receives the slot that is now confirmed.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], and `out_slot`
/// must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_confirm(
    updater: *mut PamojaUpdater,
    out_slot: *mut u8,
) -> PamojaStatus {
    if updater.is_null() {
        set_last_error("updater must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match (*updater).updater.confirm() {
        Ok(slot) => {
            write_slot(slot, out_slot);
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Fails the pending image and goes back to the confirmed one.
///
/// # Arguments
///
/// * `updater` - the updater.
/// * `out_slot` - receives the slot to fall back to.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, or a failure if there is nothing to fall
/// back to.
///
/// # Safety
///
/// `updater` must be a live handle from [`pamoja_updater_new`], and `out_slot`
/// must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_revert(
    updater: *mut PamojaUpdater,
    out_slot: *mut u8,
) -> PamojaStatus {
    if updater.is_null() {
        set_last_error("updater must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match (*updater).updater.revert() {
        Ok(slot) => {
            write_slot(slot, out_slot);
            PamojaStatus::Ok
        }
        Err(refusal) => refuse(refusal),
    }
}

/// Releases an updater handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `updater` must be a handle from [`pamoja_updater_new`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_updater_free(updater: *mut PamojaUpdater) {
    if !updater.is_null() {
        drop(Box::from_raw(updater));
    }
}

/// Records a refusal as the last error and maps it onto a status.
fn refuse(refusal: Refusal) -> PamojaStatus {
    let error = pamoja_core::Error::from(refusal);
    let status = PamojaStatus::from_error(&error);
    set_last_error(refusal.reason().to_owned());
    status
}

/// Turns the two clock arguments into the optional the crate takes.
fn clock(has_now: bool, now: u64) -> Option<u64> {
    has_now.then_some(now)
}

/// Copies out a slot number when the caller asked for one.
///
/// # Safety
///
/// `out_slot` must be writable, or null.
unsafe fn write_slot(slot: u8, out_slot: *mut u8) {
    if !out_slot.is_null() {
        *out_slot = slot;
    }
}

/// Copies out what a settled image turned out to be.
///
/// # Safety
///
/// `out_size` must be writable or null, and `out_digest` must point to at least
/// [`PAMOJA_UPDATE_DIGEST_LEN`] writable bytes, or be null.
unsafe fn write_verified(verified: &Verified, out_size: *mut u32, out_digest: *mut u8) {
    if !out_size.is_null() {
        *out_size = verified.size();
    }
    if !out_digest.is_null() {
        let digest = verified.digest();
        ptr::copy_nonoverlapping(digest.as_ptr(), out_digest, PAMOJA_UPDATE_DIGEST_LEN);
    }
}

/// Borrows the envelope an updater is part-way through, if there is one.
fn open_transfer(updater: &PamojaUpdater) -> Option<(Vec<u8>, Option<u64>)> {
    match &updater.staging {
        Some(staging) => Some((staging.envelope.clone(), staging.now)),
        None => {
            set_last_error("no transfer is open; call pamoja_updater_begin first".to_owned());
            None
        }
    }
}

/// Rebuilds the Rust manifest from the fields that crossed the boundary.
fn rust_manifest(manifest: PamojaManifest) -> Result<Manifest, PamojaStatus> {
    if manifest.format != PAMOJA_UPDATE_FORMAT_RAW {
        return Err(refuse(Refusal::UnsupportedVersion));
    }
    Ok(Manifest {
        structure_version: manifest.structure_version,
        sequence: manifest.sequence,
        vendor_id: manifest.vendor_id,
        class_id: manifest.class_id,
        format: PayloadFormat::Raw,
        storage: manifest.storage,
        digest: manifest.digest,
        size: manifest.size,
        expires: manifest.expires,
    })
}

/// Maps a Rust manifest onto the value that crosses the boundary.
fn boundary_manifest(manifest: &Manifest) -> PamojaManifest {
    PamojaManifest {
        structure_version: manifest.structure_version,
        sequence: manifest.sequence,
        vendor_id: manifest.vendor_id,
        class_id: manifest.class_id,
        format: manifest.format as u8,
        storage: manifest.storage,
        digest: manifest.digest,
        size: manifest.size,
        expires: manifest.expires,
    }
}

/// Maps a Rust delegation onto the value that crosses the boundary.
fn boundary_delegation(delegation: &Delegation) -> PamojaDelegation {
    PamojaDelegation {
        epoch: delegation.epoch,
        release_key: delegation.release_key,
        expires: delegation.expires,
    }
}

/// Maps a Rust slot record onto the value that crosses the boundary.
fn boundary_record(record: &SlotRecord) -> PamojaSlotRecord {
    PamojaSlotRecord {
        state: match record.state {
            SlotState::Empty => PamojaSlotState::Empty,
            SlotState::Receiving => PamojaSlotState::Receiving,
            SlotState::Staged => PamojaSlotState::Staged,
            SlotState::Pending => PamojaSlotState::Pending,
            SlotState::Confirmed => PamojaSlotState::Confirmed,
            SlotState::Failed => PamojaSlotState::Failed,
        },
        sequence: record.sequence,
        size: record.size,
        digest: record.digest,
        written: record.written,
    }
}

/// Maps a Rust boot decision onto the value that crosses the boundary.
fn boundary_boot(boot: Boot) -> PamojaBoot {
    match boot {
        Boot::Confirmed(slot) => PamojaBoot {
            action: PamojaBootAction::Confirmed,
            slot,
            fallback: slot,
        },
        Boot::Trying(slot) => PamojaBoot {
            action: PamojaBootAction::Trying,
            slot,
            fallback: slot,
        },
        Boot::Reverted { failed, fallback } => PamojaBoot {
            action: PamojaBootAction::Reverted,
            slot: failed,
            fallback,
        },
    }
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::security::{
        pamoja_device_identity_free, pamoja_device_identity_new, pamoja_device_identity_public_key,
    };
    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};

    /// Builds an identity and its public key from a repeated-byte seed.
    unsafe fn signer(seed: u8) -> (*mut PamojaDeviceIdentity, [u8; PAMOJA_KEY_LEN]) {
        let seed = [seed; PAMOJA_KEY_LEN];
        let identity = pamoja_device_identity_new(seed.as_ptr(), seed.len());
        assert!(!identity.is_null());
        let mut public = [0u8; PAMOJA_KEY_LEN];
        assert_eq!(
            pamoja_device_identity_public_key(identity, public.as_mut_ptr()),
            PamojaStatus::Ok
        );
        (identity, public)
    }

    /// Copies a buffer out and releases it.
    unsafe fn take(buffer: *mut PamojaBuffer) -> Vec<u8> {
        assert!(!buffer.is_null());
        let bytes =
            std::slice::from_raw_parts(pamoja_buffer_data(buffer), pamoja_buffer_len(buffer))
                .to_vec();
        pamoja_buffer_free(buffer);
        bytes
    }

    /// Describes a release of `image` at `sequence`.
    fn manifest(image: &[u8], sequence: u64, storage: u8) -> PamojaManifest {
        let digest: [u8; PAMOJA_UPDATE_DIGEST_LEN] = Sha256::digest(image).into();
        PamojaManifest {
            structure_version: PAMOJA_UPDATE_STRUCTURE_VERSION,
            sequence,
            vendor_id: [1; PAMOJA_UPDATE_ID_LEN],
            class_id: [2; PAMOJA_UPDATE_ID_LEN],
            format: PAMOJA_UPDATE_FORMAT_RAW,
            storage,
            digest,
            size: image.len() as u32,
            expires: 0,
        }
    }

    /// Builds an updater that trusts `anchor` and has two slots.
    fn updater(anchor: [u8; PAMOJA_KEY_LEN]) -> *mut PamojaUpdater {
        let handle = pamoja_updater_new(
            PamojaDevice {
                vendor_id: [1; PAMOJA_UPDATE_ID_LEN],
                class_id: [2; PAMOJA_UPDATE_ID_LEN],
                anchor,
            },
            2,
            4096,
        );
        assert!(!handle.is_null());
        handle
    }

    #[test]
    fn a_signed_release_stages_boots_and_confirms() {
        unsafe {
            let (author, anchor) = signer(3);
            let device = updater(anchor);
            assert_eq!(pamoja_updater_slot_count(device), 2);
            assert_eq!(pamoja_updater_provision(device, 0, 1), PamojaStatus::Ok);

            let image = vec![0xa5u8; 512];
            let envelope = take(pamoja_manifest_sign(manifest(&image, 2, 1), author));

            let mut slot = 0u8;
            assert_eq!(
                pamoja_updater_stage(
                    device,
                    envelope.as_ptr(),
                    envelope.len(),
                    image.as_ptr(),
                    image.len(),
                    false,
                    0,
                    &mut slot,
                ),
                PamojaStatus::Ok
            );
            assert_eq!(slot, 1);

            let mut boot = PamojaBoot {
                action: PamojaBootAction::Confirmed,
                slot: 0,
                fallback: 0,
            };
            assert_eq!(pamoja_updater_on_boot(device, &mut boot), PamojaStatus::Ok);
            assert_eq!(boot.action, PamojaBootAction::Trying);
            assert_eq!(boot.slot, 1);

            let mut confirmed = 0u8;
            assert_eq!(
                pamoja_updater_confirm(device, &mut confirmed),
                PamojaStatus::Ok
            );
            assert_eq!(confirmed, 1);

            let mut record = boundary_record(&SlotRecord::default());
            assert_eq!(
                pamoja_updater_slot_record(device, 1, &mut record),
                PamojaStatus::Ok
            );
            assert_eq!(record.state, PamojaSlotState::Confirmed);
            assert_eq!(record.size, image.len() as u32);

            pamoja_updater_free(device);
            pamoja_device_identity_free(author);
        }
    }

    #[test]
    fn an_image_arriving_in_pieces_reaches_the_same_place() {
        unsafe {
            let (author, anchor) = signer(4);
            let device = updater(anchor);
            assert_eq!(pamoja_updater_provision(device, 0, 1), PamojaStatus::Ok);

            let image = vec![0x5au8; 300];
            let envelope = take(pamoja_manifest_sign(manifest(&image, 2, 1), author));

            let mut slot = 0u8;
            assert_eq!(
                pamoja_updater_begin(
                    device,
                    envelope.as_ptr(),
                    envelope.len(),
                    false,
                    0,
                    &mut slot
                ),
                PamojaStatus::Ok
            );
            assert_eq!(slot, 1);

            for chunk in image.chunks(64) {
                assert_eq!(
                    pamoja_updater_write(device, chunk.as_ptr(), chunk.len()),
                    PamojaStatus::Ok
                );
            }

            let (mut written, mut total) = (0u32, 0u32);
            assert_eq!(
                pamoja_updater_progress(device, &mut written, &mut total),
                PamojaStatus::Ok
            );
            assert_eq!(written, image.len() as u32);
            assert_eq!(total, image.len() as u32);

            let mut staged = 0u8;
            assert_eq!(pamoja_updater_finish(device, &mut staged), PamojaStatus::Ok);
            assert_eq!(staged, 1);

            pamoja_updater_free(device);
            pamoja_device_identity_free(author);
        }
    }

    #[test]
    fn a_release_from_an_untrusted_key_is_refused() {
        unsafe {
            let (_, anchor) = signer(5);
            let (impostor, _) = signer(6);
            let device = updater(anchor);
            assert_eq!(pamoja_updater_provision(device, 0, 1), PamojaStatus::Ok);

            let image = vec![0u8; 16];
            let envelope = take(pamoja_manifest_sign(manifest(&image, 2, 1), impostor));

            assert_eq!(
                pamoja_updater_stage(
                    device,
                    envelope.as_ptr(),
                    envelope.len(),
                    image.as_ptr(),
                    image.len(),
                    false,
                    0,
                    ptr::null_mut(),
                ),
                PamojaStatus::Auth
            );

            pamoja_updater_free(device);
            pamoja_device_identity_free(impostor);
        }
    }

    #[test]
    fn an_older_release_cannot_roll_a_device_back() {
        unsafe {
            let (author, anchor) = signer(7);
            let device = updater(anchor);
            assert_eq!(pamoja_updater_provision(device, 0, 9), PamojaStatus::Ok);

            let image = vec![0u8; 16];
            let envelope = take(pamoja_manifest_sign(manifest(&image, 4, 1), author));

            assert_eq!(
                pamoja_updater_stage(
                    device,
                    envelope.as_ptr(),
                    envelope.len(),
                    image.as_ptr(),
                    image.len(),
                    false,
                    0,
                    ptr::null_mut(),
                ),
                PamojaStatus::Auth
            );

            let mut sequence = 0u64;
            assert_eq!(
                pamoja_updater_installed_sequence(device, &mut sequence),
                PamojaStatus::Ok
            );
            assert_eq!(sequence, 9);

            pamoja_updater_free(device);
            pamoja_device_identity_free(author);
        }
    }

    #[test]
    fn a_delegated_key_may_sign_releases() {
        unsafe {
            let (anchor_identity, anchor) = signer(8);
            let (release_identity, release_key) = signer(9);
            let device = updater(anchor);
            assert_eq!(pamoja_updater_provision(device, 0, 1), PamojaStatus::Ok);

            let statement = PamojaDelegation {
                epoch: 1,
                release_key,
                expires: 0,
            };
            let signed = take(pamoja_delegation_sign(statement, anchor_identity));

            let mut opened = PamojaDelegation {
                epoch: 0,
                release_key: [0; PAMOJA_KEY_LEN],
                expires: 0,
            };
            assert_eq!(
                pamoja_delegation_open(signed.as_ptr(), signed.len(), anchor.as_ptr(), &mut opened),
                PamojaStatus::Ok
            );
            assert_eq!(opened.release_key, release_key);

            assert_eq!(
                pamoja_updater_adopt(
                    device,
                    signed.as_ptr(),
                    signed.len(),
                    false,
                    0,
                    ptr::null_mut()
                ),
                PamojaStatus::Ok
            );
            assert!(pamoja_updater_delegation(device, ptr::null_mut()));

            let image = vec![7u8; 64];
            let envelope = take(pamoja_manifest_sign(
                manifest(&image, 2, 1),
                release_identity,
            ));
            assert_eq!(
                pamoja_updater_stage(
                    device,
                    envelope.as_ptr(),
                    envelope.len(),
                    image.as_ptr(),
                    image.len(),
                    false,
                    0,
                    ptr::null_mut(),
                ),
                PamojaStatus::Ok
            );

            pamoja_updater_free(device);
            pamoja_device_identity_free(release_identity);
            pamoja_device_identity_free(anchor_identity);
        }
    }

    #[test]
    fn a_manifest_survives_a_round_trip_and_verifies() {
        unsafe {
            let (author, public) = signer(10);
            let image = vec![3u8; 128];
            let want = manifest(&image, 5, 1);

            let body = take(pamoja_manifest_encode(want));
            let mut decoded = want;
            assert_eq!(
                pamoja_manifest_decode(body.as_ptr(), body.len(), &mut decoded),
                PamojaStatus::Ok
            );
            assert_eq!(decoded, want);

            let envelope = take(pamoja_manifest_sign(want, author));
            assert_eq!(
                take(pamoja_envelope_body(envelope.as_ptr(), envelope.len())),
                body
            );

            let mut verified = want;
            assert_eq!(
                pamoja_envelope_verify(
                    envelope.as_ptr(),
                    envelope.len(),
                    public.as_ptr(),
                    &mut verified
                ),
                PamojaStatus::Ok
            );
            assert_eq!(verified, want);

            pamoja_device_identity_free(author);
        }
    }

    #[test]
    fn a_verifier_refuses_an_image_that_is_not_the_one_described() {
        unsafe {
            let image = vec![1u8; 64];
            let want = manifest(&image, 2, 1);

            let verifier = pamoja_image_verifier_new(want);
            assert_eq!(
                pamoja_image_verifier_update(verifier, image.as_ptr(), image.len()),
                PamojaStatus::Ok
            );
            let mut size = 0u32;
            let mut digest = [0u8; PAMOJA_UPDATE_DIGEST_LEN];
            assert_eq!(
                pamoja_image_verifier_finish(verifier, &mut size, digest.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(size, image.len() as u32);
            assert_eq!(digest, want.digest);

            let mut altered = image.clone();
            altered[0] ^= 0xff;
            let verifier = pamoja_image_verifier_new(want);
            assert_eq!(
                pamoja_image_verifier_update(verifier, altered.as_ptr(), altered.len()),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_image_verifier_finish(verifier, ptr::null_mut(), ptr::null_mut()),
                PamojaStatus::Auth
            );
        }
    }

    #[test]
    fn writing_without_opening_a_transfer_is_refused() {
        unsafe {
            let (_, anchor) = signer(11);
            let device = updater(anchor);

            assert_eq!(
                pamoja_updater_write(device, b"x".as_ptr(), 1),
                PamojaStatus::InvalidArgument
            );

            pamoja_updater_free(device);
        }
    }

    #[test]
    fn a_null_handle_is_refused_rather_than_dereferenced() {
        unsafe {
            assert!(pamoja_manifest_sign(manifest(&[], 1, 0), ptr::null()).is_null());
            assert_eq!(
                pamoja_updater_on_boot(ptr::null_mut(), ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_updater_write(ptr::null_mut(), b"x".as_ptr(), 1),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_updater_progress(ptr::null_mut(), ptr::null_mut(), ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_updater_finish(ptr::null_mut(), ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert!(!pamoja_updater_delegation(ptr::null(), ptr::null_mut()));
            assert_eq!(pamoja_updater_slot_count(ptr::null()), 0);
            pamoja_updater_free(ptr::null_mut());
            pamoja_image_verifier_free(ptr::null_mut());
        }
    }
}
