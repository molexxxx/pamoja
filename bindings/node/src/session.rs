//! Generated Node bindings for encrypted, authenticated sessions.
//!
//! These mirror the `pamoja-session` Rust API: the key agreement two devices use
//! to arrive at the same session key without sending it, and the sealed messages
//! that key then protects.
//!
//! The Rust methods encrypt in place. JavaScript buffers are copied across the
//! boundary anyway, so these take a plaintext and return a ciphertext instead of
//! pretending to mutate what the caller passed.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_session::{
    hkdf_sha256, hmac_sha256, AgreementKey as CoreKey, AgreementPublicKey, Role as CoreRole,
    Sealed, Session as CoreSession,
};

/// The length in bytes of an agreement seed, a public key, and a digest.
const KEY_LEN: usize = 32;

/// The length in bytes of the tag that authenticates a sealed message.
const TAG_LEN: usize = 16;

/// Which side of a session a device is on.
///
/// The two devices must choose opposite roles. The role decides the order the
/// public keys are mixed in and which direction each side tags its messages
/// with, so a session where both sides claim the same role opens nothing.
#[napi(string_enum)]
pub enum Role {
    /// The device that opens the session.
    Initiator,
    /// The device that answers.
    Responder,
}

/// A message that has been sealed, with the header that travels beside it.
#[napi(object)]
pub struct SealedMessage {
    /// The counter naming this message within the session.
    pub counter: f64,
    /// The tag over the ciphertext and its associated data.
    pub tag: Buffer,
    /// The encrypted message.
    pub ciphertext: Buffer,
}

/// A key-agreement secret, and the public key to hand to a peer.
#[napi]
pub struct AgreementKey {
    inner: CoreKey,
}

#[napi]
impl AgreementKey {
    /// Creates a key-agreement secret from a provisioned 32-byte seed.
    #[napi(constructor)]
    pub fn new(seed: Buffer) -> napi::Result<Self> {
        let seed = fixed::<KEY_LEN>(seed.as_ref(), "seed")?;
        Ok(Self {
            inner: CoreKey::from_seed(&seed),
        })
    }

    /// The public key to hand to a peer.
    #[napi]
    pub fn public_key(&self) -> Buffer {
        self.inner.public().to_bytes().to_vec().into()
    }
}

/// A confidential, tamper-evident, replay-protected channel with one peer.
#[napi]
pub struct Session {
    inner: CoreSession,
}

#[napi]
impl Session {
    /// Establishes a session with a peer.
    ///
    /// Both devices call this with the same salt and opposite roles, and arrive
    /// at the same key without either sending it. The salt is a fresh
    /// per-session value exchanged in the clear; reusing one with the same pair
    /// of keys reuses the session key, so it must change each session.
    #[napi(constructor)]
    pub fn new(
        local: &AgreementKey,
        peer_public_key: Buffer,
        salt: Buffer,
        role: Role,
    ) -> napi::Result<Self> {
        let peer = fixed::<KEY_LEN>(peer_public_key.as_ref(), "peerPublicKey")?;
        Ok(Self {
            inner: CoreSession::establish(
                &local.inner,
                &AgreementPublicKey::from_bytes(&peer),
                salt.as_ref(),
                core_role(role),
            ),
        })
    }

    /// Seals a message for the peer.
    ///
    /// The associated data is authenticated but not encrypted, so it stays
    /// readable on the wire yet cannot be altered: a device identifier or a
    /// routing header belongs there.
    #[napi]
    pub fn seal(&mut self, plaintext: Buffer, aad: Option<Buffer>) -> SealedMessage {
        let mut message = plaintext.to_vec();
        let aad = aad.map(|aad| aad.to_vec()).unwrap_or_default();
        let sealed = self.inner.seal(&mut message, &aad);
        SealedMessage {
            counter: sealed.counter as f64,
            tag: sealed.tag.to_vec().into(),
            ciphertext: message.into(),
        }
    }

    /// Opens a message from the peer, returning the plaintext.
    ///
    /// Throws if the counter repeats or is older than the replay window still
    /// tracks, and if the tag does not authenticate. Nothing readable is ever
    /// returned from a message that failed either check.
    #[napi]
    pub fn open(&mut self, sealed: SealedMessage, aad: Option<Buffer>) -> napi::Result<Buffer> {
        let tag = fixed::<TAG_LEN>(sealed.tag.as_ref(), "tag")?;
        let mut message = sealed.ciphertext.to_vec();
        let aad = aad.map(|aad| aad.to_vec()).unwrap_or_default();
        let header = Sealed {
            counter: sealed.counter as u64,
            tag,
        };
        self.inner
            .open(&header, &mut message, &aad)
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        Ok(message.into())
    }
}

/// Computes a keyed hash over a message.
///
/// This is the primitive a host uses to authenticate a pairing exchange or a
/// single command, where a whole session would be more than the job needs.
#[napi]
pub fn hmac_sha256_digest(key: Buffer, message: Buffer) -> Buffer {
    hmac_sha256(key.as_ref(), message.as_ref()).to_vec().into()
}

/// Expands input keying material into `length` bytes bound to `info`.
#[napi]
pub fn hkdf_sha256_expand(salt: Buffer, ikm: Buffer, info: Buffer, length: u32) -> Buffer {
    let mut derived = vec![0u8; length as usize];
    hkdf_sha256(salt.as_ref(), ikm.as_ref(), info.as_ref(), &mut derived);
    derived.into()
}

/// Reads a fixed-width argument, naming it in the error when the length is wrong.
fn fixed<const N: usize>(bytes: &[u8], name: &str) -> napi::Result<[u8; N]> {
    <[u8; N]>::try_from(bytes)
        .map_err(|_| napi::Error::from_reason(format!("{name} must be exactly {N} bytes")))
}

/// Maps a JavaScript role back onto the core one.
fn core_role(role: Role) -> CoreRole {
    match role {
        Role::Initiator => CoreRole::Initiator,
        Role::Responder => CoreRole::Responder,
    }
}
