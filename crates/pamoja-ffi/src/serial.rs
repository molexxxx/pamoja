//! The C ABI for serial-line packet framing.
//!
//! These functions wrap [`pamoja_serial`] for callers that reach the SDK through
//! the flat C boundary. Both framings are here in full: the one-shot
//! encode and decode of a complete frame, and the streaming decoders that
//! reassemble frames from the arbitrary chunks a UART actually delivers.
//!
//! The Rust decoders take one byte at a time. Crossing the boundary per byte
//! would cost more than the decoding does, so what is exposed here is a
//! chunk-at-a-time `feed` that runs the same per-byte loop natively and hands
//! back every frame the chunk completed. A chunk that carries a corrupt frame
//! does not fail the call, because the frames around it are still good; the
//! decoder discards the corrupt one and counts it, and the count is readable
//! with the `*_discarded` calls.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use pamoja_serial::{cobs, slip, SerialError};

use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// The largest payload, in bytes, that a streaming decoder will reassemble.
///
/// The Rust decoders are generic over their capacity, which cannot cross a C
/// ABI, so the decoders here are built at one documented size. It covers the
/// 1500-byte maximum a serial link conventionally carries, with room to spare;
/// a frame longer than this is discarded rather than truncated. A caller who
/// needs a different bound has the Rust crate.
pub const PAMOJA_SERIAL_FRAME_MAX: usize = 2048;

/// An opaque handle to the frames one call to a streaming decoder completed.
///
/// Read it with [`pamoja_frames_count`], [`pamoja_frames_data`], and
/// [`pamoja_frames_len`], then release it with [`pamoja_frames_free`].
pub struct PamojaFrames {
    frames: Vec<Vec<u8>>,
}

/// An opaque handle to a streaming SLIP decoder.
pub struct PamojaSlipDecoder {
    inner: slip::SlipDecoder<PAMOJA_SERIAL_FRAME_MAX>,
    discarded: u64,
}

/// An opaque handle to a streaming COBS decoder.
pub struct PamojaCobsDecoder {
    inner: cobs::CobsDecoder<PAMOJA_SERIAL_FRAME_MAX>,
    discarded: u64,
}

/// Frames a payload as a SLIP packet (RFC 1055).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Safety
///
/// `payload` must point to at least `payload_len` readable bytes, or be null when
/// `payload_len` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_slip_encode(
    payload: *const u8,
    payload_len: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    frame(
        payload,
        payload_len,
        out_buffer,
        slip::max_encoded_len,
        slip::encode,
    )
}

/// Reads the payload back out of a SLIP frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or
/// [`PamojaStatus::Codec`] if the frame is corrupt.
///
/// # Safety
///
/// `frame` must point to at least `frame_len` readable bytes, or be null when
/// `frame_len` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_slip_decode(
    frame: *const u8,
    frame_len: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    unframe(frame, frame_len, out_buffer, slip::decode)
}

/// Frames a payload as a COBS packet, terminated by its zero delimiter.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Safety
///
/// `payload` must point to at least `payload_len` readable bytes, or be null when
/// `payload_len` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_cobs_encode(
    payload: *const u8,
    payload_len: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    frame(
        payload,
        payload_len,
        out_buffer,
        cobs::max_encoded_len,
        cobs::encode,
    )
}

/// Reads the payload back out of a COBS frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or
/// [`PamojaStatus::Codec`] if the frame is corrupt.
///
/// # Safety
///
/// `frame` must point to at least `frame_len` readable bytes, or be null when
/// `frame_len` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_cobs_decode(
    frame: *const u8,
    frame_len: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    unframe(frame, frame_len, out_buffer, cobs::decode)
}

/// Returns the largest SLIP frame a payload of `payload_len` bytes can produce.
///
/// # Returns
///
/// The worst-case encoded length, which is every byte escaped plus the delimiter.
#[no_mangle]
pub extern "C" fn pamoja_serial_slip_max_encoded_len(payload_len: usize) -> usize {
    slip::max_encoded_len(payload_len)
}

/// Returns the largest COBS frame a payload of `payload_len` bytes can produce.
///
/// # Returns
///
/// The worst-case encoded length, one overhead byte per 254 plus the delimiter.
#[no_mangle]
pub extern "C" fn pamoja_serial_cobs_max_encoded_len(payload_len: usize) -> usize {
    cobs::max_encoded_len(payload_len)
}

