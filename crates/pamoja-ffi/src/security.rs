//! The C ABI for device identity and signed telemetry.
//!
//! These functions wrap [`pamoja_security`] for callers that reach the SDK
//! through the flat C boundary. Signing and verifying are deterministic and need
//! no runtime, so unlike the transport capabilities nothing here blocks on an
//! executor.
//!
//! Every value this capability exchanges has a fixed width - a 32-byte seed, a
//! 32-byte public key, a 64-byte signature, a 16-character fingerprint - so the
//! caller supplies the output array and no allocation crosses the boundary. Only
//! the private identity is a handle, because it holds a secret that should not be
//! copied around by value.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use pamoja_security::{DeviceIdentity, PublicIdentity, Signature};

use crate::{read_bytes, set_last_error, PamojaStatus};

/// The length in bytes of an identity seed and of a public key.
pub const PAMOJA_KEY_LEN: usize = 32;

/// The length in bytes of a signature.
pub const PAMOJA_SIGNATURE_LEN: usize = 64;

/// The length in characters of a hex fingerprint.
pub const PAMOJA_FINGERPRINT_LEN: usize = 16;

/// An opaque handle to a device's private signing identity.
pub struct PamojaDeviceIdentity {
    pub(crate) inner: DeviceIdentity,
}

/// Creates a device identity from a provisioned 32-byte secret seed.
///
/// # Returns
///
/// A heap-allocated identity handle the caller owns and must release with
/// [`pamoja_device_identity_free`], or null on failure with the reason available
/// from [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `seed` must point to at least `seed_len` readable bytes, and `seed_len` must
/// be [`PAMOJA_KEY_LEN`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_device_identity_new(
    seed: *const u8,
    seed_len: usize,
) -> *mut PamojaDeviceIdentity {
    let bytes = match read_bytes(seed, seed_len) {
        Ok(bytes) => bytes,
        Err(_) => return ptr::null_mut(),
    };
    let Ok(seed) = <[u8; PAMOJA_KEY_LEN]>::try_from(bytes.as_slice()) else {
        set_last_error(format!("seed must be exactly {PAMOJA_KEY_LEN} bytes"));
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaDeviceIdentity {
        inner: DeviceIdentity::from_seed(&seed),
    }))
}

/// Writes the public key matching a device identity.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, having written [`PAMOJA_KEY_LEN`] bytes to
/// `out_public_key`.
///
/// # Safety
///
/// `identity` must be a live handle from [`pamoja_device_identity_new`], and
/// `out_public_key` must point to at least [`PAMOJA_KEY_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_device_identity_public_key(
    identity: *const PamojaDeviceIdentity,
    out_public_key: *mut u8,
) -> PamojaStatus {
    let Some(identity) = identity_handle(identity) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_public_key.is_null() {
        set_last_error("out_public_key must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let key = identity.inner.public().to_bytes();
    ptr::copy_nonoverlapping(key.as_ptr(), out_public_key, PAMOJA_KEY_LEN);
    PamojaStatus::Ok
}

/// Signs a payload with a device identity.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, having written [`PAMOJA_SIGNATURE_LEN`] bytes
/// to `out_signature`.
///
/// # Safety
///
/// `identity` must be a live handle from [`pamoja_device_identity_new`];
/// `payload` must point to at least `payload_len` readable bytes, or be null when
/// `payload_len` is 0; and `out_signature` must point to at least
/// [`PAMOJA_SIGNATURE_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_device_identity_sign(
    identity: *const PamojaDeviceIdentity,
    payload: *const u8,
    payload_len: usize,
    out_signature: *mut u8,
) -> PamojaStatus {
    let Some(identity) = identity_handle(identity) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_signature.is_null() {
        set_last_error("out_signature must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| {
        identity.inner.sign(&payload).to_bytes()
    })) {
        Ok(signature) => {
            ptr::copy_nonoverlapping(signature.as_ptr(), out_signature, PAMOJA_SIGNATURE_LEN);
            PamojaStatus::Ok
        }
        Err(_) => {
            set_last_error("panic at the FFI boundary".to_owned());
            PamojaStatus::Panic
        }
    }
}

