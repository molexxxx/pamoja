//! Generated Python bindings for tamper-evident audit logs.
//!
//! These mirror the `pamoja-audit` Rust API: a log that signs each record and
//! chains it to the one before, and the two ways to check such a chain, one
//! entry at a time as it streams in or all at once over a batch that has
//! arrived.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_audit::{verify_chain, AuditLog as CoreLog, Entry, Verifier};
use pamoja_security::PublicIdentity;

use crate::security::DeviceIdentity;
use crate::PamojaError;

/// The length in bytes of a public key.
const KEY_LEN: usize = 32;

/// One signed record, chained onto the one before it.
#[gen_stub_pyclass]
// Opted in because `verify_audit_chain` takes a list of these by value.
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct AuditEntry {
    pub(crate) inner: Entry,
}

#[gen_stub_pymethods]
#[pymethods]
impl AuditEntry {
    /// Reads an entry back from the bytes it was written as.
    #[staticmethod]
    fn from_bytes(data: Vec<u8>) -> PyResult<Self> {
        Entry::from_bytes(&data)
            .map(|inner| AuditEntry { inner })
            .map_err(|error| PamojaError::new_err(error.to_string()))
    }

    /// The position of this entry in its chain.
    #[getter]
    fn index(&self) -> u64 {
        self.inner.index()
    }

    /// The hash of the entry before this one, all zeroes for the first.
    #[getter]
    fn previous(&self) -> Vec<u8> {
        self.inner.previous().to_vec()
    }

    /// The hash of this entry, which the next one chains onto.
    #[getter]
    fn digest(&self) -> Vec<u8> {
        self.inner.digest().to_vec()
    }

    /// The record this entry carries.
    #[getter]
    fn payload(&self) -> Vec<u8> {
        self.inner.payload().to_vec()
    }

    /// The signature over this entry.
    #[getter]
    fn signature(&self) -> Vec<u8> {
        self.inner.signature().to_bytes().to_vec()
    }

    /// Encodes this entry for storage or transmission.
    fn to_bytes(&self) -> Vec<u8> {
        self.inner.to_bytes()
    }
}

/// A log that signs what it is given and chains it onto what came before.
#[gen_stub_pyclass]
#[pyclass]
pub struct AuditLog {
    inner: CoreLog,
}

#[gen_stub_pymethods]
#[pymethods]
impl AuditLog {
    /// Creates a log that signs with a device identity, starting empty.
    #[new]
    fn new(identity: &DeviceIdentity) -> Self {
        AuditLog {
            inner: CoreLog::new(identity.inner.clone()),
        }
    }

    /// Creates a log that carries on from the last entry an earlier one wrote.
    ///
    /// This is what a device does after a restart: the chain continues at the
    /// next index and hashes onto the entry it left off at, so a reboot leaves
    /// no gap for a record to be removed through.
    #[staticmethod]
    fn resume(identity: &DeviceIdentity, last: &AuditEntry) -> Self {
        AuditLog {
            inner: CoreLog::resume(identity.inner.clone(), &last.inner),
        }
    }

    /// Appends a payload, signing it and chaining it onto the last entry.
    fn append(&mut self, payload: Vec<u8>) -> AuditEntry {
        AuditEntry {
            inner: self.inner.append(&payload),
        }
    }
}

/// Checks a chain one entry at a time, in the order the entries were written.
#[gen_stub_pyclass]
#[pyclass]
pub struct AuditVerifier {
    inner: Verifier,
}

#[gen_stub_pymethods]
#[pymethods]
impl AuditVerifier {
    /// Creates a verifier for a chain signed by a 32-byte public key.
    #[new]
    fn new(public_key: Vec<u8>) -> PyResult<Self> {
        Ok(AuditVerifier {
            inner: Verifier::new(public(&public_key)?),
        })
    }

    /// Checks the next entry, returning whether it belongs where it was offered.
    ///
    /// Feeding entries out of order, skipping one, or repeating one is refused
    /// just as an altered payload is.
    fn check(&mut self, entry: &AuditEntry) -> bool {
        self.inner.check(&entry.inner).is_ok()
    }
}

/// Checks a whole chain that has already arrived.
///
/// Returns `False` if any entry fails to follow the one before it or carries a
/// signature that does not hold.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn verify_audit_chain(public_key: Vec<u8>, entries: Vec<AuditEntry>) -> PyResult<bool> {
    let key = public(&public_key)?;
    let owned: Vec<Entry> = entries.into_iter().map(|entry| entry.inner).collect();
    Ok(verify_chain(&key, &owned).is_ok())
}

/// Reads a 32-byte public key, rejecting one that is not a valid key.
fn public(bytes: &[u8]) -> PyResult<PublicIdentity> {
    let key = <[u8; KEY_LEN]>::try_from(bytes)
        .map_err(|_| PamojaError::new_err(format!("public_key must be exactly {KEY_LEN} bytes")))?;
    PublicIdentity::from_bytes(&key).map_err(|error| PamojaError::new_err(error.to_string()))
}
