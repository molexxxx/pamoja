//! Keyed hashing primitives: HMAC-SHA256 (RFC 2104, FIPS 198-1) and HKDF-SHA256
//! (RFC 5869).
//!
//! These are the building blocks the session key agreement uses, exposed so a host can
//! reuse the same audited, vector-pinned primitives instead of pulling in a second
//! crypto stack. A local-first dashboard, for example, derives a per-session key from a
//! pairing secret with [`hkdf_sha256`] and authenticates each command with
//! [`hmac_sha256`].

use hkdf::Hkdf;
use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Computes HMAC-SHA256 over a message with a key of any length.
///
/// # Arguments
///
/// * `key` - the secret key; any length is accepted, as HMAC defines.
/// * `message` - the bytes to authenticate.
///
/// # Returns
///
/// The 32-byte message authentication code.
///
/// # Examples
///
/// ```
/// // RFC 4231 test case 2.
/// let mac = pamoja_session::hmac_sha256(b"Jefe", b"what do ya want for nothing?");
/// assert_eq!(mac[..4], [0x5b, 0xdc, 0xc1, 0x46]);
/// ```
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts a key of any length");
    mac.update(message);
    mac.finalize().into_bytes().into()
}

/// Derives output key material from input keying material with HKDF-SHA256.
///
/// Extracts a pseudorandom key from `salt` and `ikm`, then expands it under `info` to
/// fill `out`.
///
/// # Arguments
///
/// * `salt` - a non-secret salt; a fresh per-session value gives each session its own key.
/// * `ikm` - the input keying material, such as a shared or pairing secret.
/// * `info` - a context label binding the output to its purpose.
/// * `out` - the buffer to fill with derived key material.
///
/// # Panics
///
/// Panics if `out` is longer than HKDF-SHA256's `255 * 32`-byte limit.
pub fn hkdf_sha256(salt: &[u8], ikm: &[u8], info: &[u8], out: &mut [u8]) {
    Hkdf::<Sha256>::new(Some(salt), ikm)
        .expand(info, out)
        .expect("output length is within HKDF-SHA256's 255 * 32-byte limit");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_the_rfc_4231_test_case_1() {
        // RFC 4231 section 4.2: 20 bytes of 0x0b keying "Hi There".
        let mac = hmac_sha256(&[0x0b; 20], b"Hi There");
        let expected: [u8; 32] = [
            0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b,
            0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7, 0x26, 0xe9, 0x37, 0x6c,
            0x2e, 0x32, 0xcf, 0xf7,
        ];
        assert_eq!(mac, expected);
    }

    #[test]
    fn hmac_matches_the_rfc_4231_test_case_2() {
        // RFC 4231 section 4.3: a short ASCII key with a longer message.
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        let expected: [u8; 32] = [
            0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24, 0x26, 0x08, 0x95,
            0x75, 0xc7, 0x5a, 0x00, 0x3f, 0x08, 0x9d, 0x27, 0x39, 0x83, 0x9d, 0xec, 0x58, 0xb9,
            0x64, 0xec, 0x38, 0x43,
        ];
        assert_eq!(mac, expected);
    }

    #[test]
    fn hkdf_matches_the_rfc_5869_basic_vector() {
        // RFC 5869 Appendix A.1: the SHA-256 basic test case.
        let ikm = [0x0bu8; 22];
        let salt: [u8; 13] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
        ];
        let info: [u8; 10] = [0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9];
        let expected: [u8; 42] = [
            0x3c, 0xb2, 0x5f, 0x25, 0xfa, 0xac, 0xd5, 0x7a, 0x90, 0x43, 0x4f, 0x64, 0xd0, 0x36,
            0x2f, 0x2a, 0x2d, 0x2d, 0x0a, 0x90, 0xcf, 0x1a, 0x5a, 0x4c, 0x5d, 0xb0, 0x2d, 0x56,
            0xec, 0xc4, 0xc5, 0xbf, 0x34, 0x00, 0x72, 0x08, 0xd5, 0xb8, 0x87, 0x18, 0x58, 0x65,
        ];
        let mut okm = [0u8; 42];
        hkdf_sha256(&salt, &ikm, &info, &mut okm);
        assert_eq!(okm, expected);
    }
}