/// Creates a streaming SLIP decoder.
///
/// # Returns
///
/// A new decoder the caller must release with [`pamoja_slip_decoder_free`].
#[no_mangle]
pub extern "C" fn pamoja_slip_decoder_new() -> *mut PamojaSlipDecoder {
    Box::into_raw(Box::new(PamojaSlipDecoder {
        inner: slip::SlipDecoder::new(),
        discarded: 0,
    }))
}

/// Feeds a chunk of the byte stream to a SLIP decoder.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frames` set to a new handle
/// holding every frame this chunk completed, in order, which the caller must
/// release with [`pamoja_frames_free`]. A chunk that completes no frame yields
/// an empty handle rather than an error.
///
/// # Safety
///
/// `decoder` must be a live handle from [`pamoja_slip_decoder_new`], `bytes` must
/// point to at least `bytes_len` readable bytes or be null when `bytes_len` is 0,
/// and `out_frames` must point to a writable `*mut PamojaFrames`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_slip_decoder_feed(
    decoder: *mut PamojaSlipDecoder,
    bytes: *const u8,
    bytes_len: usize,
    out_frames: *mut *mut PamojaFrames,
) -> PamojaStatus {
    let out_frames = match out_slot(out_frames, "out_frames") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    if decoder.is_null() {
        set_last_error("decoder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let decoder = &mut *decoder;
    match catch_unwind(AssertUnwindSafe(|| {
        let mut frames = Vec::new();
        for &byte in &bytes {
            match decoder.inner.push(byte) {
                Ok(Some(frame)) => frames.push(frame.to_vec()),
                Ok(None) => {}
                Err(_) => decoder.discarded += 1,
            }
        }
        frames
    })) {
        Ok(frames) => {
            *out_frames = Box::into_raw(Box::new(PamojaFrames { frames }));
            PamojaStatus::Ok
        }
        Err(_) => panicked(),
    }
}

/// Returns how many corrupt frames a SLIP decoder has discarded.
///
/// # Returns
///
/// The running count, or 0 if `decoder` is null.
///
/// # Safety
///
/// `decoder` must be a live handle from [`pamoja_slip_decoder_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_slip_decoder_discarded(decoder: *const PamojaSlipDecoder) -> u64 {
    if decoder.is_null() {
        return 0;
    }
    (*decoder).discarded
}

/// Discards any partly assembled frame, returning a SLIP decoder to its initial state.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `decoder` must be a live handle from [`pamoja_slip_decoder_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_slip_decoder_reset(decoder: *mut PamojaSlipDecoder) {
    if !decoder.is_null() {
        (*decoder).inner.reset();
    }
}

/// Releases a SLIP decoder handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `decoder` must be a handle from [`pamoja_slip_decoder_new`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_slip_decoder_free(decoder: *mut PamojaSlipDecoder) {
    if !decoder.is_null() {
        drop(Box::from_raw(decoder));
    }
}

/// Creates a streaming COBS decoder.
///
/// # Returns
///
/// A new decoder the caller must release with [`pamoja_cobs_decoder_free`].
#[no_mangle]
pub extern "C" fn pamoja_cobs_decoder_new() -> *mut PamojaCobsDecoder {
    Box::into_raw(Box::new(PamojaCobsDecoder {
        inner: cobs::CobsDecoder::new(),
        discarded: 0,
    }))
}

