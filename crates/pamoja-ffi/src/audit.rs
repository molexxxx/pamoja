//! The C ABI for tamper-evident audit logs.
//!
//! These functions wrap [`pamoja_audit`] for callers that reach the SDK through
//! the flat C boundary: a log that signs each record and chains it to the one
//! before, and the two ways to check such a chain, one entry at a time as it
//! streams in or all at once over a batch that has already arrived.
//!
//! A log carries the position and hash it will chain the next entry onto, and an
//! entry owns its payload, so both cross as opaque handles. The identity that
//! signs and the key that checks come from the [`security`](crate::security)
//! capability, which this one builds on.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use pamoja_audit::{verify_chain, AuditLog, Entry, Verifier};

use crate::security::{identity_handle, read_public, PamojaDeviceIdentity};
use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// The length in bytes of an entry hash.
pub const PAMOJA_AUDIT_DIGEST_LEN: usize = 32;

/// The length in bytes of an entry signature.
pub const PAMOJA_AUDIT_SIGNATURE_LEN: usize = 64;

/// An opaque handle to one signed, chained record.
///
/// Obtain one from [`pamoja_audit_log_append`] or [`pamoja_audit_entry_from_bytes`],
/// and release it with [`pamoja_audit_entry_free`].
pub struct PamojaAuditEntry {
    entry: Entry,
}

/// An opaque handle to a log that signs and chains what it is given.
///
/// Create it with [`pamoja_audit_log_new`], or with
/// [`pamoja_audit_log_resume`] to carry on from a log that already has entries,
/// and release it with [`pamoja_audit_log_free`].
pub struct PamojaAuditLog {
    log: AuditLog,
}

/// An opaque handle that checks a chain one entry at a time as it arrives.
///
/// Create it with [`pamoja_audit_verifier_new`] and release it with
/// [`pamoja_audit_verifier_free`].
pub struct PamojaAuditVerifier {
    verifier: Verifier,
}

/// Creates a log that signs with a device identity and starts from nothing.
///
/// # Arguments
///
/// * `identity` - the identity whose signature each entry will carry.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_audit_log_free`], or null on
/// failure with the reason available from
/// [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `identity` must be a live handle from
/// [`pamoja_device_identity_new`](crate::security::pamoja_device_identity_new),
/// or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_log_new(
    identity: *const PamojaDeviceIdentity,
) -> *mut PamojaAuditLog {
    let Some(handle) = identity_handle(identity) else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaAuditLog {
        log: AuditLog::new(handle.inner.clone()),
    }))
}

/// Creates a log that carries on from the last entry an earlier one wrote.
///
/// This is what a device does after a restart: the chain continues at the next
/// index and hashes onto the entry it left off at, so a reboot leaves no gap for
/// a record to be removed through.
///
/// # Arguments
///
/// * `identity` - the identity whose signature each entry will carry.
/// * `last` - the final entry of the existing log.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_audit_log_free`], or null on
/// failure.
///
/// # Safety
///
/// `identity` must be a live identity handle, and `last` a live handle from a
/// call that produced one, or either may be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_log_resume(
    identity: *const PamojaDeviceIdentity,
    last: *const PamojaAuditEntry,
) -> *mut PamojaAuditLog {
    let Some(handle) = identity_handle(identity) else {
        return ptr::null_mut();
    };
    let Some(last) = entry_handle(last) else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaAuditLog {
        log: AuditLog::resume(handle.inner.clone(), &last.entry),
    }))
}

/// Appends a payload to a log, signing it and chaining it onto the last entry.
///
/// # Arguments
///
/// * `log` - the log to append to.
/// * `payload` - the record to store.
/// * `payload_len` - the length of `payload` in bytes.
///
/// # Returns
///
/// A handle to the new entry, which the caller must release with
/// [`pamoja_audit_entry_free`], or null on failure.
///
/// # Safety
///
/// `log` must be a live handle from [`pamoja_audit_log_new`] or
/// [`pamoja_audit_log_resume`], and `payload` must point to at least
/// `payload_len` readable bytes, or be null when `payload_len` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_log_append(
    log: *mut PamojaAuditLog,
    payload: *const u8,
    payload_len: usize,
) -> *mut PamojaAuditEntry {
    if log.is_null() {
        set_last_error("log must not be null".to_owned());
        return ptr::null_mut();
    }
    let Ok(payload) = read_bytes(payload, payload_len) else {
        return ptr::null_mut();
    };
    let result = catch_unwind(AssertUnwindSafe(|| (*log).log.append(&payload)));
    match result {
        Ok(entry) => Box::into_raw(Box::new(PamojaAuditEntry { entry })),
        Err(_) => {
            set_last_error("append panicked".to_owned());
            ptr::null_mut()
        }
    }
}

