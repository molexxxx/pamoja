//! The C ABI for wire formats and metered-link packing.
//!
//! These functions wrap [`pamoja_codec`] for callers that reach the SDK through
//! the flat C boundary. The [`Codec`](pamoja_codec::Codec) trait is generic over
//! the value being carried and so cannot cross a C ABI; what crosses instead is
//! the concrete work a caller with an untyped payload actually needs. Converting
//! a document between JSON and CBOR, and packing a batch of readings small enough
//! for a metered link.
//!
//! Encoded output is bytes and comes back as a [`PamojaBuffer`]. Decoded output is
//! a typed series, so it comes back as [`PamojaSamples`] (`int64`) or
//! [`PamojaReadings`] (`float`) rather than bytes the caller would have to
//! reinterpret.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};
use pamoja_core::Result as CoreResult;

use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// An opaque handle to a decoded series of integer samples.
///
/// Read it with [`pamoja_samples_data`] and [`pamoja_samples_len`], then release
/// it with [`pamoja_samples_free`].
pub struct PamojaSamples {
    samples: Vec<i64>,
}

/// An opaque handle to a decoded series of float readings.
///
/// Read it with [`pamoja_readings_data`] and [`pamoja_readings_len`], then
/// release it with [`pamoja_readings_free`].
pub struct PamojaReadings {
    readings: Vec<f32>,
}

/// Converts a JSON document into its CBOR encoding.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or an error status whose
/// message is available from
/// [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `json` must point to at least `json_len` readable bytes, or be null when
/// `json_len` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_codec_json_to_cbor(
    json: *const u8,
    json_len: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    transcode(json, json_len, out_buffer, json_to_cbor)
}

/// Converts a CBOR document into its JSON encoding.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or an error status.
///
/// # Safety
///
/// `cbor` must point to at least `cbor_len` readable bytes, or be null when
/// `cbor_len` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_codec_cbor_to_json(
    cbor: *const u8,
    cbor_len: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    transcode(cbor, cbor_len, out_buffer, cbor_to_json)
}

/// Delta-encodes a series of integer samples into a compact buffer.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Safety
///
/// `samples` must point to at least `count` readable `int64` values, or be null
/// when `count` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_codec_encode_deltas(
    samples: *const i64,
    count: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if let Err(status) = clear_out(out_buffer, "out_buffer") {
        return status;
    }
    let samples = match read_slice(samples, count, "samples") {
        Ok(samples) => samples,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| encode_deltas(&samples))) {
        Ok(bytes) => {
            *out_buffer = PamojaBuffer::into_raw(bytes);
            PamojaStatus::Ok
        }
        Err(_) => panicked(),
    }
}

/// Decodes a delta-encoded buffer back into its integer samples.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_samples` set to a new handle the
/// caller must release with [`pamoja_samples_free`], or
/// [`PamojaStatus::Codec`] if the buffer is malformed.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes, or be null when
/// `bytes_len` is 0, and `out_samples` must point to a writable
/// `*mut PamojaSamples`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_codec_decode_deltas(
    bytes: *const u8,
    bytes_len: usize,
    out_samples: *mut *mut PamojaSamples,
) -> PamojaStatus {
    if let Err(status) = clear_out(out_samples, "out_samples") {
        return status;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| decode_deltas(&bytes))) {
        Ok(Ok(samples)) => {
            *out_samples = Box::into_raw(Box::new(PamojaSamples { samples }));
            PamojaStatus::Ok
        }
        Ok(Err(error)) => failed(&error),
        Err(_) => panicked(),
    }
}

/// Quantizes and delta-encodes a batch of float readings.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_buffer` set to a new buffer handle
/// the caller must release with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free), or
/// [`PamojaStatus::InvalidArgument`] if `scale` is not positive and finite.
///
/// # Safety
///
/// `readings` must point to at least `count` readable `float` values, or be null
/// when `count` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_codec_quantizer_encode(
    scale: f32,
    readings: *const f32,
    count: usize,
    out_buffer: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if let Err(status) = clear_out(out_buffer, "out_buffer") {
        return status;
    }
    if let Err(status) = check_scale(scale) {
        return status;
    }
    let readings = match read_slice(readings, count, "readings") {
        Ok(readings) => readings,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| Quantizer::new(scale).encode(&readings))) {
        Ok(bytes) => {
            *out_buffer = PamojaBuffer::into_raw(bytes);
            PamojaStatus::Ok
        }
        Err(_) => panicked(),
    }
}

/// Decodes a quantized batch back into float readings.
///
/// The readings come back to within the precision `scale` selected, which must be
/// the same scale the batch was encoded with.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_readings` set to a new handle the
/// caller must release with [`pamoja_readings_free`], or
/// [`PamojaStatus::Codec`] if the buffer is malformed.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes, or be null when
/// `bytes_len` is 0, and `out_readings` must point to a writable
/// `*mut PamojaReadings`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_codec_quantizer_decode(
    scale: f32,
    bytes: *const u8,
    bytes_len: usize,
    out_readings: *mut *mut PamojaReadings,
) -> PamojaStatus {
    if let Err(status) = clear_out(out_readings, "out_readings") {
        return status;
    }
    if let Err(status) = check_scale(scale) {
        return status;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| Quantizer::new(scale).decode(&bytes))) {
        Ok(Ok(readings)) => {
            *out_readings = Box::into_raw(Box::new(PamojaReadings { readings }));
            PamojaStatus::Ok
        }
        Ok(Err(error)) => failed(&error),
        Err(_) => panicked(),
    }
}

