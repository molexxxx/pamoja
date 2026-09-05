//! Generated Node bindings for tamper-evident audit logs.
//!
//! These mirror the `pamoja-audit` Rust API: a log that signs each record and
//! chains it to the one before, and the two ways to check such a chain, one entry
//! at a time as it streams in or all at once over a batch that has arrived.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_audit::{verify_chain, AuditLog as CoreLog, Entry, Verifier as CoreVerifier};
use pamoja_security::PublicIdentity;

use crate::security::DeviceIdentity;

/// The length in bytes of an identity seed and of a public key.
const KEY_LEN: usize = 32;

/// One signed record, chained onto the one before it.
#[napi]
pub struct AuditEntry {
    inner: Entry,
}

#[napi]
impl AuditEntry {
    /// Reads an entry back from the bytes it was written as.
    #[napi(factory)]
    pub fn from_bytes(bytes: Buffer) -> napi::Result<Self> {
        Entry::from_bytes(bytes.as_ref())
            .map(|inner| Self { inner })
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// The position of this entry in its chain.
    #[napi(getter)]
    pub fn index(&self) -> f64 {
        self.inner.index() as f64
    }

    /// The hash of the entry before this one, all zeroes for the first.
    #[napi(getter)]
    pub fn previous(&self) -> Buffer {
        self.inner.previous().to_vec().into()
    }

    /// The hash of this entry, which the next one chains onto.
    #[napi(getter)]
    pub fn digest(&self) -> Buffer {
        self.inner.digest().to_vec().into()
    }

    /// The record this entry carries.
    #[napi(getter)]
    pub fn payload(&self) -> Buffer {
        self.inner.payload().to_vec().into()
    }

    /// The signature over this entry.
    #[napi(getter)]
    pub fn signature(&self) -> Buffer {
        self.inner.signature().to_bytes().to_vec().into()
    }

    /// Encodes this entry for storage or transmission.
    #[napi]
    pub fn to_bytes(&self) -> Buffer {
        self.inner.to_bytes().into()
    }
}

/// A log that signs what it is given and chains it onto what came before.
#[napi]
pub struct AuditLog {
    inner: CoreLog,
}

#[napi]
impl AuditLog {
    /// Creates a log that signs with a device identity, starting empty.
    #[napi(constructor)]
    pub fn new(identity: &DeviceIdentity) -> Self {
        Self {
            inner: CoreLog::new(identity.inner.clone()),
        }
    }

    /// Creates a log that carries on from the last entry an earlier one wrote.
    ///
    /// This is what a device does after a restart: the chain continues at the
    /// next index and hashes onto the entry it left off at, so a reboot leaves no
    /// gap for a record to be removed through.
    #[napi(factory)]
    pub fn resume(identity: &DeviceIdentity, last: &AuditEntry) -> Self {
        Self {
            inner: CoreLog::resume(identity.inner.clone(), &last.inner),
        }
    }

    /// Appends a payload, signing it and chaining it onto the last entry.
    #[napi]
    pub fn append(&mut self, payload: Buffer) -> AuditEntry {
        AuditEntry {
            inner: self.inner.append(payload.as_ref()),
        }
    }
}

/// Checks a chain one entry at a time, in the order the entries were written.
#[napi]
pub struct AuditVerifier {
    inner: CoreVerifier,
}

#[napi]
impl AuditVerifier {
    /// Creates a verifier for a chain signed by a 32-byte public key.
    #[napi(constructor)]
    pub fn new(public_key: Buffer) -> napi::Result<Self> {
        Ok(Self {
            inner: CoreVerifier::new(public(public_key.as_ref())?),
        })
    }

    /// Checks the next entry, returning whether it belongs where it was offered.
    ///
    /// Feeding entries out of order, skipping one, or repeating one is refused
    /// just as an altered payload is.
    #[napi]
    pub fn check(&mut self, entry: &AuditEntry) -> bool {
        self.inner.check(&entry.inner).is_ok()
    }
}

/// Checks a whole chain that has already arrived.
///
/// Throws with the reason if any entry fails to follow the one before it or
/// carries a signature that does not hold.
#[napi]
pub fn verify_audit_chain(public_key: Buffer, entries: Vec<&AuditEntry>) -> napi::Result<()> {
    let key = public(public_key.as_ref())?;
    let owned: Vec<Entry> = entries.iter().map(|entry| entry.inner.clone()).collect();
    verify_chain(&key, &owned).map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Reads a 32-byte public key, rejecting one that is not a valid key.
fn public(bytes: &[u8]) -> napi::Result<PublicIdentity> {
    let key = <[u8; KEY_LEN]>::try_from(bytes).map_err(|_| {
        napi::Error::from_reason(format!("publicKey must be exactly {KEY_LEN} bytes"))
    })?;
    PublicIdentity::from_bytes(&key).map_err(|error| napi::Error::from_reason(error.to_string()))
}
