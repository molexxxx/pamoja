//! Who a device will take an update from, and how that can change.
//!
//! Trusting one key forever is a trap. The key has to be reachable to sign each
//! release, which is exactly what makes it likely to leak eventually, and a device
//! that trusts only that key is then permanently takeable. Lose it instead of
//! leaking it and the fleet becomes permanently unreachable, which is no better.
//!
//! So a device anchors its trust in a key that is used almost never and can live
//! in a safe, and that anchor signs a [`Delegation`] naming the key that actually
//! signs releases. Rotating the release key means issuing a new delegation, not
//! visiting the devices. This is the arrangement RFC 9124 calls a delegation
//! chain, at the depth that covers rotation: anchor, then release key.

use pamoja_security::{DeviceIdentity, PublicIdentity};

use crate::cbor::{Reader, Writer};
use crate::error::{Refusal, Result};
use crate::manifest::{expect_key, read_key, seal, Envelope};

/// A buffer of this size always holds an encoded delegation envelope.
pub const DELEGATION_MAX: usize = 192;

/// The length of a public key, in bytes.
const KEY_LEN: usize = 32;

// Map keys, ascending, as the deterministic encoding requires.
const KEY_EPOCH: u64 = 1;
const KEY_RELEASE: u64 = 2;
const KEY_EXPIRES: u64 = 3;

/// A statement, signed by a device's trust anchor, naming the key that may sign
/// its updates.
///
/// # Examples
///
/// ```
/// use pamoja_security::DeviceIdentity;
/// use pamoja_update::{Delegation, DELEGATION_MAX};
///
/// let anchor = DeviceIdentity::from_seed(&[1u8; 32]);
/// let release = DeviceIdentity::from_seed(&[2u8; 32]);
///
/// let delegation = Delegation {
///     epoch: 1,
///     release_key: release.public().to_bytes(),
///     expires: 0,
/// };
///
/// let mut buf = [0u8; DELEGATION_MAX];
/// let written = delegation.sign(&anchor, &mut buf).unwrap();
///
/// let adopted = Delegation::open(&buf[..written], &anchor.public()).unwrap();
/// assert_eq!(adopted, delegation);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delegation {
    /// Rises with every rotation. A device refuses a delegation not above the one
    /// it holds, so a retired key cannot be reinstated by replaying the statement
    /// that once authorised it.
    pub epoch: u64,
    /// The public key that may sign manifests while this delegation stands.
    pub release_key: [u8; KEY_LEN],
    /// When this delegation stops being honoured, in seconds since the Unix epoch,
    /// or `0` to never expire. Setting one requires the device to have a clock.
    pub expires: u64,
}

