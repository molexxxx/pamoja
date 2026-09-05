//! The C ABI for encrypted, authenticated sessions.
//!
//! These functions wrap [`pamoja_session`] for callers that reach the SDK through
//! the flat C boundary: the key agreement two devices use to arrive at the same
//! session key without sending it, and the sealed messages that key then protects.
//!
//! A session holds the counter it sends under and the window of what it has
//! accepted, so it crosses as an opaque handle; so does an agreement key, because
//! it holds a secret that should not be copied around by value. Messages are
//! encrypted in place, so the caller supplies the buffer and nothing allocates.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::slice;

use pamoja_session::{
    hkdf_sha256, hmac_sha256, AgreementKey, AgreementPublicKey, Role, Sealed, Session,
};

use crate::{read_bytes, set_last_error, PamojaStatus};

/// The length in bytes of an agreement seed, a public key, and a digest.
pub const PAMOJA_SESSION_KEY_LEN: usize = 32;

/// The length in bytes of the tag that authenticates a sealed message.
pub const PAMOJA_SESSION_TAG_LEN: usize = 16;

/// Which side of a session a device is on.
///
/// The two devices must choose opposite roles. The role decides the order the
/// public keys are mixed in and which direction each side tags its messages with,
/// so a session where both sides claim the same role will not open anything.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaSessionRole {
    /// The device that opens the session.
    Initiator = 0,
    /// The device that answers.
    Responder = 1,
}

/// The header that travels beside a sealed message.
///
/// The peer needs the counter to rebuild the nonce and to reject a replay, and
/// the tag to tell whether the message arrived as it was sent.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSealed {
    /// The counter naming this message within the session.
    pub counter: u64,
    /// The tag over the ciphertext and its associated data.
    pub tag: [u8; PAMOJA_SESSION_TAG_LEN],
}

/// An opaque handle to a key-agreement secret.
///
/// Create it with [`pamoja_agreement_key_from_seed`] and release it with
/// [`pamoja_agreement_key_free`].
pub struct PamojaAgreementKey {
    key: AgreementKey,
}

/// An opaque handle to a live session with one peer.
///
/// Create it with [`pamoja_session_establish`] and release it with
/// [`pamoja_session_free`].
pub struct PamojaSession {
    session: Session,
}

/// Creates a key-agreement secret from a provisioned 32-byte seed.
///
/// # Arguments
///
/// * `seed` - the [`PAMOJA_SESSION_KEY_LEN`] secret bytes.
/// * `seed_len` - the length of `seed`, which must be
///   [`PAMOJA_SESSION_KEY_LEN`].
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_agreement_key_free`], or null
/// on failure with the reason available from
/// [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `seed` must point to at least `seed_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_agreement_key_from_seed(
    seed: *const u8,
    seed_len: usize,
) -> *mut PamojaAgreementKey {
    let Ok(bytes) = read_bytes(seed, seed_len) else {
        return ptr::null_mut();
    };
    let Ok(seed) = <[u8; PAMOJA_SESSION_KEY_LEN]>::try_from(bytes.as_slice()) else {
        set_last_error(format!(
            "seed must be exactly {PAMOJA_SESSION_KEY_LEN} bytes"
        ));
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaAgreementKey {
        key: AgreementKey::from_seed(&seed),
    }))
}