/// Releases a log handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `log` must be a handle from a call that produced one and that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_log_free(log: *mut PamojaAuditLog) {
    if !log.is_null() {
        drop(Box::from_raw(log));
    }
}

/// Reads an entry back from the bytes it was written as.
///
/// # Arguments
///
/// * `bytes` - the encoded entry.
/// * `len` - the length of `bytes`.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_audit_entry_free`], or null if
/// the bytes are not a well-formed entry.
///
/// # Safety
///
/// `bytes` must point to at least `len` readable bytes, or be null when `len` is
/// 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_entry_from_bytes(
    bytes: *const u8,
    len: usize,
) -> *mut PamojaAuditEntry {
    let Ok(bytes) = read_bytes(bytes, len) else {
        return ptr::null_mut();
    };
    match Entry::from_bytes(&bytes) {
        Ok(entry) => Box::into_raw(Box::new(PamojaAuditEntry { entry })),
        Err(error) => {
            set_last_error(error.to_string());
            ptr::null_mut()
        }
    }
}

/// Encodes an entry for storage or transmission.
///
/// # Arguments
///
/// * `entry` - the entry to encode.
///
/// # Returns
///
/// A buffer the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or null if `entry` is null.
///
/// # Safety
///
/// `entry` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_entry_to_bytes(
    entry: *const PamojaAuditEntry,
) -> *mut PamojaBuffer {
    let Some(entry) = entry_handle(entry) else {
        return ptr::null_mut();
    };
    PamojaBuffer::into_raw(entry.entry.to_bytes())
}

/// Returns the position of an entry in its chain.
///
/// # Arguments
///
/// * `entry` - the entry.
///
/// # Returns
///
/// The zero-based index, or 0 if `entry` is null.
///
/// # Safety
///
/// `entry` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_entry_index(entry: *const PamojaAuditEntry) -> u64 {
    match entry_handle(entry) {
        Some(entry) => entry.entry.index(),
        None => 0,
    }
}

/// Copies out the hash of the entry before this one.
///
/// The first entry of a chain carries all zeroes here, since nothing precedes it.
///
/// # Arguments
///
/// * `entry` - the entry.
/// * `out_previous` - receives [`PAMOJA_AUDIT_DIGEST_LEN`] bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `entry` must be a live handle, and `out_previous` must point to at least
/// [`PAMOJA_AUDIT_DIGEST_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_entry_previous(
    entry: *const PamojaAuditEntry,
    out_previous: *mut u8,
) -> PamojaStatus {
    let Some(entry) = entry_handle(entry) else {
        return PamojaStatus::InvalidArgument;
    };
    write_digest(entry.entry.previous(), out_previous, "out_previous")
}

/// Copies out the hash of this entry, which the next one chains onto.
///
/// # Arguments
///
/// * `entry` - the entry.
/// * `out_digest` - receives [`PAMOJA_AUDIT_DIGEST_LEN`] bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `entry` must be a live handle, and `out_digest` must point to at least
/// [`PAMOJA_AUDIT_DIGEST_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_entry_digest(
    entry: *const PamojaAuditEntry,
    out_digest: *mut u8,
) -> PamojaStatus {
    let Some(entry) = entry_handle(entry) else {
        return PamojaStatus::InvalidArgument;
    };
    write_digest(entry.entry.digest(), out_digest, "out_digest")
}

/// Copies out the signature over an entry.
///
/// # Arguments
///
/// * `entry` - the entry.
/// * `out_signature` - receives [`PAMOJA_AUDIT_SIGNATURE_LEN`] bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `entry` must be a live handle, and `out_signature` must point to at least
/// [`PAMOJA_AUDIT_SIGNATURE_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_entry_signature(
    entry: *const PamojaAuditEntry,
    out_signature: *mut u8,
) -> PamojaStatus {
    let Some(entry) = entry_handle(entry) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_signature.is_null() {
        set_last_error("out_signature must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = entry.entry.signature().to_bytes();
    ptr::copy_nonoverlapping(bytes.as_ptr(), out_signature, PAMOJA_AUDIT_SIGNATURE_LEN);
    PamojaStatus::Ok
}

/// Copies out the record an entry carries.
///
/// # Arguments
///
/// * `entry` - the entry.
///
/// # Returns
///
/// A buffer the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or null if `entry` is null.
///
/// # Safety
///
/// `entry` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_entry_payload(
    entry: *const PamojaAuditEntry,
) -> *mut PamojaBuffer {
    let Some(entry) = entry_handle(entry) else {
        return ptr::null_mut();
    };
    PamojaBuffer::into_raw(entry.entry.payload().to_vec())
}

/// Releases an entry handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `entry` must be a handle from a call that produced one and that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_entry_free(entry: *mut PamojaAuditEntry) {
    if !entry.is_null() {
        drop(Box::from_raw(entry));
    }
}

