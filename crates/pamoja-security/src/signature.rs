//! The signature a device produces over a payload.

use ed25519_dalek::Signature as Ed25519Signature;

/// A detached ed25519 signature over a payload.
///
/// A signature is 64 bytes on the wire. Send it alongside the payload it covers, and
/// the receiver checks it with the signer's [`PublicIdentity`](crate::PublicIdentity)
/// to confirm the payload came from that device and was not altered in transit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signature(pub(crate) Ed25519Signature);

impl Signature {
    /// The length of a signature on the wire, in bytes.
    pub const LEN: usize = 64;

    /// Returns the 64-byte wire form of the signature.
    ///
    /// # Returns
    ///
    /// The signature encoded as 64 bytes.
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0.to_bytes()
    }

    /// Reconstructs a signature from its 64-byte wire form.
    ///
    /// The bytes are not validated here; an invalid signature is rejected when it is
    /// checked by [`PublicIdentity::verify`](crate::PublicIdentity::verify).
    ///
    /// # Arguments
    ///
    /// * `bytes` - the 64-byte encoded signature.
    ///
    /// # Returns
    ///
    /// The signature.
    pub fn from_bytes(bytes: &[u8; 64]) -> Self {
        Self(Ed25519Signature::from_bytes(bytes))
    }
}