/// Feeds a chunk of the byte stream to a COBS decoder.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frames` set to a new handle
/// holding every frame this chunk completed, in order, which the caller must
/// release with [`pamoja_frames_free`].
///
/// # Safety
///
/// `decoder` must be a live handle from [`pamoja_cobs_decoder_new`], `bytes` must
/// point to at least `bytes_len` readable bytes or be null when `bytes_len` is 0,
/// and `out_frames` must point to a writable `*mut PamojaFrames`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cobs_decoder_feed(
    decoder: *mut PamojaCobsDecoder,
    bytes: *const u8,
    bytes_len: usize,
    out_frames: *mut *mut PamojaFrames,
) -> PamojaStatus {
    let out_frames = match out_slot(out_frames, "out_frames") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    if decoder.is_null() {
        set_last_error("decoder must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let decoder = &mut *decoder;
    match catch_unwind(AssertUnwindSafe(|| {
        let mut frames = Vec::new();
        for &byte in &bytes {
            match decoder.inner.push(byte) {
                Ok(Some(frame)) => frames.push(frame.to_vec()),
                Ok(None) => {}
                Err(_) => decoder.discarded += 1,
            }
        }
        frames
    })) {
        Ok(frames) => {
            *out_frames = Box::into_raw(Box::new(PamojaFrames { frames }));
            PamojaStatus::Ok
        }
        Err(_) => panicked(),
    }
}

/// Returns how many corrupt frames a COBS decoder has discarded.
///
/// # Returns
///
/// The running count, or 0 if `decoder` is null.
///
/// # Safety
///
/// `decoder` must be a live handle from [`pamoja_cobs_decoder_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cobs_decoder_discarded(decoder: *const PamojaCobsDecoder) -> u64 {
    if decoder.is_null() {
        return 0;
    }
    (*decoder).discarded
}

/// Discards any partly assembled frame, returning a COBS decoder to its initial state.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `decoder` must be a live handle from [`pamoja_cobs_decoder_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cobs_decoder_reset(decoder: *mut PamojaCobsDecoder) {
    if !decoder.is_null() {
        (*decoder).inner.reset();
    }
}

/// Releases a COBS decoder handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `decoder` must be a handle from [`pamoja_cobs_decoder_new`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_cobs_decoder_free(decoder: *mut PamojaCobsDecoder) {
    if !decoder.is_null() {
        drop(Box::from_raw(decoder));
    }
}

/// Returns how many frames a decoder call produced.
///
/// # Returns
///
/// The count, or 0 if `frames` is null.
///
/// # Safety
///
/// `frames` must be a live handle from a decoder `feed` call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_frames_count(frames: *const PamojaFrames) -> usize {
    if frames.is_null() {
        return 0;
    }
    (*frames).frames.len()
}

/// Returns a pointer to one frame's payload bytes.
///
/// Use [`pamoja_frames_len`] for its length. The pointer is valid until the
/// handle is freed.
///
/// # Returns
///
/// A pointer to the payload, or null if `frames` is null or `index` is out of
/// range.
///
/// # Safety
///
/// `frames` must be a live handle from a decoder `feed` call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_frames_data(
    frames: *const PamojaFrames,
    index: usize,
) -> *const u8 {
    if frames.is_null() {
        return ptr::null();
    }
    let frames = &*frames;
    match frames.frames.get(index) {
        Some(frame) => frame.as_ptr(),
        None => ptr::null(),
    }
}

/// Returns the length in bytes of one frame's payload.
///
/// # Returns
///
/// The length, or 0 if `frames` is null or `index` is out of range.
///
/// # Safety
///
/// `frames` must be a live handle from a decoder `feed` call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_frames_len(frames: *const PamojaFrames, index: usize) -> usize {
    if frames.is_null() {
        return 0;
    }
    let frames = &*frames;
    frames.frames.get(index).map_or(0, Vec::len)
}

/// Releases a decoded frame set.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `frames` must be a handle from a decoder `feed` call that has not already been
/// freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_frames_free(frames: *mut PamojaFrames) {
    if !frames.is_null() {
        drop(Box::from_raw(frames));
    }
}

/// Runs one of the encoders into a buffer sized by its own worst case.
///
/// # Safety
///
/// `payload` must point to at least `payload_len` readable bytes, or be null when
/// `payload_len` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
unsafe fn frame(
    payload: *const u8,
    payload_len: usize,
    out_buffer: *mut *mut PamojaBuffer,
    bound: fn(usize) -> usize,
    encode: fn(&[u8], &mut [u8]) -> Result<usize, SerialError>,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| {
        let mut out = vec![0u8; bound(payload.len())];
        encode(&payload, &mut out).map(|written| {
            out.truncate(written);
            out
        })
    })) {
        Ok(Ok(bytes)) => {
            *out_buffer = PamojaBuffer::into_raw(bytes);
            PamojaStatus::Ok
        }
        Ok(Err(error)) => failed(error),
        Err(_) => panicked(),
    }
}