/// Returns a pointer to a decoded series of integer samples.
///
/// Use [`pamoja_samples_len`] for the count. The pointer is valid until the
/// handle is freed.
///
/// # Returns
///
/// A pointer to the samples, or null if `samples` is null.
///
/// # Safety
///
/// `samples` must be a live handle from [`pamoja_codec_decode_deltas`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_samples_data(samples: *const PamojaSamples) -> *const i64 {
    if samples.is_null() {
        return ptr::null();
    }
    (*samples).samples.as_ptr()
}

/// Returns the number of integer samples in a decoded series.
///
/// # Returns
///
/// The count, or 0 if `samples` is null.
///
/// # Safety
///
/// `samples` must be a live handle from [`pamoja_codec_decode_deltas`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_samples_len(samples: *const PamojaSamples) -> usize {
    if samples.is_null() {
        return 0;
    }
    (*samples).samples.len()
}

/// Releases a decoded sample series.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `samples` must be a handle from [`pamoja_codec_decode_deltas`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_samples_free(samples: *mut PamojaSamples) {
    if !samples.is_null() {
        drop(Box::from_raw(samples));
    }
}

/// Returns a pointer to a decoded series of float readings.
///
/// Use [`pamoja_readings_len`] for the count. The pointer is valid until the
/// handle is freed.
///
/// # Returns
///
/// A pointer to the readings, or null if `readings` is null.
///
/// # Safety
///
/// `readings` must be a live handle from [`pamoja_codec_quantizer_decode`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_readings_data(readings: *const PamojaReadings) -> *const f32 {
    if readings.is_null() {
        return ptr::null();
    }
    (*readings).readings.as_ptr()
}

/// Returns the number of float readings in a decoded series.
///
/// # Returns
///
/// The count, or 0 if `readings` is null.
///
/// # Safety
///
/// `readings` must be a live handle from [`pamoja_codec_quantizer_decode`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_readings_len(readings: *const PamojaReadings) -> usize {
    if readings.is_null() {
        return 0;
    }
    (*readings).readings.len()
}

/// Releases a decoded reading series.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `readings` must be a handle from [`pamoja_codec_quantizer_decode`] that has
/// not already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_readings_free(readings: *mut PamojaReadings) {
    if !readings.is_null() {
        drop(Box::from_raw(readings));
    }
}

/// Runs one of the document conversions, wiring input and output to the boundary.
///
/// # Safety
///
/// `input` must point to at least `input_len` readable bytes, or be null when
/// `input_len` is 0, and `out_buffer` must point to a writable
/// `*mut PamojaBuffer`.
unsafe fn transcode(
    input: *const u8,
    input_len: usize,
    out_buffer: *mut *mut PamojaBuffer,
    convert: fn(&[u8]) -> CoreResult<Vec<u8>>,
) -> PamojaStatus {
    if let Err(status) = clear_out(out_buffer, "out_buffer") {
        return status;
    }
    let input = match read_bytes(input, input_len) {
        Ok(input) => input,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| convert(&input))) {
        Ok(Ok(bytes)) => {
            *out_buffer = PamojaBuffer::into_raw(bytes);
            PamojaStatus::Ok
        }
        Ok(Err(error)) => failed(&error),
        Err(_) => panicked(),
    }
}

/// Rejects a null out-pointer, and otherwise clears it before the call proceeds.
///
/// Clearing first means a caller that ignores the status never reads a stale
/// handle out of its own variable.
///
/// # Safety
///
/// `out` must be null or point to a writable `*mut T`.
unsafe fn clear_out<T>(out: *mut *mut T, name: &str) -> Result<(), PamojaStatus> {
    if out.is_null() {
        set_last_error(format!("{name} must not be null"));
        return Err(PamojaStatus::InvalidArgument);
    }
    *out = ptr::null_mut();
    Ok(())
}

/// Copies a borrowed array of `count` values, treating a zero count as empty.
///
/// # Safety
///
/// When `count` is non-zero, `ptr` must point to at least `count` readable `T`
/// values.
unsafe fn read_slice<T: Copy>(
    ptr: *const T,
    count: usize,
    name: &str,
) -> Result<Vec<T>, PamojaStatus> {
    if count == 0 {
        Ok(Vec::new())
    } else if ptr.is_null() {
        set_last_error(format!(
            "{name} must not be null when its count is non-zero"
        ));
        Err(PamojaStatus::InvalidArgument)
    } else {
        Ok(std::slice::from_raw_parts(ptr, count).to_vec())
    }
}

