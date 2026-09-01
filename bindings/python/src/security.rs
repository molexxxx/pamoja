//! Generated Python bindings for device identity and signed telemetry.
//!
//! These mirror the `pamoja-security` Rust API one-to-one. Signing and verifying
//! are deterministic and need no runtime, so unlike the MQTT transport nothing
//! here is awaitable.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_security::{DeviceIdentity as CoreIdentity, PublicIdentity, Signature};

use crate::PamojaError;

/// The length in bytes of an identity seed and of a public key.
const KEY_LEN: usize = 32;

/// The length in bytes of a signature.
const SIGNATURE_LEN: usize = 64;

/// A device's private signing identity.
#[gen_stub_pyclass]
#[pyclass]
pub struct DeviceIdentity {
    inner: CoreIdentity,
}

#[gen_stub_pymethods]
#[pymethods]
impl DeviceIdentity {
    /// Creates an identity from a provisioned 32-byte secret seed.
    #[new]
    fn new(seed: Vec<u8>) -> PyResult<Self> {
        Ok(Self {
            inner: CoreIdentity::from_seed(&fixed::<KEY_LEN>(&seed, "seed")?),
        })
    }

    /// The public key matching this identity, which is safe to share.
    #[getter]
    fn public_key<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.public().to_bytes())
    }

    /// The short hex fingerprint of this identity, for logs and displays.
    #[getter]
    fn fingerprint(&self) -> String {
        self.inner.public().fingerprint()
    }

    /// Signs a payload, returning the 64-byte detached signature.
    fn sign<'py>(&self, py: Python<'py>, payload: Vec<u8>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.sign(&payload).to_bytes())
    }

    fn __repr__(&self) -> String {
        format!(
            "DeviceIdentity(fingerprint={:?})",
            self.inner.public().fingerprint()
        )
    }
}

/// Verifies that a signature covers a payload and was made by a public key.
///
/// Returns `False` when the payload was altered or was signed by a different
/// device, and raises only when an argument is the wrong length.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn verify(public_key: Vec<u8>, payload: Vec<u8>, signature: Vec<u8>) -> PyResult<bool> {
    let key = fixed::<KEY_LEN>(&public_key, "public_key")?;
    let signature = fixed::<SIGNATURE_LEN>(&signature, "signature")?;
    let Ok(public) = PublicIdentity::from_bytes(&key) else {
        return Ok(false);
    };
    Ok(public
        .verify(&payload, &Signature::from_bytes(&signature))
        .is_ok())
}

/// Returns the short hex fingerprint of a public key.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn fingerprint(public_key: Vec<u8>) -> PyResult<String> {
    let key = fixed::<KEY_LEN>(&public_key, "public_key")?;
    PublicIdentity::from_bytes(&key)
        .map(|public| public.fingerprint())
        .map_err(|error| PamojaError::new_err(error.to_string()))
}

/// Reads a fixed-width argument, naming it in the error when the length is wrong.
fn fixed<const N: usize>(bytes: &[u8], name: &str) -> PyResult<[u8; N]> {
    <[u8; N]>::try_from(bytes).map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(format!("{name} must be exactly {N} bytes"))
    })
}
