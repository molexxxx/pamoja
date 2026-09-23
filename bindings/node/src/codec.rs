//! Generated Node bindings for wire formats and metered-link packing.
//!
//! These mirror the `pamoja-codec` Rust API for callers that hold an untyped
//! document. The `Codec` trait is generic over the value it carries and has no
//! JavaScript equivalent, so what is exposed here is the concrete work: moving a
//! document between JSON and CBOR, and packing a batch of readings small enough
//! for a metered link.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer as Inner};

/// Converts a JSON document into its CBOR encoding, which is typically smaller.
#[napi]
pub fn json_to_cbor_bytes(json: Buffer) -> napi::Result<Buffer> {
    json_to_cbor(json.as_ref()).map(Into::into).map_err(to_napi)
}

/// Converts a CBOR document back into its JSON encoding.
#[napi]
pub fn cbor_to_json_bytes(cbor: Buffer) -> napi::Result<Buffer> {
    cbor_to_json(cbor.as_ref()).map(Into::into).map_err(to_napi)
}

/// The largest whole number a JavaScript number holds exactly, `2^53 - 1`.
const SAFE_INTEGER: u64 = (1 << 53) - 1;

/// Delta-encodes a series of integer samples into a compact buffer.
///
/// A sample must be a whole number a JavaScript number holds exactly. Anything else is
/// refused rather than rounded, since packing is meant to lose nothing.
#[napi]
pub fn encode_delta_samples(samples: Vec<f64>) -> napi::Result<Buffer> {
    let mut whole = Vec::with_capacity(samples.len());
    for (index, sample) in samples.into_iter().enumerate() {
        if sample.fract() != 0.0 {
            return Err(napi::Error::from_reason(format!(
                "sample {index} is {sample}, which is not a whole number"
            )));
        }
        if sample.abs() > SAFE_INTEGER as f64 {
            return Err(napi::Error::from_reason(format!(
                "sample {index} is {sample}, past the largest whole number a JavaScript number holds exactly"
            )));
        }
        whole.push(sample as i64);
    }
    Ok(encode_deltas(&whole).into())
}

/// Decodes a delta-encoded buffer back into its integer samples.
///
/// A sample past what a JavaScript number holds exactly is refused rather than
/// rounded, since packing is meant to lose nothing.
#[napi]
pub fn decode_delta_samples(bytes: Buffer) -> napi::Result<Vec<i64>> {
    let samples = decode_deltas(bytes.as_ref()).map_err(to_napi)?;
    if let Some((index, sample)) = samples
        .iter()
        .enumerate()
        .find(|(_, sample)| sample.unsigned_abs() > SAFE_INTEGER)
    {
        return Err(napi::Error::from_reason(format!(
            "sample {index} is {sample}, past the largest whole number a JavaScript number holds exactly"
        )));
    }
    Ok(samples)
}

/// Packs float readings to a fixed precision for a metered link.
#[napi]
pub struct Quantizer {
    inner: Inner,
}

#[napi]
impl Quantizer {
    /// Creates a quantizer whose `scale` sets the precision kept.
    ///
    /// A scale of `100` keeps two decimal places. It must be positive and finite,
    /// and decoding must use the same scale the batch was encoded with.
    #[napi(constructor)]
    pub fn new(scale: f64) -> napi::Result<Self> {
        let narrowed = scale as f32;
        if !narrowed.is_finite() || narrowed <= 0.0 {
            return Err(napi::Error::from_reason(format!(
                "a quantizer's scale must be a positive, finite number, not {scale}"
            )));
        }
        Ok(Self {
            inner: Inner::new(narrowed),
        })
    }

    /// Quantizes and delta-encodes a batch of readings.
    ///
    /// A reading that is not a number, is infinite, or is too large for the scale is
    /// refused, since the format has no way to carry a missing reading.
    #[napi]
    pub fn encode(&self, readings: Vec<f64>) -> napi::Result<Buffer> {
        let readings: Vec<f32> = readings.into_iter().map(|value| value as f32).collect();
        self.inner
            .encode(&readings)
            .map(Into::into)
            .map_err(to_napi)
    }

    /// Decodes a batch back into readings, to within the quantizer's precision.
    #[napi]
    pub fn decode(&self, bytes: Buffer) -> napi::Result<Vec<f64>> {
        self.inner
            .decode(bytes.as_ref())
            .map(|readings| readings.into_iter().map(f64::from).collect())
            .map_err(to_napi)
    }
}

/// Maps a core error onto a rejected promise or thrown exception.
fn to_napi(error: pamoja_core::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