/// Rejects a scale that would make quantizing meaningless or produce infinities.
fn check_scale(scale: f32) -> Result<(), PamojaStatus> {
    if scale.is_finite() && scale > 0.0 {
        Ok(())
    } else {
        set_last_error("scale must be positive and finite".to_owned());
        Err(PamojaStatus::InvalidArgument)
    }
}

/// Records a core error and maps it onto its status.
fn failed(error: &pamoja_core::Error) -> PamojaStatus {
    set_last_error(error.to_string());
    PamojaStatus::from_error(error)
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

    #[test]
    fn a_document_round_trips_through_cbor() {
        let json = br#"{"c":21.5}"#;
        let mut cbor = ptr::null_mut();
        let mut back = ptr::null_mut();

        // Safety: the inputs are valid slices and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_codec_json_to_cbor(json.as_ptr(), json.len(), &mut cbor),
                PamojaStatus::Ok
            );
            let cbor_bytes = take(cbor);
            assert!(cbor_bytes.len() < json.len());
            assert_eq!(
                pamoja_codec_cbor_to_json(cbor_bytes.as_ptr(), cbor_bytes.len(), &mut back),
                PamojaStatus::Ok
            );
            assert_eq!(take(back), json);
        }
    }

    #[test]
    fn invalid_json_reports_a_codec_status() {
        let json = b"not json";
        let mut out = ptr::null_mut();
        // Safety: the input is a valid slice and the out-pointer is writable.
        let status = unsafe { pamoja_codec_json_to_cbor(json.as_ptr(), json.len(), &mut out) };
        assert_eq!(status, PamojaStatus::Codec);
        assert!(out.is_null());
    }

    #[test]
    fn samples_round_trip_through_delta_encoding() {
        let samples = [10i64, 11, 13, 12, 900];
        let mut encoded = ptr::null_mut();
        let mut decoded = ptr::null_mut();

        // Safety: the inputs are valid slices and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_codec_encode_deltas(samples.as_ptr(), samples.len(), &mut encoded),
                PamojaStatus::Ok
            );
            let bytes = take(encoded);
            assert_eq!(
                pamoja_codec_decode_deltas(bytes.as_ptr(), bytes.len(), &mut decoded),
                PamojaStatus::Ok
            );
            let restored = std::slice::from_raw_parts(
                pamoja_samples_data(decoded),
                pamoja_samples_len(decoded),
            )
            .to_vec();
            pamoja_samples_free(decoded);
            assert_eq!(restored, samples);
        }
    }

    #[test]
    fn readings_round_trip_to_within_the_quantizer_precision() {
        let readings = [20.0f32, 20.1, 20.2, 20.3];
        let mut encoded = ptr::null_mut();
        let mut decoded = ptr::null_mut();

        // Safety: the inputs are valid slices and the out-pointers are writable.
        unsafe {
            assert_eq!(
                pamoja_codec_quantizer_encode(
                    100.0,
                    readings.as_ptr(),
                    readings.len(),
                    &mut encoded
                ),
                PamojaStatus::Ok
            );
            let bytes = take(encoded);
            assert!(bytes.len() < readings.len() * 4);
            assert_eq!(
                pamoja_codec_quantizer_decode(100.0, bytes.as_ptr(), bytes.len(), &mut decoded),
                PamojaStatus::Ok
            );
            let restored = std::slice::from_raw_parts(
                pamoja_readings_data(decoded),
                pamoja_readings_len(decoded),
            )
            .to_vec();
            pamoja_readings_free(decoded);
            for (got, want) in restored.iter().zip(readings.iter()) {
                assert!((got - want).abs() < 0.05);
            }
        }
    }

    #[test]
    fn a_non_positive_scale_is_rejected() {
        let readings = [1.0f32];
        let mut out = ptr::null_mut();
        // Safety: the input is a valid slice and the out-pointer is writable.
        let status = unsafe {
            pamoja_codec_quantizer_encode(0.0, readings.as_ptr(), readings.len(), &mut out)
        };
        assert_eq!(status, PamojaStatus::InvalidArgument);
        assert!(out.is_null());
    }

    #[test]
    fn a_null_out_pointer_is_rejected() {
        let json = br#"{}"#;
        // Safety: passing a null out-pointer is explicitly handled.
        let status =
            unsafe { pamoja_codec_json_to_cbor(json.as_ptr(), json.len(), ptr::null_mut()) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
    }

    #[test]
    fn an_empty_series_encodes_and_decodes_as_empty() {
        let mut encoded = ptr::null_mut();
        let mut decoded = ptr::null_mut();
        // Safety: a null data pointer is allowed when the count is zero.
        unsafe {
            assert_eq!(
                pamoja_codec_encode_deltas(ptr::null(), 0, &mut encoded),
                PamojaStatus::Ok
            );
            let bytes = take(encoded);
            assert_eq!(
                pamoja_codec_decode_deltas(bytes.as_ptr(), bytes.len(), &mut decoded),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_samples_len(decoded), 0);
            pamoja_samples_free(decoded);
        }
    }
}