/// Creates a verifier that checks a chain signed by one public key.
///
/// # Arguments
///
/// * `public_key` - the `PAMOJA_KEY_LEN`-byte key the entries were signed with.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_audit_verifier_free`], or null
/// if the key is not a valid public key.
///
/// # Safety
///
/// `public_key` must point to at least `PAMOJA_KEY_LEN` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_verifier_new(
    public_key: *const u8,
) -> *mut PamojaAuditVerifier {
    let Ok(public) = read_public(public_key) else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaAuditVerifier {
        verifier: Verifier::new(public),
    }))
}

/// Checks the next entry of a chain, in the order the entries were written.
///
/// A verifier only accepts an entry that follows the one before it, so feeding
/// entries out of order, skipping one, or repeating one is refused just as an
/// altered payload is.
///
/// # Arguments
///
/// * `verifier` - the verifier.
/// * `entry` - the next entry to check.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] if the entry belongs where it was offered, or
/// [`PamojaStatus::Auth`] if the chain, the index, or the signature does not hold.
///
/// # Safety
///
/// `verifier` must be a live handle from [`pamoja_audit_verifier_new`], and
/// `entry` a live entry handle, or either may be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_verifier_check(
    verifier: *mut PamojaAuditVerifier,
    entry: *const PamojaAuditEntry,
) -> PamojaStatus {
    if verifier.is_null() {
        set_last_error("verifier must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(entry) = entry_handle(entry) else {
        return PamojaStatus::InvalidArgument;
    };
    match (*verifier).verifier.check(&entry.entry) {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => {
            let status = PamojaStatus::from_error(&error);
            set_last_error(error.to_string());
            status
        }
    }
}

/// Releases a verifier handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `verifier` must be a handle from [`pamoja_audit_verifier_new`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_verifier_free(verifier: *mut PamojaAuditVerifier) {
    if !verifier.is_null() {
        drop(Box::from_raw(verifier));
    }
}

/// Checks a whole chain that has already arrived.
///
/// # Arguments
///
/// * `public_key` - the `PAMOJA_KEY_LEN`-byte key the entries were signed with.
/// * `entries` - an array of entry handles, in the order they were written.
/// * `count` - how many handles `entries` holds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] if every entry follows the one before it and carries a
/// signature that holds, or [`PamojaStatus::Auth`] if any does not.
///
/// # Safety
///
/// `public_key` must point to at least `PAMOJA_KEY_LEN` readable bytes, and
/// `entries` must point to at least `count` live entry handles, none of them
/// null, or be null when `count` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_audit_verify_chain(
    public_key: *const u8,
    entries: *const *const PamojaAuditEntry,
    count: usize,
) -> PamojaStatus {
    let public = match read_public(public_key) {
        Ok(public) => public,
        Err(status) => return status,
    };
    if count != 0 && entries.is_null() {
        set_last_error("entries must not be null when count is non-zero".to_owned());
        return PamojaStatus::InvalidArgument;
    }

    let mut owned = Vec::with_capacity(count);
    for offset in 0..count {
        let Some(entry) = entry_handle(*entries.add(offset)) else {
            return PamojaStatus::InvalidArgument;
        };
        owned.push(entry.entry.clone());
    }

    match verify_chain(&public, &owned) {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => {
            let status = PamojaStatus::from_error(&error);
            set_last_error(error.to_string());
            status
        }
    }
}

/// Borrows an entry handle, rejecting a null pointer.
///
/// # Safety
///
/// `entry` must be a live handle from a call that produced one, or null.
unsafe fn entry_handle<'a>(entry: *const PamojaAuditEntry) -> Option<&'a PamojaAuditEntry> {
    if entry.is_null() {
        set_last_error("entry must not be null".to_owned());
        return None;
    }
    Some(&*entry)
}

