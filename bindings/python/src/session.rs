//! Generated Python bindings for encrypted, authenticated sessions.
//!
//! These mirror the `pamoja-session` Rust API: the key agreement two devices use
//! to arrive at the same session key without sending it, and the sealed messages
//! that key then protects.
//!
//! The Rust methods encrypt in place. Python bytes are immutable, so these take
//! a plaintext and return a ciphertext instead of pretending to mutate what the
//! caller passed.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_session::{
    hkdf_sha256, hmac_sha256, AgreementKey as CoreKey, AgreementPublicKey, Role, Sealed,
    Session as CoreSession,
};

use crate::PamojaError;

/// The length in bytes of an agreement seed, a public key, and a digest.
const KEY_LEN: usize = 32;

/// The length in bytes of the tag that authenticates a sealed message.
const TAG_LEN: usize = 16;

/// A message that has been sealed, with the header that travels beside it.
#[gen_stub_pyclass]
#[pyclass]
pub struct SealedMessage {
    /// The counter naming this message within the session.
    #[pyo3(get)]
    counter: u64,
    /// The tag over the ciphertext and its associated data.
    #[pyo3(get)]
    tag: Vec<u8>,
    /// The encrypted message.
    #[pyo3(get)]
    ciphertext: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl SealedMessage {
    /// Rebuilds a sealed message from the parts that arrived over the wire.
    #[new]
    fn new(counter: u64, tag: Vec<u8>, ciphertext: Vec<u8>) -> Self {
        SealedMessage {
            counter,
            tag,
            ciphertext,
        }
    }
}

/// A key-agreement secret, and the public key to hand to a peer.
#[gen_stub_pyclass]
#[pyclass]
pub struct AgreementKey {
    inner: CoreKey,
}

#[gen_stub_pymethods]
#[pymethods]
impl AgreementKey {
    /// Creates a key-agreement secret from a provisioned 32-byte seed.
    #[new]
    fn new(seed: Vec<u8>) -> PyResult<Self> {
        let seed = fixed::<KEY_LEN>(&seed, "seed")?;
        Ok(AgreementKey {
            inner: CoreKey::from_seed(&seed),
        })
    }

    /// The public key to hand to a peer.
    #[getter]
    fn public_key(&self) -> Vec<u8> {
        self.inner.public().to_bytes().to_vec()
    }
}

/// A confidential, tamper-evident, replay-protected channel with one peer.
#[gen_stub_pyclass]
#[pyclass]
pub struct Session {
    inner: CoreSession,
}

#[gen_stub_pymethods]
#[pymethods]
impl Session {
    /// Establishes a session with a peer.
    ///
    /// Both devices call this with the same salt and opposite roles, and arrive
    /// at the same key without either sending it. The salt is a fresh
    /// per-session value exchanged in the clear; reusing one with the same pair
    /// of keys reuses the session key, so it must change each session.
    #[new]
    fn new(
        local: &AgreementKey,
        peer_public_key: Vec<u8>,
        salt: Vec<u8>,
        role: &str,
    ) -> PyResult<Self> {
        let peer = fixed::<KEY_LEN>(&peer_public_key, "peer_public_key")?;
        Ok(Session {
            inner: CoreSession::establish(
                &local.inner,
                &AgreementPublicKey::from_bytes(&peer),
                &salt,
                core_role(role)?,
            ),
        })
    }

    /// Seals a message for the peer.
    ///
    /// The associated data is authenticated but not encrypted, so it stays
    /// readable on the wire yet cannot be altered: a device identifier or a
    /// routing header belongs there.
    #[pyo3(signature = (plaintext, aad = Vec::new()))]
    fn seal(&mut self, plaintext: Vec<u8>, aad: Vec<u8>) -> SealedMessage {
        let mut message = plaintext;
        let sealed = self.inner.seal(&mut message, &aad);
        SealedMessage {
            counter: sealed.counter,
            tag: sealed.tag.to_vec(),
            ciphertext: message,
        }
    }

    /// Opens a message from the peer, returning the plaintext.
    ///
    /// Raises if the counter repeats or is older than the replay window still
    /// tracks, and if the tag does not authenticate. Nothing readable is ever
    /// returned from a message that failed either check.
    #[pyo3(signature = (sealed, aad = Vec::new()))]
    fn open(&mut self, sealed: &SealedMessage, aad: Vec<u8>) -> PyResult<Vec<u8>> {
        let tag = fixed::<TAG_LEN>(&sealed.tag, "tag")?;
        let mut message = sealed.ciphertext.clone();
        let header = Sealed {
            counter: sealed.counter,
            tag,
        };
        self.inner
            .open(&header, &mut message, &aad)
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        Ok(message)
    }
}

/// Computes a keyed hash over a message.
///
/// This is the primitive a host uses to authenticate a pairing exchange or a
/// single command, where a whole session would be more than the job needs.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hmac_sha256_digest(key: Vec<u8>, message: Vec<u8>) -> Vec<u8> {
    hmac_sha256(&key, &message).to_vec()
}

/// Expands input keying material into `length` bytes bound to `info`.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn hkdf_sha256_expand(salt: Vec<u8>, ikm: Vec<u8>, info: Vec<u8>, length: usize) -> Vec<u8> {
    let mut derived = vec![0u8; length];
    hkdf_sha256(&salt, &ikm, &info, &mut derived);
    derived
}

/// Reads a fixed-width argument, naming it in the error when the length is wrong.
fn fixed<const N: usize>(bytes: &[u8], name: &str) -> PyResult<[u8; N]> {
    <[u8; N]>::try_from(bytes)
        .map_err(|_| PamojaError::new_err(format!("{name} must be exactly {N} bytes")))
}

/// Reads a role back from its name, refusing one that is not a role.
fn core_role(role: &str) -> PyResult<Role> {
    match role {
        "Initiator" => Ok(Role::Initiator),
        "Responder" => Ok(Role::Responder),
        other => Err(PamojaError::new_err(format!("unknown role {other}"))),
    }
}
