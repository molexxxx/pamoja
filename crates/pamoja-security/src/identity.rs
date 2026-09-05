//! Device identities: the private key that signs and the public key that verifies.

use alloc::string::String;

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};

use pamoja_core::{Error, Result};

use crate::Signature;

/// A device's private signing identity.
///
/// This is the secret half of a device's identity: the key it uses to sign its own
/// telemetry so a gateway or auditor can later prove the data came from this device
/// and was not tampered with. It is built from a 32-byte seed, which a device is
/// provisioned with and keeps in secure storage, so the same identity is recreated
/// deterministically across reboots without generating a new key each time.
///
/// Signing is deterministic and needs no randomness, so this works unchanged on a
/// microcontroller.
///
/// # Examples
///
/// ```
/// use pamoja_security::DeviceIdentity;
///
/// let device = DeviceIdentity::from_seed(&[7u8; 32]);
/// let signature = device.sign(b"fridge-1: 4.8C");
/// assert!(device.public().verify(b"fridge-1: 4.8C", &signature).is_ok());
/// ```
#[derive(Clone)]
pub struct DeviceIdentity {
    signing: SigningKey,
}

impl DeviceIdentity {
    /// Builds an identity from a 32-byte secret seed.
    ///
    /// # Arguments
    ///
    /// * `seed` - the 32 secret bytes the identity is derived from.
    ///
    /// # Returns
    ///
    /// The device identity.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        Self {
            signing: SigningKey::from_bytes(seed),
        }
    }

    /// Returns the public identity others use to verify this device's signatures.
    ///
    /// # Returns
    ///
    /// The matching [`PublicIdentity`].
    pub fn public(&self) -> PublicIdentity {
        PublicIdentity {
            verifying: self.signing.verifying_key(),
        }
    }

    /// Signs a payload with this device's key.
    ///
    /// # Arguments
    ///
    /// * `payload` - the bytes to sign, such as an encoded reading.
    ///
    /// # Returns
    ///
    /// A [`Signature`] over `payload`.
    pub fn sign(&self, payload: &[u8]) -> Signature {
        Signature(self.signing.sign(payload))
    }
}

/// A device's public identity: it names the device and verifies its signatures.
///
/// This is the public half of a device's identity, safe to share and distribute. A
/// gateway holds the public identities of the devices it trusts and uses them to
/// check that each signed payload is authentic and unaltered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicIdentity {
    verifying: VerifyingKey,
}

impl PublicIdentity {
    /// Reconstructs a public identity from its 32-byte form.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the 32-byte encoded public key.
    ///
    /// # Returns
    ///
    /// The public identity.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Auth`](pamoja_core::Error::Auth) if `bytes` is not a valid
    /// public key.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        VerifyingKey::from_bytes(bytes)
            .map(|verifying| Self { verifying })
            .map_err(|_| Error::Auth("invalid public identity".into()))
    }

    /// Returns the 32-byte wire form of this identity.
    ///
    /// # Returns
    ///
    /// The public key encoded as 32 bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.verifying.to_bytes()
    }

    /// Returns a short hex fingerprint of this identity for logs and displays.
    ///
    /// The fingerprint is the first eight bytes of the public key in hex. It is a
    /// convenient label, not a substitute for the full key when checking trust.
    ///
    /// # Returns
    ///
    /// A 16-character lowercase hex string.
    pub fn fingerprint(&self) -> String {
        let bytes = self.verifying.to_bytes();
        let mut hex = String::with_capacity(16);
        for &byte in &bytes[..8] {
            hex.push(nibble(byte >> 4));
            hex.push(nibble(byte & 0x0f));
        }
        hex
    }

    /// Verifies that `signature` covers `payload` and was made by this identity.
    ///
    /// # Arguments
    ///
    /// * `payload` - the bytes the signature is expected to cover.
    /// * `signature` - the signature to check.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the signature is authentic for `payload`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Auth`](pamoja_core::Error::Auth) if the signature does not
    /// match, which means the payload was altered or was not signed by this device.
    pub fn verify(&self, payload: &[u8], signature: &Signature) -> Result<()> {
        self.verifying
            .verify(payload, &signature.0)
            .map_err(|_| Error::Auth("signature verification failed".into()))
    }
}

