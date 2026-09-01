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

/// Delta-encodes a series of integer samples into a compact buffer.
#[napi]
pub fn encode_delta_samples(samples: Vec<i64>) -> Buffer {
    encode_deltas(&samples).into()
}

/// Decodes a delta-encoded buffer back into its integer samples.
#[napi]
pub fn decode_delta_samples(bytes: Buffer) -> napi::Result<Vec<i64>> {
    decode_deltas(bytes.as_ref()).map_err(to_napi)
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
        let scale = scale as f32;
        if !scale.is_finite() || scale <= 0.0 {
            return Err(napi::Error::from_reason(
                "scale must be positive and finite",
            ));
        }
        Ok(Self {
            inner: Inner::new(scale),
        })
    }

    /// Quantizes and delta-encodes a batch of readings.
    #[napi]
    pub fn encode(&self, readings: Vec<f64>) -> Buffer {
        let readings: Vec<f32> = readings.into_iter().map(|value| value as f32).collect();
        self.inner.encode(&readings).into()
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
