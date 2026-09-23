//! Compact batch encoding for metered links: delta plus variable-length integers.
//!
//! On a long-range radio or a metered cellular link, every byte costs power or money,
//! so it pays to send a batch of readings in as few bytes as possible rather than one
//! full-width value at a time. The functions here encode a sequence of integers as a
//! starting value followed by the differences between consecutive values, each
//! written as a variable-length integer. A slowly changing signal - a temperature, a
//! tank level, a battery voltage - then costs about one byte per sample instead of
//! eight, with no loss. [`Quantizer`] extends this to `f32` readings by rounding each
//! to a fixed precision first.

use alloc::format;
use alloc::vec::Vec;

use pamoja_core::{Error, Result};

/// The magnitude past which a quantized reading no longer fits the `i64` it is packed
/// as, kept a little inside `i64::MAX` so the conversion never saturates.
const QUANTIZED_LIMIT: f32 = 9.2e18;

// Writes an unsigned integer as LEB128: seven bits per byte, high bit as a continue
// flag.
fn write_uvarint(mut value: u64, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

// Reads a LEB128 unsigned integer, advancing `pos`. The tenth byte holds bit 63 alone,
// so anything more in it is a value past 64 bits.
fn read_uvarint(bytes: &[u8], pos: &mut usize) -> Result<u64> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = *bytes
            .get(*pos)
            .ok_or_else(|| Error::Codec("the batch ends part-way through a value".into()))?;
        *pos += 1;
        if shift == 63 && byte > 1 {
            return Err(Error::Codec(
                "a value in the batch does not fit in 64 bits".into(),
            ));
        }
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
    }
}

// Rounds half away from zero, as `f32::round` does, with only `core`. Every `f32` of
// magnitude 2^23 or more is already whole, and below that the fraction is exact.
fn round(value: f32) -> f32 {
    const WHOLE: f32 = 8_388_608.0;
    if value.is_nan() || value <= -WHOLE || value >= WHOLE {
        return value;
    }
    let truncated = value as i32 as f32;
    let fraction = value - truncated;
    if fraction >= 0.5 {
        truncated + 1.0
    } else if fraction <= -0.5 {
        truncated - 1.0
    } else {
        truncated
    }
}

// Maps a signed integer to an unsigned one whose size grows with magnitude, so small
// negative deltas stay small (protobuf zigzag).
fn zigzag(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

fn unzigzag(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}

/// Encodes a batch of integer samples as a starting value plus variable-length deltas.
///
/// # Arguments
///
/// * `samples` - the integers to encode, in order.
///
/// # Returns
///
/// The compact encoding. A slowly changing series is far smaller than the eight bytes
/// per sample a raw encoding would use.
///
/// # Examples
///
/// ```
/// use pamoja_codec::{decode_deltas, encode_deltas};
///
/// let samples = [1000, 1001, 1003, 1002];
/// let bytes = encode_deltas(&samples);
/// assert!(bytes.len() < samples.len() * 8); // far smaller than eight bytes each
/// assert_eq!(decode_deltas(&bytes).unwrap(), samples);
/// ```
pub fn encode_deltas(samples: &[i64]) -> Vec<u8> {
    let mut out = Vec::new();
    write_uvarint(samples.len() as u64, &mut out);
    let mut previous = 0i64;
    for &sample in samples {
        let delta = sample.wrapping_sub(previous);
        write_uvarint(zigzag(delta), &mut out);
        previous = sample;
    }
    out
}

/// Decodes a batch encoded by [`encode_deltas`].
///
/// # Arguments
///
/// * `bytes` - the encoded batch.
///
/// # Returns
///
/// The decoded samples, in order.
///
/// # Errors
///
/// Returns [`Error::Codec`](pamoja_core::Error::Codec) if `bytes` ends part-way
/// through a value, holds a value past 64 bits, or carries bytes after the last
/// sample, which a batch cut short or run into another would.
pub fn decode_deltas(bytes: &[u8]) -> Result<Vec<i64>> {
    let mut pos = 0;
    let count = read_uvarint(bytes, &mut pos)?;
    // The count is untrusted, so the vector grows with the data actually present
    // rather than pre-allocating from a claimed length.
    let mut samples = Vec::new();
    let mut previous = 0i64;
    for _ in 0..count {
        let delta = unzigzag(read_uvarint(bytes, &mut pos)?);
        let sample = previous.wrapping_add(delta);
        samples.push(sample);
        previous = sample;
    }
    match bytes.len() - pos {
        0 => Ok(samples),
        extra => Err(Error::Codec(format!(
            "{extra} bytes follow the batch's last sample"
        ))),
    }
}

/// Packs a batch of `f32` readings into a compact byte form for a metered link.
///
/// A quantizer rounds each reading to a fixed precision - set by the `scale`, where
/// `100.0` keeps two decimal places - turns it into an integer, and delta-encodes the
/// batch with [`encode_deltas`]. This is lossy by exactly the rounding step, which is
/// the right trade for a cheap sensor on an expensive link: a temperature to the
/// nearest hundredth of a degree costs a byte or two per sample instead of four.
/// The same `scale` must be used to encode and decode, and nothing in the bytes
/// records it.
///
/// A reading that is not a number, or is infinite, is refused rather than packed:
/// the format has no way to say "no reading", and a dead sensor's `NaN` sent as a
/// real value would be worse than an error.
///
/// # Examples
///
/// ```
/// use pamoja_codec::Quantizer;
///
/// // Quantize to 0.1 precision and pack a slowly-rising series.
/// let quantizer = Quantizer::new(10.0);
/// let readings = [20.0, 20.1, 20.2, 20.3];
/// let packed = quantizer.encode(&readings).unwrap();
/// assert!(packed.len() < readings.len() * 4); // smaller than four bytes per reading
///
/// let restored = quantizer.decode(&packed).unwrap();
/// assert!((restored[2] - 20.2).abs() < 0.05);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Quantizer {
    scale: f32,
}