// Maps a 0-15 value to its lowercase hex digit.
fn nibble(value: u8) -> char {
    char::from_digit(u32::from(value), 16).unwrap_or('0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_derivation_and_signing_match_the_rfc_8032_test_vector() {
        // RFC 8032 section 7.1, TEST 2: the 32-byte secret, the one-byte message 0x72,
        // and the signature the specification publishes for them. Anchoring to the
        // document rather than to a round trip is what catches an implementation that
        // is wrong but self-consistent.
        let device = DeviceIdentity::from_seed(&[
            0x4c, 0xcd, 0x08, 0x9b, 0x28, 0xff, 0x96, 0xda, 0x9d, 0xb6, 0xc3, 0x46, 0xec, 0x11,
            0x4e, 0x0f, 0x5b, 0x8a, 0x31, 0x9f, 0x35, 0xab, 0xa6, 0x24, 0xda, 0x8c, 0xf6, 0xed,
            0x4f, 0xb8, 0xa6, 0xfb,
        ]);
        assert_eq!(device.public().fingerprint(), "3d4017c3e843895a");
        assert_eq!(
            device.sign(&[0x72]).to_bytes(),
            [
                0x92, 0xa0, 0x09, 0xa9, 0xf0, 0xd4, 0xca, 0xb8, 0x72, 0x0e, 0x82, 0x0b, 0x5f, 0x64,
                0x25, 0x40, 0xa2, 0xb2, 0x7b, 0x54, 0x16, 0x50, 0x3f, 0x8f, 0xb3, 0x76, 0x22, 0x23,
                0xeb, 0xdb, 0x69, 0xda, 0x08, 0x5a, 0xc1, 0xe4, 0x3e, 0x15, 0x99, 0x6e, 0x45, 0x8f,
                0x36, 0x13, 0xd0, 0xf1, 0x1d, 0x8c, 0x38, 0x7b, 0x2e, 0xae, 0xb4, 0x30, 0x2a, 0xee,
                0xb0, 0x0d, 0x29, 0x16, 0x12, 0xbb, 0x0c, 0x00,
            ]
        );
    }

    #[test]
    fn a_signature_verifies_against_its_signer() {
        let device = DeviceIdentity::from_seed(&[1u8; 32]);
        let signature = device.sign(b"reading");
        assert!(device.public().verify(b"reading", &signature).is_ok());
    }

    #[test]
    fn a_tampered_payload_fails_verification() {
        let device = DeviceIdentity::from_seed(&[2u8; 32]);
        let signature = device.sign(b"4.8C");
        let result = device.public().verify(b"9.9C", &signature);
        assert!(matches!(result, Err(Error::Auth(_))));
    }

    #[test]
    fn another_device_cannot_verify_the_signature() {
        let device = DeviceIdentity::from_seed(&[3u8; 32]);
        let other = DeviceIdentity::from_seed(&[4u8; 32]);
        let signature = device.sign(b"reading");
        assert!(other.public().verify(b"reading", &signature).is_err());
    }

    #[test]
    fn a_public_identity_round_trips_through_bytes() {
        let public = DeviceIdentity::from_seed(&[5u8; 32]).public();
        let restored = PublicIdentity::from_bytes(&public.to_bytes()).expect("valid key");
        assert_eq!(public, restored);
    }

    #[test]
    fn a_signature_round_trips_through_bytes() {
        let device = DeviceIdentity::from_seed(&[6u8; 32]);
        let signature = device.sign(b"reading");
        let restored = Signature::from_bytes(&signature.to_bytes());
        assert!(device.public().verify(b"reading", &restored).is_ok());
    }

    #[test]
    fn the_fingerprint_is_sixteen_hex_characters() {
        let public = DeviceIdentity::from_seed(&[7u8; 32]).public();
        let fingerprint = public.fingerprint();
        assert_eq!(fingerprint.len(), 16);
        assert!(fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