/// Copies out the public key to hand to a peer.
///
/// # Arguments
///
/// * `key` - the agreement key.
/// * `out_public_key` - receives [`PAMOJA_SESSION_KEY_LEN`] bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `key` must be a live handle from [`pamoja_agreement_key_from_seed`], and
/// `out_public_key` must point to at least [`PAMOJA_SESSION_KEY_LEN`] writable
/// bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_agreement_key_public(
    key: *const PamojaAgreementKey,
    out_public_key: *mut u8,
) -> PamojaStatus {
    if key.is_null() {
        set_last_error("key must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    if out_public_key.is_null() {
        set_last_error("out_public_key must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let public = (*key).key.public().to_bytes();
    ptr::copy_nonoverlapping(public.as_ptr(), out_public_key, PAMOJA_SESSION_KEY_LEN);
    PamojaStatus::Ok
}

/// Releases an agreement key handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `key` must be a handle from [`pamoja_agreement_key_from_seed`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_agreement_key_free(key: *mut PamojaAgreementKey) {
    if !key.is_null() {
        drop(Box::from_raw(key));
    }
}

/// Establishes a session with a peer.
///
/// Both devices call this with the same salt and opposite roles, and arrive at
/// the same key without either sending it. The salt is a fresh per-session value
/// exchanged in the clear; reusing one with the same pair of keys reuses the
/// session key, so it must change each session.
///
/// # Arguments
///
/// * `local` - this device key-agreement secret.
/// * `peer_public_key` - the [`PAMOJA_SESSION_KEY_LEN`]-byte public key of the
///   peer, already authenticated by pinning or by a signature.
/// * `salt` - the fresh per-session salt both sides share.
/// * `salt_len` - the length of `salt`.
/// * `role` - whether this device opens the session or answers.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_session_free`], or null on
/// failure.
///
/// # Safety
///
/// `local` must be a live agreement key handle, `peer_public_key` must point to
/// at least [`PAMOJA_SESSION_KEY_LEN`] readable bytes, and `salt` must point to
/// at least `salt_len` readable bytes, or be null when `salt_len` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_session_establish(
    local: *const PamojaAgreementKey,
    peer_public_key: *const u8,
    salt: *const u8,
    salt_len: usize,
    role: PamojaSessionRole,
) -> *mut PamojaSession {
    if local.is_null() {
        set_last_error("local must not be null".to_owned());
        return ptr::null_mut();
    }
    let Ok(peer_bytes) = read_bytes(peer_public_key, PAMOJA_SESSION_KEY_LEN) else {
        return ptr::null_mut();
    };
    let Ok(peer_bytes) = <[u8; PAMOJA_SESSION_KEY_LEN]>::try_from(peer_bytes.as_slice()) else {
        set_last_error(format!(
            "peer public key must be exactly {PAMOJA_SESSION_KEY_LEN} bytes"
        ));
        return ptr::null_mut();
    };
    let Ok(salt) = read_bytes(salt, salt_len) else {
        return ptr::null_mut();
    };

    let peer = AgreementPublicKey::from_bytes(&peer_bytes);
    let established = catch_unwind(AssertUnwindSafe(|| {
        Session::establish(&(*local).key, &peer, &salt, rust_role(role))
    }));
    match established {
        Ok(session) => Box::into_raw(Box::new(PamojaSession { session })),
        Err(_) => {
            set_last_error("establishing the session panicked".to_owned());
            ptr::null_mut()
        }
    }
}

/// Seals a message for the peer, encrypting it in place.
///
/// The associated data is authenticated but not encrypted, so it stays readable
/// on the wire yet cannot be altered: a device identifier or a routing header
/// belongs there. On success `buf` holds the ciphertext and `out_sealed` holds
/// the counter and tag to send with it.
///
/// # Arguments
///
/// * `session` - the session.
/// * `buf` - the plaintext, replaced by the ciphertext of equal length.
/// * `len` - the length of `buf`.
/// * `aad` - associated data to authenticate alongside the message.
/// * `aad_len` - the length of `aad`.
/// * `out_sealed` - receives the counter and tag.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `session` must be a live handle from [`pamoja_session_establish`], `buf` must
/// point to at least `len` readable and writable bytes or be null when `len` is
/// 0, `aad` must point to at least `aad_len` readable bytes or be null when
/// `aad_len` is 0, and `out_sealed` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_session_seal(
    session: *mut PamojaSession,
    buf: *mut u8,
    len: usize,
    aad: *const u8,
    aad_len: usize,
    out_sealed: *mut PamojaSealed,
) -> PamojaStatus {
    if session.is_null() {
        set_last_error("session must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    if out_sealed.is_null() {
        set_last_error("out_sealed must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    if len != 0 && buf.is_null() {
        set_last_error("buf must not be null when its length is non-zero".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let aad = match read_bytes(aad, aad_len) {
        Ok(aad) => aad,
        Err(status) => return status,
    };

    let message = if len == 0 {
        &mut [][..]
    } else {
        slice::from_raw_parts_mut(buf, len)
    };
    let sealed = (*session).session.seal(message, &aad);
    *out_sealed = PamojaSealed {
        counter: sealed.counter,
        tag: sealed.tag,
    };
    PamojaStatus::Ok
}

/// Opens a message from the peer, verifying it and decrypting it in place.
///
/// A message is rejected if its counter repeats or is older than the replay
/// window still tracks, and if its tag does not authenticate. On any rejection
/// `buf` is left zeroed, so a failed open never yields readable bytes.
///
/// # Arguments
///
/// * `session` - the session.
/// * `sealed` - the counter and tag that arrived with the ciphertext.
/// * `buf` - the ciphertext, replaced by the plaintext on success.
/// * `len` - the length of `buf`.
/// * `aad` - the same associated data the sender authenticated.
/// * `aad_len` - the length of `aad`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] if the message is authentic and fresh, or
/// [`PamojaStatus::Auth`] if it is not, with the message from
/// [`pamoja_last_error_message`](crate::pamoja_last_error_message) saying whether
/// it failed authentication or repeated a counter.
///
/// # Safety
///
/// `session` must be a live handle from [`pamoja_session_establish`], `buf` must
/// point to at least `len` readable and writable bytes or be null when `len` is
/// 0, and `aad` must point to at least `aad_len` readable bytes or be null when
/// `aad_len` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_session_open(
    session: *mut PamojaSession,
    sealed: PamojaSealed,
    buf: *mut u8,
    len: usize,
    aad: *const u8,
    aad_len: usize,
) -> PamojaStatus {
    if session.is_null() {
        set_last_error("session must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    if len != 0 && buf.is_null() {
        set_last_error("buf must not be null when its length is non-zero".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let aad = match read_bytes(aad, aad_len) {
        Ok(aad) => aad,
        Err(status) => return status,
    };

    let message = if len == 0 {
        &mut [][..]
    } else {
        slice::from_raw_parts_mut(buf, len)
    };
    let header = Sealed {
        counter: sealed.counter,
        tag: sealed.tag,
    };
    match (*session).session.open(&header, message, &aad) {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Auth
        }
    }
}

/// Releases a session handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `session` must be a handle from [`pamoja_session_establish`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_session_free(session: *mut PamojaSession) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

/// Computes a keyed hash over a message.
///
/// This is the primitive a host uses to authenticate a pairing exchange or a
/// single command, where a whole session would be more than the job needs.
///
/// # Arguments
///
/// * `key` - the secret key.
/// * `key_len` - the length of `key`.
/// * `message` - the message to authenticate.
/// * `message_len` - the length of `message`.
/// * `out_digest` - receives [`PAMOJA_SESSION_KEY_LEN`] bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `key` and `message` must point to at least their stated lengths of readable
/// bytes, or be null when those lengths are 0, and `out_digest` must point to at
/// least [`PAMOJA_SESSION_KEY_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_session_hmac_sha256(
    key: *const u8,
    key_len: usize,
    message: *const u8,
    message_len: usize,
    out_digest: *mut u8,
) -> PamojaStatus {
    let key = match read_bytes(key, key_len) {
        Ok(key) => key,
        Err(status) => return status,
    };
    let message = match read_bytes(message, message_len) {
        Ok(message) => message,
        Err(status) => return status,
    };
    if out_digest.is_null() {
        set_last_error("out_digest must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let digest = hmac_sha256(&key, &message);
    ptr::copy_nonoverlapping(digest.as_ptr(), out_digest, PAMOJA_SESSION_KEY_LEN);
    PamojaStatus::Ok
}

/// Expands input keying material into as many bytes as are asked for.
///
/// # Arguments
///
/// * `salt` - the salt, which may be empty.
/// * `salt_len` - the length of `salt`.
/// * `ikm` - the input keying material.
/// * `ikm_len` - the length of `ikm`.
/// * `info` - context binding the output to its purpose, which may be empty.
/// * `info_len` - the length of `info`.
/// * `out` - receives `out_len` derived bytes.
/// * `out_len` - how many bytes to derive.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `salt`, `ikm`, and `info` must each point to at least their stated lengths of
/// readable bytes, or be null when those lengths are 0, and `out` must point to
/// at least `out_len` writable bytes, or be null when `out_len` is 0.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn pamoja_session_hkdf_sha256(
    salt: *const u8,
    salt_len: usize,
    ikm: *const u8,
    ikm_len: usize,
    info: *const u8,
    info_len: usize,
    out: *mut u8,
    out_len: usize,
) -> PamojaStatus {
    let salt = match read_bytes(salt, salt_len) {
        Ok(salt) => salt,
        Err(status) => return status,
    };
    let ikm = match read_bytes(ikm, ikm_len) {
        Ok(ikm) => ikm,
        Err(status) => return status,
    };
    let info = match read_bytes(info, info_len) {
        Ok(info) => info,
        Err(status) => return status,
    };
    if out_len == 0 {
        return PamojaStatus::Ok;
    }
    if out.is_null() {
        set_last_error("out must not be null when its length is non-zero".to_owned());
        return PamojaStatus::InvalidArgument;
    }

    let mut derived = vec![0u8; out_len];
    hkdf_sha256(&salt, &ikm, &info, &mut derived);
    ptr::copy_nonoverlapping(derived.as_ptr(), out, out_len);
    PamojaStatus::Ok
}

/// Maps a boundary role back onto the Rust one.
fn rust_role(role: PamojaSessionRole) -> Role {
    match role {
        PamojaSessionRole::Initiator => Role::Initiator,
        PamojaSessionRole::Responder => Role::Responder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Establishes the two ends of one session over a fixed salt.
    unsafe fn pair() -> (
        *mut PamojaAgreementKey,
        *mut PamojaAgreementKey,
        *mut PamojaSession,
        *mut PamojaSession,
    ) {
        let device_seed = [1u8; PAMOJA_SESSION_KEY_LEN];
        let gateway_seed = [2u8; PAMOJA_SESSION_KEY_LEN];
        let device_key = pamoja_agreement_key_from_seed(device_seed.as_ptr(), device_seed.len());
        let gateway_key = pamoja_agreement_key_from_seed(gateway_seed.as_ptr(), gateway_seed.len());

        let mut device_public = [0u8; PAMOJA_SESSION_KEY_LEN];
        let mut gateway_public = [0u8; PAMOJA_SESSION_KEY_LEN];
        assert_eq!(
            pamoja_agreement_key_public(device_key, device_public.as_mut_ptr()),
            PamojaStatus::Ok
        );
        assert_eq!(
            pamoja_agreement_key_public(gateway_key, gateway_public.as_mut_ptr()),
            PamojaStatus::Ok
        );

        let salt = [9u8; 16];
        let device = pamoja_session_establish(
            device_key,
            gateway_public.as_ptr(),
            salt.as_ptr(),
            salt.len(),
            PamojaSessionRole::Initiator,
        );
        let gateway = pamoja_session_establish(
            gateway_key,
            device_public.as_ptr(),
            salt.as_ptr(),
            salt.len(),
            PamojaSessionRole::Responder,
        );
        assert!(!device.is_null() && !gateway.is_null());
        (device_key, gateway_key, device, gateway)
    }

    #[test]
    fn a_sealed_message_opens_at_the_peer() {
        unsafe {
            let (device_key, gateway_key, device, gateway) = pair();

            let mut message = *b"4.8C";
            let mut sealed = PamojaSealed {
                counter: 0,
                tag: [0; PAMOJA_SESSION_TAG_LEN],
            };
            assert_eq!(
                pamoja_session_seal(
                    device,
                    message.as_mut_ptr(),
                    message.len(),
                    b"fridge-1".as_ptr(),
                    8,
                    &mut sealed,
                ),
                PamojaStatus::Ok
            );
            assert_ne!(&message, b"4.8C");

            assert_eq!(
                pamoja_session_open(
                    gateway,
                    sealed,
                    message.as_mut_ptr(),
                    message.len(),
                    b"fridge-1".as_ptr(),
                    8,
                ),
                PamojaStatus::Ok
            );
            assert_eq!(&message, b"4.8C");

            pamoja_session_free(gateway);
            pamoja_session_free(device);
            pamoja_agreement_key_free(gateway_key);
            pamoja_agreement_key_free(device_key);
        }
    }

    #[test]
    fn a_repeated_counter_is_refused() {
        unsafe {
            let (device_key, gateway_key, device, gateway) = pair();

            let mut message = *b"on";
            let mut sealed = PamojaSealed {
                counter: 0,
                tag: [0; PAMOJA_SESSION_TAG_LEN],
            };
            pamoja_session_seal(
                device,
                message.as_mut_ptr(),
                message.len(),
                ptr::null(),
                0,
                &mut sealed,
            );
            let ciphertext = message;

            assert_eq!(
                pamoja_session_open(
                    gateway,
                    sealed,
                    message.as_mut_ptr(),
                    message.len(),
                    ptr::null(),
                    0
                ),
                PamojaStatus::Ok
            );

            message = ciphertext;
            assert_eq!(
                pamoja_session_open(
                    gateway,
                    sealed,
                    message.as_mut_ptr(),
                    message.len(),
                    ptr::null(),
                    0
                ),
                PamojaStatus::Auth
            );

            pamoja_session_free(gateway);
            pamoja_session_free(device);
            pamoja_agreement_key_free(gateway_key);
            pamoja_agreement_key_free(device_key);
        }
    }

    #[test]
    fn altered_associated_data_fails_authentication() {
        unsafe {
            let (device_key, gateway_key, device, gateway) = pair();

            let mut message = *b"open";
            let mut sealed = PamojaSealed {
                counter: 0,
                tag: [0; PAMOJA_SESSION_TAG_LEN],
            };
            pamoja_session_seal(
                device,
                message.as_mut_ptr(),
                message.len(),
                b"door-1".as_ptr(),
                6,
                &mut sealed,
            );

            assert_eq!(
                pamoja_session_open(
                    gateway,
                    sealed,
                    message.as_mut_ptr(),
                    message.len(),
                    b"door-2".as_ptr(),
                    6,
                ),
                PamojaStatus::Auth
            );

            pamoja_session_free(gateway);
            pamoja_session_free(device);
            pamoja_agreement_key_free(gateway_key);
            pamoja_agreement_key_free(device_key);
        }
    }

    #[test]
    fn the_keyed_hash_matches_the_crate() {
        unsafe {
            let mut digest = [0u8; PAMOJA_SESSION_KEY_LEN];
            assert_eq!(
                pamoja_session_hmac_sha256(
                    b"key".as_ptr(),
                    3,
                    b"message".as_ptr(),
                    7,
                    digest.as_mut_ptr()
                ),
                PamojaStatus::Ok
            );
            assert_eq!(digest, hmac_sha256(b"key", b"message"));
        }
    }

    #[test]
    fn expansion_matches_the_crate() {
        unsafe {
            let mut derived = [0u8; 40];
            assert_eq!(
                pamoja_session_hkdf_sha256(
                    b"salt".as_ptr(),
                    4,
                    b"secret".as_ptr(),
                    6,
                    b"pairing".as_ptr(),
                    7,
                    derived.as_mut_ptr(),
                    derived.len(),
                ),
                PamojaStatus::Ok
            );

            let mut want = [0u8; 40];
            hkdf_sha256(b"salt", b"secret", b"pairing", &mut want);
            assert_eq!(derived, want);
        }
    }

    #[test]
    fn a_null_handle_is_refused_rather_than_dereferenced() {
        unsafe {
            assert!(pamoja_agreement_key_from_seed(ptr::null(), 0).is_null());
            assert_eq!(
                pamoja_agreement_key_public(ptr::null(), ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert!(pamoja_session_establish(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                PamojaSessionRole::Initiator
            )
            .is_null());
            pamoja_agreement_key_free(ptr::null_mut());
            pamoja_session_free(ptr::null_mut());
        }
    }
}