/// Runs one of the decoders into a buffer no smaller than the frame it reads.
///
/// A decoded payload is never longer than the frame that carried it, so the
/// frame's own length is a sound bound for the output.
///
/// # Safety
///
/// `frame` must point to at least `frame_len` readable bytes, or be null when
/// `frame_len` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
unsafe fn unframe(
    frame: *const u8,
    frame_len: usize,
    out_buffer: *mut *mut PamojaBuffer,
    decode: fn(&[u8], &mut [u8]) -> Result<usize, SerialError>,
) -> PamojaStatus {
    let out_buffer = match out_slot(out_buffer, "out_buffer") {
        Ok(slot) => slot,
        Err(status) => return status,
    };
    let frame = match read_bytes(frame, frame_len) {
        Ok(frame) => frame,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| {
        let mut out = vec![0u8; frame.len()];
        decode(&frame, &mut out).map(|written| {
            out.truncate(written);
            out
        })
    })) {
        Ok(Ok(bytes)) => {
            *out_buffer = PamojaBuffer::into_raw(bytes);
            PamojaStatus::Ok
        }
        Ok(Err(error)) => failed(error),
        Err(_) => panicked(),
    }
}

/// Rejects a null out-pointer and borrows the slot it names, cleared.
///
/// # Safety
///
/// `out` must be null or point to a writable `*mut T` that outlives the call.
unsafe fn out_slot<'a, T>(out: *mut *mut T, name: &str) -> Result<&'a mut *mut T, PamojaStatus> {
    if out.is_null() {
        set_last_error(format!("{name} must not be null"));
        return Err(PamojaStatus::InvalidArgument);
    }
    let slot = &mut *out;
    *slot = ptr::null_mut();
    Ok(slot)
}

/// Records a framing error and maps it onto its status.
fn failed(error: SerialError) -> PamojaStatus {
    set_last_error(error.to_string());
    match error {
        SerialError::BufferTooSmall => PamojaStatus::InvalidArgument,
        SerialError::InvalidEscape | SerialError::TruncatedFrame => PamojaStatus::Codec,
    }
}