impl Quantizer {
    /// Creates a quantizer with the given precision scale.
    ///
    /// # Arguments
    ///
    /// * `scale` - the multiplier applied before rounding; `100.0` keeps two decimal
    ///   places. It must be a positive, finite number, which
    ///   [`encode`](Quantizer::encode) and [`decode`](Quantizer::decode) check.
    ///
    /// # Returns
    ///
    /// The quantizer.
    pub fn new(scale: f32) -> Self {
        Self { scale }
    }

    /// Quantizes and delta-encodes a batch of readings.
    ///
    /// # Arguments
    ///
    /// * `readings` - the readings to pack, in order.
    ///
    /// # Returns
    ///
    /// The compact encoding of the batch.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if the scale is not a
    /// positive, finite number, if a reading is not a number or is infinite, or if a
    /// reading at this scale is past what the batch can carry, about 9.2e18.
    pub fn encode(&self, readings: &[f32]) -> Result<Vec<u8>> {
        self.check_scale()?;
        let mut samples = Vec::with_capacity(readings.len());
        for (index, &reading) in readings.iter().enumerate() {
            if !reading.is_finite() {
                return Err(Error::Codec(format!(
                    "reading {index} is {reading}, which cannot be quantized"
                )));
            }
            let scaled = round(reading * self.scale);
            if !scaled.is_finite() || scaled <= -QUANTIZED_LIMIT || scaled >= QUANTIZED_LIMIT {
                return Err(Error::Codec(format!(
                    "reading {index} is {reading}, too large for a scale of {}",
                    self.scale
                )));
            }
            samples.push(scaled as i64);
        }
        Ok(encode_deltas(&samples))
    }

    /// Decodes a batch back into readings, to within the quantizer's precision.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the encoding produced by [`encode`](Quantizer::encode) with the
    ///   same scale.
    ///
    /// # Returns
    ///
    /// The decoded readings, in order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if the scale is not a
    /// positive, finite number, or `bytes` is malformed.
    pub fn decode(&self, bytes: &[u8]) -> Result<Vec<f32>> {
        self.check_scale()?;
        let samples = decode_deltas(bytes)?;
        Ok(samples
            .iter()
            .map(|&sample| sample as f32 / self.scale)
            .collect())
    }

