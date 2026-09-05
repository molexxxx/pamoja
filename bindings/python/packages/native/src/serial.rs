//! Generated Python bindings for serial-line packet framing.
//!
//! These mirror the `pamoja-serial` Rust API: SLIP and COBS, both as a one-shot
//! call over a complete frame and as a streaming decoder for the arbitrary chunks
//! a UART hands an application.
//!
//! The Rust decoders take one byte at a time. A Python call per byte would cost
//! far more than the decoding does, so the decoders here take a chunk and run the
//! same per-byte loop natively. A corrupt frame inside a chunk does not raise,
//! because the frames around it are still good; it is discarded and counted, and
//! the count is readable from `discarded`.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_serial::{cobs, slip, SerialError};

use crate::PamojaError;

/// The largest payload, in bytes, that a streaming decoder will reassemble.
const FRAME_MAX: usize = 2048;

/// Frames a payload as a SLIP packet (RFC 1055).
#[gen_stub_pyfunction]
#[pyfunction]
pub fn slip_encode<'py>(py: Python<'py>, payload: Vec<u8>) -> PyResult<Bound<'py, PyBytes>> {
    let mut out = vec![0u8; slip::max_encoded_len(payload.len())];
    let written = slip::encode(&payload, &mut out).map_err(to_py)?;
    Ok(PyBytes::new(py, &out[..written]))
}

/// Reads the payload back out of a SLIP frame.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn slip_decode<'py>(py: Python<'py>, frame: Vec<u8>) -> PyResult<Bound<'py, PyBytes>> {
    let mut out = vec![0u8; frame.len()];
    let written = slip::decode(&frame, &mut out).map_err(to_py)?;
    Ok(PyBytes::new(py, &out[..written]))
}

/// Frames a payload as a COBS packet, terminated by its zero delimiter.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn cobs_encode<'py>(py: Python<'py>, payload: Vec<u8>) -> PyResult<Bound<'py, PyBytes>> {
    let mut out = vec![0u8; cobs::max_encoded_len(payload.len())];
    let written = cobs::encode(&payload, &mut out).map_err(to_py)?;
    Ok(PyBytes::new(py, &out[..written]))
}

/// Reads the payload back out of a COBS frame.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn cobs_decode<'py>(py: Python<'py>, frame: Vec<u8>) -> PyResult<Bound<'py, PyBytes>> {
    let mut out = vec![0u8; frame.len()];
    let written = cobs::decode(&frame, &mut out).map_err(to_py)?;
    Ok(PyBytes::new(py, &out[..written]))
}

/// Returns the reserved framing bytes: SLIP end, escape, the two escape codes, and
/// the COBS delimiter.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn serial_framing_bytes() -> (u8, u8, u8, u8, u8) {
    (
        slip::END,
        slip::ESC,
        slip::ESC_END,
        slip::ESC_ESC,
        cobs::DELIMITER,
    )
}

/// Returns the largest SLIP frame a payload of this length can produce.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn slip_max_encoded_len(payload_len: usize) -> usize {
    slip::max_encoded_len(payload_len)
}

/// Returns the largest COBS frame a payload of this length can produce.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn cobs_max_encoded_len(payload_len: usize) -> usize {
    cobs::max_encoded_len(payload_len)
}

/// Reassembles whole SLIP frames from the chunks a serial port delivers.
#[gen_stub_pyclass]
#[pyclass]
pub struct SlipDecoder {
    inner: slip::SlipDecoder<FRAME_MAX>,
    discarded: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl SlipDecoder {
    /// Creates an empty decoder, ready for the first chunk.
    #[new]
    fn new() -> Self {
        Self {
            inner: slip::SlipDecoder::new(),
            discarded: 0,
        }
    }

    /// Feeds a chunk of the stream and returns every frame it completed.
    fn feed<'py>(&mut self, py: Python<'py>, chunk: Vec<u8>) -> Vec<Bound<'py, PyBytes>> {
        let mut frames = Vec::new();
        for &byte in &chunk {
            match self.inner.push(byte) {
                Ok(Some(frame)) => frames.push(PyBytes::new(py, frame)),
                Ok(None) => {}
                Err(_) => self.discarded += 1,
            }
        }
        frames
    }

    /// How many corrupt frames this decoder has discarded.
    #[getter]
    fn discarded(&self) -> u64 {
        self.discarded
    }

    /// Discards any partly assembled frame.
    fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Reassembles whole COBS frames from the chunks a serial port delivers.
#[gen_stub_pyclass]
#[pyclass]
pub struct CobsDecoder {
    inner: cobs::CobsDecoder<FRAME_MAX>,
    discarded: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl CobsDecoder {
    /// Creates an empty decoder, ready for the first chunk.
    #[new]
    fn new() -> Self {
        Self {
            inner: cobs::CobsDecoder::new(),
            discarded: 0,
        }
    }

    /// Feeds a chunk of the stream and returns every frame it completed.
    fn feed<'py>(&mut self, py: Python<'py>, chunk: Vec<u8>) -> Vec<Bound<'py, PyBytes>> {
        let mut frames = Vec::new();
        for &byte in &chunk {
            match self.inner.push(byte) {
                Ok(Some(frame)) => frames.push(PyBytes::new(py, frame)),
                Ok(None) => {}
                Err(_) => self.discarded += 1,
            }
        }
        frames
    }

    /// How many corrupt frames this decoder has discarded.
    #[getter]
    fn discarded(&self) -> u64 {
        self.discarded
    }

    /// Discards any partly assembled frame.
    fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Maps a framing error onto the SDK's Python exception.
fn to_py(error: SerialError) -> PyErr {
    PamojaError::new_err(error.to_string())
}