/// Records a caught panic and reports it as [`PamojaStatus::Panic`].
fn panicked() -> PamojaStatus {
    set_last_error("panic at the FFI boundary".to_owned());
    PamojaStatus::Panic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};

    /// Copies a buffer handle's bytes out and releases the handle.
    ///
    /// # Safety
    ///
    /// `buffer` must be a live handle that has not already been freed.
    unsafe fn take(buffer: *mut PamojaBuffer) -> Vec<u8> {
        let bytes =
            std::slice::from_raw_parts(pamoja_buffer_data(buffer), pamoja_buffer_len(buffer))
                .to_vec();
        pamoja_buffer_free(buffer);
        bytes
    }

    /// Copies every frame out of a frame set and releases the handle.
    ///
    /// # Safety
    ///
    /// `frames` must be a live handle that has not already been freed.
    unsafe fn drain(frames: *mut PamojaFrames) -> Vec<Vec<u8>> {
        let collected = (0..pamoja_frames_count(frames))
            .map(|index| {
                std::slice::from_raw_parts(
                    pamoja_frames_data(frames, index),
                    pamoja_frames_len(frames, index),
                )
                .to_vec()
            })
            .collect();
        pamoja_frames_free(frames);
        collected
    }

    #[test]
    fn a_payload_round_trips_through_slip() {
        let payload = b"gps:37.42,-122.08\xc0\xdb";
        let mut framed = ptr::null_mut();
        let mut back = ptr::null_mut();

        // Safety: the inputs are valid slices and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_serial_slip_encode(payload.as_ptr(), payload.len(), &mut framed),
                PamojaStatus::Ok
            );
            let frame = take(framed);
            assert_eq!(
                pamoja_serial_slip_decode(frame.as_ptr(), frame.len(), &mut back),
                PamojaStatus::Ok
            );
            assert_eq!(take(back), payload);
        }
    }

    #[test]
    fn a_payload_round_trips_through_cobs() {
        let payload = b"\x11\x22\x00\x33";
        let mut framed = ptr::null_mut();
        let mut back = ptr::null_mut();

        // Safety: the inputs are valid slices and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_serial_cobs_encode(payload.as_ptr(), payload.len(), &mut framed),
                PamojaStatus::Ok
            );
            let frame = take(framed);
            assert_eq!(frame[frame.len() - 1], 0x00, "the frame delimiter");
            assert_eq!(
                pamoja_serial_cobs_decode(frame.as_ptr(), frame.len(), &mut back),
                PamojaStatus::Ok
            );
            assert_eq!(take(back), payload);
        }
    }

    #[test]
    fn a_corrupt_frame_reports_a_codec_status() {
        // An escape byte followed by neither escaped marker.
        let frame = [0xDBu8, 0x01, 0xC0];
        let mut out = ptr::null_mut();
        // Safety: the input is a valid slice and the out-pointer is writable.
        let status = unsafe { pamoja_serial_slip_decode(frame.as_ptr(), frame.len(), &mut out) };
        assert_eq!(status, PamojaStatus::Codec);
        assert!(out.is_null());
    }

    #[test]
    fn a_stream_split_across_chunks_still_yields_whole_frames() {
        let decoder = pamoja_slip_decoder_new();
        let stream = [b'o', b'k', 0xC0, b'g', b'o', 0xC0];
        let mut collected = Vec::new();

        // Safety: the decoder is live and each chunk is a valid slice.
        unsafe {
            for chunk in stream.chunks(2) {
                let mut frames = ptr::null_mut();
                assert_eq!(
                    pamoja_slip_decoder_feed(decoder, chunk.as_ptr(), chunk.len(), &mut frames),
                    PamojaStatus::Ok
                );
                collected.extend(drain(frames));
            }
            assert_eq!(pamoja_slip_decoder_discarded(decoder), 0);
            pamoja_slip_decoder_free(decoder);
        }

        assert_eq!(collected, vec![b"ok".to_vec(), b"go".to_vec()]);
    }

    #[test]
    fn a_corrupt_frame_mid_stream_is_counted_and_the_rest_survive() {
        let decoder = pamoja_slip_decoder_new();
        // A good frame, then an escape truncated by the delimiter, then a good frame.
        let stream = [b'o', b'k', 0xC0, 0xDB, 0xC0, b'g', b'o', 0xC0];
        let mut frames = ptr::null_mut();

        // Safety: the decoder is live, the input is a valid slice, and the
        // out-pointer is writable.
        let collected = unsafe {
            assert_eq!(
                pamoja_slip_decoder_feed(decoder, stream.as_ptr(), stream.len(), &mut frames),
                PamojaStatus::Ok
            );
            let collected = drain(frames);
            assert_eq!(pamoja_slip_decoder_discarded(decoder), 1);
            pamoja_slip_decoder_free(decoder);
            collected
        };

        assert_eq!(
            collected,
            vec![b"ok".to_vec(), b"go".to_vec()],
            "the frames either side of the corrupt one are still delivered"
        );
    }

    #[test]
    fn a_cobs_stream_reassembles_across_chunks() {
        let decoder = pamoja_cobs_decoder_new();
        let stream = [0x03, 0x11, 0x22, 0x02, 0x33, 0x00];
        let mut collected = Vec::new();

        // Safety: the decoder is live and each chunk is a valid slice.
        unsafe {
            for chunk in stream.chunks(4) {
                let mut frames = ptr::null_mut();
                assert_eq!(
                    pamoja_cobs_decoder_feed(decoder, chunk.as_ptr(), chunk.len(), &mut frames),
                    PamojaStatus::Ok
                );
                collected.extend(drain(frames));
            }
            pamoja_cobs_decoder_reset(decoder);
            pamoja_cobs_decoder_free(decoder);
        }

        assert_eq!(collected, vec![vec![0x11, 0x22, 0x00, 0x33]]);
    }

    #[test]
    fn a_null_decoder_is_rejected() {
        let mut frames = ptr::null_mut();
        // Safety: passing a null decoder is explicitly handled.
        let status =
            unsafe { pamoja_slip_decoder_feed(ptr::null_mut(), ptr::null(), 0, &mut frames) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
        assert!(frames.is_null());
    }

    #[test]
    fn the_worst_case_bounds_are_reported() {
        assert_eq!(
            pamoja_serial_slip_max_encoded_len(4),
            slip::max_encoded_len(4)
        );
        assert_eq!(
            pamoja_serial_cobs_max_encoded_len(4),
            cobs::max_encoded_len(4)
        );
    }
}