/// Releases a device identity handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `identity` must be a handle from [`pamoja_device_identity_new`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_device_identity_free(identity: *mut PamojaDeviceIdentity) {
    if !identity.is_null() {
        drop(Box::from_raw(identity));
    }
}

/// Writes the short hex fingerprint of a public key.
///
/// The fingerprint is a convenient label for logs and displays, not a substitute
/// for the full key when checking trust.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, having written [`PAMOJA_FINGERPRINT_LEN`]
/// lowercase hex characters to `out_fingerprint`. No null terminator is written.
///
/// # Safety
///
/// `public_key` must point to at least [`PAMOJA_KEY_LEN`] readable bytes, and
/// `out_fingerprint` must point to at least [`PAMOJA_FINGERPRINT_LEN`] writable
/// bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_public_identity_fingerprint(
    public_key: *const u8,
    out_fingerprint: *mut u8,
) -> PamojaStatus {
    let public = match read_public(public_key) {
        Ok(public) => public,
        Err(status) => return status,
    };
    if out_fingerprint.is_null() {
        set_last_error("out_fingerprint must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let fingerprint = public.fingerprint();
    ptr::copy_nonoverlapping(
        fingerprint.as_ptr(),
        out_fingerprint,
        PAMOJA_FINGERPRINT_LEN,
    );
    PamojaStatus::Ok
}

/// Verifies that a signature covers a payload and was made by a public key.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] if the signature is authentic, or [`PamojaStatus::Auth`]
/// if it is not, which means the payload was altered or was signed by a different
/// device.
///
/// # Safety
///
/// `public_key` must point to at least [`PAMOJA_KEY_LEN`] readable bytes;
/// `payload` must point to at least `payload_len` readable bytes, or be null when
/// `payload_len` is 0; and `signature` must point to at least
/// [`PAMOJA_SIGNATURE_LEN`] readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_public_identity_verify(
    public_key: *const u8,
    payload: *const u8,
    payload_len: usize,
    signature: *const u8,
) -> PamojaStatus {
    let public = match read_public(public_key) {
        Ok(public) => public,
        Err(status) => return status,
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    let signature_bytes = match read_bytes(signature, PAMOJA_SIGNATURE_LEN) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(signature_bytes) = <[u8; PAMOJA_SIGNATURE_LEN]>::try_from(signature_bytes.as_slice())
    else {
        set_last_error(format!(
            "signature must be exactly {PAMOJA_SIGNATURE_LEN} bytes"
        ));
        return PamojaStatus::InvalidArgument;
    };
    let signature = Signature::from_bytes(&signature_bytes);

    match public.verify(&payload, &signature) {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::from_error(&error)
        }
    }
}

/// Borrows an identity handle, recording an error when it is null.
///
/// # Safety
///
/// `identity` must be a live handle from [`pamoja_device_identity_new`], or null.
pub(crate) unsafe fn identity_handle<'a>(
    identity: *const PamojaDeviceIdentity,
) -> Option<&'a PamojaDeviceIdentity> {
    if identity.is_null() {
        set_last_error("identity must not be null".to_owned());
        return None;
    }
    Some(&*identity)
}

