//! Generated Node bindings for serial-line packet framing.
//!
//! These mirror the `pamoja-serial` Rust API: SLIP and COBS, both as a one-shot
//! call over a complete frame and as a streaming decoder for the arbitrary chunks
//! a UART hands an application.
//!
//! The Rust decoders take one byte at a time. A JavaScript call per byte would
//! cost far more than the decoding does, so the decoders here take a chunk and
//! run the same per-byte loop natively. A corrupt frame inside a chunk does not
//! throw, because the frames around it are still good; it is discarded and
//! counted, and the count is readable from `discarded`.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_serial::{cobs, slip, SerialError};

/// The largest payload, in bytes, that a streaming decoder will reassemble.
///
/// The Rust decoders are generic over their capacity; these are built at one
/// documented size, covering the 1500 bytes a serial link conventionally carries
/// with room to spare.
pub const FRAME_MAX: usize = 2048;

/// Frames a payload as a SLIP packet (RFC 1055).
#[napi]
pub fn slip_encode(payload: Buffer) -> napi::Result<Buffer> {
    let mut out = vec![0u8; slip::max_encoded_len(payload.len())];
    let written = slip::encode(payload.as_ref(), &mut out).map_err(to_napi)?;
    out.truncate(written);
    Ok(out.into())
}

/// Reads the payload back out of a SLIP frame.
#[napi]
pub fn slip_decode(frame: Buffer) -> napi::Result<Buffer> {
    let mut out = vec![0u8; frame.len()];
    let written = slip::decode(frame.as_ref(), &mut out).map_err(to_napi)?;
    out.truncate(written);
    Ok(out.into())
}

/// Frames a payload as a COBS packet, terminated by its zero delimiter.
#[napi]
pub fn cobs_encode(payload: Buffer) -> napi::Result<Buffer> {
    let mut out = vec![0u8; cobs::max_encoded_len(payload.len())];
    let written = cobs::encode(payload.as_ref(), &mut out).map_err(to_napi)?;
    out.truncate(written);
    Ok(out.into())
}

/// Reads the payload back out of a COBS frame.
#[napi]
pub fn cobs_decode(frame: Buffer) -> napi::Result<Buffer> {
    let mut out = vec![0u8; frame.len()];
    let written = cobs::decode(frame.as_ref(), &mut out).map_err(to_napi)?;
    out.truncate(written);
    Ok(out.into())
}

/// Returns the largest SLIP frame a payload of this length can produce.
#[napi]
pub fn slip_max_encoded_len(payload_len: u32) -> u32 {
    slip::max_encoded_len(payload_len as usize) as u32
}

/// Returns the largest COBS frame a payload of this length can produce.
#[napi]
pub fn cobs_max_encoded_len(payload_len: u32) -> u32 {
    cobs::max_encoded_len(payload_len as usize) as u32
}

/// Reassembles whole SLIP frames from the chunks a serial port delivers.
#[napi]
pub struct SlipDecoder {
    inner: slip::SlipDecoder<FRAME_MAX>,
    discarded: u64,
}

#[napi]
impl SlipDecoder {
    /// Creates an empty decoder, ready for the first chunk.
    #[napi(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: slip::SlipDecoder::new(),
            discarded: 0,
        }
    }

    /// Feeds a chunk of the stream and returns every frame it completed.
    #[napi]
    pub fn feed(&mut self, chunk: Buffer) -> Vec<Buffer> {
        let mut frames = Vec::new();
        for &byte in chunk.as_ref() {
            match self.inner.push(byte) {
                Ok(Some(frame)) => frames.push(frame.to_vec().into()),
                Ok(None) => {}
                Err(_) => self.discarded += 1,
            }
        }
        frames
    }

    /// Returns how many corrupt frames this decoder has discarded.
    #[napi(getter)]
    pub fn discarded(&self) -> f64 {
        self.discarded as f64
    }

    /// Discards any partly assembled frame.
    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Reassembles whole COBS frames from the chunks a serial port delivers.
#[napi]
pub struct CobsDecoder {
    inner: cobs::CobsDecoder<FRAME_MAX>,
    discarded: u64,
}

#[napi]
impl CobsDecoder {
    /// Creates an empty decoder, ready for the first chunk.
    #[napi(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: cobs::CobsDecoder::new(),
            discarded: 0,
        }
    }

    /// Feeds a chunk of the stream and returns every frame it completed.
    #[napi]
    pub fn feed(&mut self, chunk: Buffer) -> Vec<Buffer> {
        let mut frames = Vec::new();
        for &byte in chunk.as_ref() {
            match self.inner.push(byte) {
                Ok(Some(frame)) => frames.push(frame.to_vec().into()),
                Ok(None) => {}
                Err(_) => self.discarded += 1,
            }
        }
        frames
    }

    /// Returns how many corrupt frames this decoder has discarded.
    #[napi(getter)]
    pub fn discarded(&self) -> f64 {
        self.discarded as f64
    }

    /// Discards any partly assembled frame.
    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Maps a framing error onto a thrown exception.
fn to_napi(error: SerialError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
