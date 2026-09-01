//! Generated Python bindings for wire formats and metered-link packing.
//!
//! These mirror the `pamoja-codec` Rust API for callers that hold an untyped
//! document. The `Codec` trait is generic over the value it carries and has no
//! Python equivalent, so what is exposed here is the concrete work: moving a
//! document between JSON and CBOR, and packing a batch of readings small enough
//! for a metered link.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer as Inner};

use crate::PamojaError;

/// Converts a JSON document into its CBOR encoding, which is typically smaller.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn json_to_cbor_bytes<'py>(py: Python<'py>, json: Vec<u8>) -> PyResult<Bound<'py, PyBytes>> {
    json_to_cbor(&json)
        .map(|bytes| PyBytes::new(py, &bytes))
        .map_err(to_py)
}

/// Converts a CBOR document back into its JSON encoding.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn cbor_to_json_bytes<'py>(py: Python<'py>, cbor: Vec<u8>) -> PyResult<Bound<'py, PyBytes>> {
    cbor_to_json(&cbor)
        .map(|bytes| PyBytes::new(py, &bytes))
        .map_err(to_py)
}

/// Delta-encodes a series of integer samples into a compact buffer.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn encode_delta_samples<'py>(py: Python<'py>, samples: Vec<i64>) -> Bound<'py, PyBytes> {
    PyBytes::new(py, &encode_deltas(&samples))
}

/// Decodes a delta-encoded buffer back into its integer samples.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn decode_delta_samples(bytes: Vec<u8>) -> PyResult<Vec<i64>> {
    decode_deltas(&bytes).map_err(to_py)
}

/// Packs float readings to a fixed precision for a metered link.
#[gen_stub_pyclass]
#[pyclass]
pub struct Quantizer {
    inner: Inner,
}

#[gen_stub_pymethods]
#[pymethods]
impl Quantizer {
    /// Creates a quantizer whose `scale` sets the precision kept.
    ///
    /// A scale of `100` keeps two decimal places. It must be positive and finite,
    /// and decoding must use the same scale the batch was encoded with.
    #[new]
    fn new(scale: f32) -> PyResult<Self> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "scale must be positive and finite",
            ));
        }
        Ok(Self {
            inner: Inner::new(scale),
        })
    }

    /// Quantizes and delta-encodes a batch of readings.
    fn encode<'py>(&self, py: Python<'py>, readings: Vec<f32>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.encode(&readings))
    }

    /// Decodes a batch back into readings, to within the quantizer's precision.
    fn decode(&self, bytes: Vec<u8>) -> PyResult<Vec<f32>> {
        self.inner.decode(&bytes).map_err(to_py)
    }
}

/// Maps a core error onto the SDK's Python exception.
fn to_py(error: pamoja_core::Error) -> PyErr {
    PamojaError::new_err(error.to_string())
}