/// Reads a 32-byte public key, rejecting a null pointer or an invalid key.
///
/// # Safety
///
/// `public_key` must point to at least [`PAMOJA_KEY_LEN`] readable bytes, or be
/// null.
pub(crate) unsafe fn read_public(public_key: *const u8) -> Result<PublicIdentity, PamojaStatus> {
    let bytes = read_bytes(public_key, PAMOJA_KEY_LEN)?;
    let Ok(bytes) = <[u8; PAMOJA_KEY_LEN]>::try_from(bytes.as_slice()) else {
        set_last_error(format!("public key must be exactly {PAMOJA_KEY_LEN} bytes"));
        return Err(PamojaStatus::InvalidArgument);
    };
    PublicIdentity::from_bytes(&bytes).map_err(|error| {
        let status = PamojaStatus::from_error(&error);
        set_last_error(error.to_string());
        status
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an identity from a repeated-byte seed for the tests below.
    fn identity(seed: u8) -> *mut PamojaDeviceIdentity {
        let seed = [seed; PAMOJA_KEY_LEN];
        // Safety: the seed is a valid 32-byte buffer for the call.
        let handle = unsafe { pamoja_device_identity_new(seed.as_ptr(), seed.len()) };
        assert!(!handle.is_null());
        handle
    }

    #[test]
    fn a_signature_verifies_against_its_signer() {
        let device = identity(1);
        let mut public = [0u8; PAMOJA_KEY_LEN];
        let mut signature = [0u8; PAMOJA_SIGNATURE_LEN];
        let payload = b"reading";

        // Safety: every buffer below is correctly sized and the handle is live.
        unsafe {
            assert_eq!(
                pamoja_device_identity_public_key(device, public.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_device_identity_sign(
                    device,
                    payload.as_ptr(),
                    payload.len(),
                    signature.as_mut_ptr()
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_public_identity_verify(
                    public.as_ptr(),
                    payload.as_ptr(),
                    payload.len(),
                    signature.as_ptr()
                ),
                PamojaStatus::Ok
            );
            pamoja_device_identity_free(device);
        }
    }

    #[test]
    fn a_tampered_payload_fails_with_an_auth_status() {
        let device = identity(2);
        let mut public = [0u8; PAMOJA_KEY_LEN];
        let mut signature = [0u8; PAMOJA_SIGNATURE_LEN];
        let payload = b"reading";

        // Safety: every buffer below is correctly sized and the handle is live.
        unsafe {
            pamoja_device_identity_public_key(device, public.as_mut_ptr());
            pamoja_device_identity_sign(
                device,
                payload.as_ptr(),
                payload.len(),
                signature.as_mut_ptr(),
            );
            let tampered = b"reading!";
            assert_eq!(
                pamoja_public_identity_verify(
                    public.as_ptr(),
                    tampered.as_ptr(),
                    tampered.len(),
                    signature.as_ptr()
                ),
                PamojaStatus::Auth
            );
            pamoja_device_identity_free(device);
        }
    }

    #[test]
    fn a_seed_of_the_wrong_length_is_rejected() {
        let seed = [7u8; 16];
        // Safety: the pointer and length agree; only the length is wrong for a seed.
        let handle = unsafe { pamoja_device_identity_new(seed.as_ptr(), seed.len()) };
        assert!(handle.is_null());
    }

    #[test]
    fn the_fingerprint_is_lowercase_hex() {
        let device = identity(3);
        let mut public = [0u8; PAMOJA_KEY_LEN];
        let mut fingerprint = [0u8; PAMOJA_FINGERPRINT_LEN];

        // Safety: every buffer below is correctly sized and the handle is live.
        unsafe {
            pamoja_device_identity_public_key(device, public.as_mut_ptr());
            assert_eq!(
                pamoja_public_identity_fingerprint(public.as_ptr(), fingerprint.as_mut_ptr()),
                PamojaStatus::Ok
            );
            pamoja_device_identity_free(device);
        }
        assert!(fingerprint
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)));
    }

    #[test]
    fn calls_on_a_null_identity_are_rejected() {
        let mut public = [0u8; PAMOJA_KEY_LEN];
        // Safety: every entry point tolerates a null handle without dereferencing it.
        unsafe {
            assert_eq!(
                pamoja_device_identity_public_key(ptr::null(), public.as_mut_ptr()),
                PamojaStatus::InvalidArgument
            );
            // Freeing null is a documented no-op.
            pamoja_device_identity_free(ptr::null_mut());
        }
    }
}