impl Delegation {
    /// Encodes the delegation body, which is the part a signature covers.
    ///
    /// # Arguments
    ///
    /// * `buf` - the destination.
    ///
    /// # Returns
    ///
    /// How many bytes were written.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Malformed`] if `buf` is too small.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        let mut writer = Writer::new(buf);
        writer.map(3)?;
        writer.uint(KEY_EPOCH)?;
        writer.uint(self.epoch)?;
        writer.uint(KEY_RELEASE)?;
        writer.bytes(&self.release_key)?;
        writer.uint(KEY_EXPIRES)?;
        writer.uint(self.expires)?;
        Ok(writer.finish())
    }

    /// Decodes a delegation body.
    ///
    /// # Arguments
    ///
    /// * `bytes` - an encoded delegation body.
    ///
    /// # Returns
    ///
    /// The delegation.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Malformed`] if the encoding is not a well-formed
    /// delegation with its keys in order.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(bytes);
        if reader.map()? != 3 {
            return Err(Refusal::Malformed);
        }

        let epoch = read_key(&mut reader, KEY_EPOCH)?;

        expect_key(&mut reader, KEY_RELEASE)?;
        let release_key =
            <[u8; KEY_LEN]>::try_from(reader.bytes()?).map_err(|_| Refusal::Malformed)?;

        let expires = read_key(&mut reader, KEY_EXPIRES)?;

        if reader.position() != bytes.len() {
            return Err(Refusal::Malformed);
        }

        Ok(Self {
            epoch,
            release_key,
            expires,
        })
    }

    /// Encodes the delegation and signs it with the trust anchor.
    ///
    /// # Arguments
    ///
    /// * `anchor` - the trust anchor's identity, which alone may delegate.
    /// * `buf` - the destination, at least [`DELEGATION_MAX`] bytes.
    ///
    /// # Returns
    ///
    /// How many bytes of `buf` the envelope occupies.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Malformed`] if `buf` is too small.
    pub fn sign(&self, anchor: &DeviceIdentity, buf: &mut [u8]) -> Result<usize> {
        let mut body = [0u8; DELEGATION_MAX];
        let body_len = self.encode(&mut body)?;
        seal(&body[..body_len], anchor, buf)
    }

    /// Checks a delegation envelope against a trust anchor and reads it.
    ///
    /// # Arguments
    ///
    /// * `envelope` - the signed delegation.
    /// * `anchor` - the key the device anchors its trust in.
    ///
    /// # Returns
    ///
    /// The delegation, now known to be from the anchor and unaltered.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Signature`] if it is not the anchor's, or a decoding
    /// refusal if the body is not a valid delegation.
    pub fn open(envelope: &[u8], anchor: &PublicIdentity) -> Result<Self> {
        let body = Envelope::decode(envelope)?.verified_body(anchor)?;
        Self::decode(body)
    }

    /// Returns the release key as an identity that can check a manifest.
    ///
    /// # Returns
    ///
    /// The delegated public key.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Signature`] if the delegated bytes are not a usable
    /// public key, so a malformed delegation cannot leave a device trusting
    /// nothing while believing it trusts something.
    pub fn signer(&self) -> Result<PublicIdentity> {
        PublicIdentity::from_bytes(&self.release_key).map_err(|_| Refusal::Signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The anchor the tests delegate from.
    fn anchor() -> DeviceIdentity {
        DeviceIdentity::from_seed(&[1u8; 32])
    }

    /// A delegation naming a release key derived from `seed`.
    fn delegation(epoch: u64, seed: u8) -> Delegation {
        Delegation {
            epoch,
            release_key: DeviceIdentity::from_seed(&[seed; 32]).public().to_bytes(),
            expires: 0,
        }
    }

    #[test]
    fn a_delegation_round_trips() {
        let original = delegation(3, 2);
        let mut buf = [0u8; DELEGATION_MAX];
        let written = original.encode(&mut buf).expect("encode");
        assert_eq!(
            Delegation::decode(&buf[..written]).expect("decode"),
            original
        );
    }

    #[test]
    fn the_encoding_fits_the_documented_buffer_size() {
        let mut buf = [0u8; DELEGATION_MAX];
        assert!(delegation(u64::MAX, 2).sign(&anchor(), &mut buf).is_ok());
    }

    #[test]
    fn a_delegation_opens_against_its_anchor() {
        let original = delegation(1, 2);
        let mut buf = [0u8; DELEGATION_MAX];
        let written = original.sign(&anchor(), &mut buf).expect("sign");
        assert_eq!(
            Delegation::open(&buf[..written], &anchor().public()).expect("open"),
            original
        );
    }

    #[test]
    fn a_delegation_from_anyone_else_is_refused() {
        let impostor = DeviceIdentity::from_seed(&[9u8; 32]);
        let mut buf = [0u8; DELEGATION_MAX];
        let written = delegation(1, 2).sign(&impostor, &mut buf).expect("sign");
        assert_eq!(
            Delegation::open(&buf[..written], &anchor().public()),
            Err(Refusal::Signature),
            "only the trust anchor may hand on the right to sign updates"
        );
    }

    #[test]
    fn altering_a_delegation_breaks_its_signature() {
        let mut buf = [0u8; DELEGATION_MAX];
        let written = delegation(1, 2).sign(&anchor(), &mut buf).expect("sign");
        // Swap in a different release key inside the signed body.
        buf[10] ^= 0x01;
        assert_eq!(
            Delegation::open(&buf[..written], &anchor().public()),
            Err(Refusal::Signature)
        );
    }

    #[test]
    fn the_delegated_key_is_usable() {
        let release = DeviceIdentity::from_seed(&[2u8; 32]);
        let signer = delegation(1, 2).signer().expect("signer");
        assert_eq!(signer.to_bytes(), release.public().to_bytes());
    }
}