    /// Refuses a scale that cannot round a reading to anything meaningful.
    fn check_scale(&self) -> Result<()> {
        if self.scale.is_finite() && self.scale > 0.0 {
            Ok(())
        } else {
            Err(Error::Codec(format!(
                "a quantizer's scale must be a positive, finite number, not {}",
                self.scale
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_integer_batch_round_trips() {
        for samples in [
            vec![],
            vec![0],
            vec![42],
            vec![-5, -4, -3, -2],
            vec![1000, 1001, 1003, 1002, 999],
            vec![i64::MIN, 0, i64::MAX],
        ] {
            let bytes = encode_deltas(&samples);
            assert_eq!(decode_deltas(&bytes).expect("decode"), samples);
        }
    }

    #[test]
    fn a_slow_series_is_far_smaller_than_raw() {
        let samples: Vec<i64> = (0..100).map(|i| 5000 + i).collect();
        let bytes = encode_deltas(&samples);
        // Each delta is one, so a sample costs about a byte instead of eight.
        assert!(bytes.len() < samples.len() * 2);
    }

    #[test]
    fn truncated_bytes_are_a_codec_error() {
        // Claims three samples but supplies none.
        match decode_deltas(&[3]) {
            Err(Error::Codec(reason)) => {
                assert_eq!(reason, "the batch ends part-way through a value");
            }
            other => panic!("a batch cut short decoded: {other:?}"),
        }
    }

    #[test]
    fn a_value_past_64_bits_is_refused_rather_than_truncated() {
        let mut largest = Vec::new();
        write_uvarint(u64::MAX, &mut largest);
        assert_eq!(largest.len(), 10);
        assert_eq!(largest[9], 0x01);
        let mut pos = 0;
        assert_eq!(
            read_uvarint(&largest, &mut pos).expect("u64::MAX"),
            u64::MAX
        );

        for tenth in [0x02, 0x7f, 0x81] {
            let mut bytes = largest.clone();
            bytes[9] = tenth;
            bytes.push(0x00);
            let mut pos = 0;
            match read_uvarint(&bytes, &mut pos) {
                Err(Error::Codec(reason)) => {
                    assert_eq!(reason, "a value in the batch does not fit in 64 bits");
                }
                other => panic!("a tenth byte of {tenth:#04x} read as {other:?}"),
            }
        }
    }

    #[test]
    fn rounding_matches_the_standard_library_at_its_edges() {
        for value in [
            0.0,
            -0.0,
            0.5,
            -0.5,
            1.5,
            -1.5,
            2.5,
            -2.5,
            0.499_999_97,
            -0.499_999_97,
            8_388_607.5,
            -8_388_607.5,
            8_388_608.0,
            16_777_217.0,
            f32::MAX,
            f32::MIN,
            f32::MIN_POSITIVE,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            assert_eq!(round(value), value.round(), "rounding {value}");
        }
        assert!(round(f32::NAN).is_nan());
    }

    proptest::proptest! {
        #[test]
        fn rounding_matches_the_standard_library(value in proptest::num::f32::ANY) {
            let ours = round(value);
            let theirs = value.round();
            proptest::prop_assert!(ours == theirs || (ours.is_nan() && theirs.is_nan()));
        }
    }

    #[test]
    fn varints_cover_the_boundaries() {
        for value in [0u64, 1, 127, 128, 16_383, 16_384, u64::MAX] {
            let mut out = Vec::new();
            write_uvarint(value, &mut out);
            let mut pos = 0;
            assert_eq!(read_uvarint(&out, &mut pos).expect("read"), value);
            assert_eq!(pos, out.len());
        }
    }

    #[test]
    fn varints_match_the_canonical_leb128_encodings() {
        let cases: [(u64, &[u8]); 4] = [
            (0, &[0x00]),
            (1, &[0x01]),
            (128, &[0x80, 0x01]),
            (300, &[0xAC, 0x02]),
        ];
        for (value, encoded) in cases {
            let mut out = Vec::new();
            write_uvarint(value, &mut out);
            assert_eq!(out, encoded, "varint of {value}");
        }
    }

    #[test]
    fn zigzag_matches_the_protobuf_mapping() {
        let cases: [(i64, u64); 6] = [
            (0, 0),
            (-1, 1),
            (1, 2),
            (-2, 3),
            (2, 4),
            (2_147_483_647, 4_294_967_294),
        ];
        for (signed, unsigned) in cases {
            assert_eq!(zigzag(signed), unsigned, "zigzag of {signed}");
            assert_eq!(unzigzag(unsigned), signed, "unzigzag of {unsigned}");
        }
    }

    #[test]
    fn a_quantizer_round_trips_within_its_precision() {
        let quantizer = Quantizer::new(100.0);
        let readings = [4.0, 4.62, 5.13, 4.77, 3.98];
        let packed = quantizer.encode(&readings).expect("encode");
        let restored = quantizer.decode(&packed).expect("decode");
        for (original, decoded) in readings.iter().zip(&restored) {
            assert!((original - decoded).abs() <= 0.005 + f32::EPSILON);
        }
    }

    #[test]
    fn a_reading_that_is_not_a_number_is_refused_rather_than_packed_as_zero() {
        let quantizer = Quantizer::new(100.0);
        for (readings, index) in [
            (vec![20.0, f32::NAN], 1),
            (vec![f32::INFINITY], 0),
            (vec![20.0, 20.1, f32::NEG_INFINITY], 2),
        ] {
            match quantizer.encode(&readings) {
                Err(Error::Codec(reason)) => {
                    assert!(
                        reason.starts_with(&format!("reading {index} is")),
                        "{reason}"
                    );
                }
                other => panic!("{readings:?} was packed: {other:?}"),
            }
        }
    }

    #[test]
    fn a_reading_too_large_for_its_scale_is_refused() {
        let quantizer = Quantizer::new(1e10);
        match quantizer.encode(&[1e10]) {
            Err(Error::Codec(reason)) => assert!(reason.contains("too large"), "{reason}"),
            other => panic!("an overflowing reading was packed: {other:?}"),
        }
        assert!(Quantizer::new(1e10).encode(&[1.0]).is_ok());
    }

    #[test]
    fn a_scale_that_is_not_a_positive_number_is_refused() {
        for scale in [0.0, -100.0, f32::NAN, f32::INFINITY] {
            let quantizer = Quantizer::new(scale);
            assert!(matches!(quantizer.encode(&[1.0]), Err(Error::Codec(_))));
            assert!(matches!(quantizer.decode(&[0]), Err(Error::Codec(_))));
        }
    }

    #[test]
    fn bytes_after_the_last_sample_are_refused() {
        let mut bytes = encode_deltas(&[1, 2, 3]);
        bytes.extend_from_slice(&encode_deltas(&[4]));
        match decode_deltas(&bytes) {
            Err(Error::Codec(reason)) => {
                assert_eq!(reason, "2 bytes follow the batch's last sample");
            }
            other => panic!("two batches run together decoded: {other:?}"),
        }
    }
}