/// Copies a 32-byte hash into a caller buffer, rejecting a null destination.
///
/// # Safety
///
/// `out` must point to at least [`PAMOJA_AUDIT_DIGEST_LEN`] writable bytes, or
/// be null.
unsafe fn write_digest(
    digest: [u8; PAMOJA_AUDIT_DIGEST_LEN],
    out: *mut u8,
    name: &str,
) -> PamojaStatus {
    if out.is_null() {
        set_last_error(format!("{name} must not be null"));
        return PamojaStatus::InvalidArgument;
    }
    ptr::copy_nonoverlapping(digest.as_ptr(), out, PAMOJA_AUDIT_DIGEST_LEN);
    PamojaStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{
        pamoja_device_identity_free, pamoja_device_identity_new, PAMOJA_KEY_LEN,
    };
    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};

    /// Builds an identity and its public key from a repeated-byte seed.
    unsafe fn signer(seed: u8) -> (*mut PamojaDeviceIdentity, [u8; PAMOJA_KEY_LEN]) {
        let seed = [seed; PAMOJA_KEY_LEN];
        let identity = pamoja_device_identity_new(seed.as_ptr(), seed.len());
        assert!(!identity.is_null());
        let mut public = [0u8; PAMOJA_KEY_LEN];
        assert_eq!(
            crate::security::pamoja_device_identity_public_key(identity, public.as_mut_ptr()),
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

    #[test]
    fn a_chain_verifies_entry_by_entry() {
        unsafe {
            let (identity, public) = signer(7);
            let log = pamoja_audit_log_new(identity);
            let verifier = pamoja_audit_verifier_new(public.as_ptr());

            for index in 0..3u64 {
                let payload = [index as u8; 4];
                let entry = pamoja_audit_log_append(log, payload.as_ptr(), payload.len());
                assert_eq!(pamoja_audit_entry_index(entry), index);
                assert_eq!(
                    pamoja_audit_verifier_check(verifier, entry),
                    PamojaStatus::Ok
                );
                assert_eq!(take(pamoja_audit_entry_payload(entry)), payload);
                pamoja_audit_entry_free(entry);
            }

            pamoja_audit_verifier_free(verifier);
            pamoja_audit_log_free(log);
            pamoja_device_identity_free(identity);
        }
    }

    #[test]
    fn an_altered_record_breaks_the_chain() {
        unsafe {
            let (identity, public) = signer(9);
            let log = pamoja_audit_log_new(identity);

            let first = pamoja_audit_log_append(log, b"open".as_ptr(), 4);
            let second = pamoja_audit_log_append(log, b"shut".as_ptr(), 4);

            let mut bytes = take(pamoja_audit_entry_to_bytes(second));
            let last = bytes.len() - 1;
            bytes[last] ^= 0xff;
            let tampered = pamoja_audit_entry_from_bytes(bytes.as_ptr(), bytes.len());
            assert!(!tampered.is_null());

            let chain = [first.cast_const(), tampered.cast_const()];
            assert_eq!(
                pamoja_audit_verify_chain(public.as_ptr(), chain.as_ptr(), chain.len()),
                PamojaStatus::Auth
            );

            let honest = [first.cast_const(), second.cast_const()];
            assert_eq!(
                pamoja_audit_verify_chain(public.as_ptr(), honest.as_ptr(), honest.len()),
                PamojaStatus::Ok
            );

            pamoja_audit_entry_free(tampered);
            pamoja_audit_entry_free(second);
            pamoja_audit_entry_free(first);
            pamoja_audit_log_free(log);
            pamoja_device_identity_free(identity);
        }
    }

    #[test]
    fn a_resumed_log_continues_the_chain() {
        unsafe {
            let (identity, public) = signer(11);
            let first_log = pamoja_audit_log_new(identity);
            let first = pamoja_audit_log_append(first_log, b"boot".as_ptr(), 4);

            let resumed = pamoja_audit_log_resume(identity, first);
            let second = pamoja_audit_log_append(resumed, b"read".as_ptr(), 4);
            assert_eq!(pamoja_audit_entry_index(second), 1);

            let mut previous = [0u8; PAMOJA_AUDIT_DIGEST_LEN];
            let mut digest = [0u8; PAMOJA_AUDIT_DIGEST_LEN];
            assert_eq!(
                pamoja_audit_entry_previous(second, previous.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_audit_entry_digest(first, digest.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(previous, digest);

            let chain = [first.cast_const(), second.cast_const()];
            assert_eq!(
                pamoja_audit_verify_chain(public.as_ptr(), chain.as_ptr(), chain.len()),
                PamojaStatus::Ok
            );

            pamoja_audit_entry_free(second);
            pamoja_audit_entry_free(first);
            pamoja_audit_log_free(resumed);
            pamoja_audit_log_free(first_log);
            pamoja_device_identity_free(identity);
        }
    }

    #[test]
    fn a_null_handle_is_refused_rather_than_dereferenced() {
        unsafe {
            assert!(pamoja_audit_log_new(ptr::null()).is_null());
            assert!(pamoja_audit_entry_to_bytes(ptr::null()).is_null());
            assert_eq!(pamoja_audit_entry_index(ptr::null()), 0);
            assert_eq!(
                pamoja_audit_verifier_check(ptr::null_mut(), ptr::null()),
                PamojaStatus::InvalidArgument
            );
            pamoja_audit_entry_free(ptr::null_mut());
            pamoja_audit_log_free(ptr::null_mut());
            pamoja_audit_verifier_free(ptr::null_mut());
        }
    }
}
