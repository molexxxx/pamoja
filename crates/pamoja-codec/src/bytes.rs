//! A codec that carries raw bytes unchanged.

use alloc::vec::Vec;

use pamoja_core::Result;

use crate::Codec;

/// A no-op codec for payloads that are already byte buffers.
///
/// Encoding clones the buffer and decoding copies the input, so values pass
/// through unchanged. This is the "raw framing" option for payloads that carry
/// their own format, such as an image chunk or a pre-encoded frame.
///
/// # Examples
///
/// ```
/// use pamoja_codec::{BytesCodec, Codec};
///
/// let codec = BytesCodec;
/// let payload = vec![0xde, 0xad, 0xbe, 0xef];
/// let encoded = codec.encode(&payload).unwrap();
/// assert_eq!(codec.decode(&encoded).unwrap(), payload);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct BytesCodec;

impl Codec<Vec<u8>> for BytesCodec {
    fn encode(&self, value: &Vec<u8>) -> Result<Vec<u8>> {
        Ok(value.clone())
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_arbitrary_bytes() {
        let codec = BytesCodec;
        let payload = vec![1u8, 2, 3, 255, 0];
        let encoded = codec.encode(&payload).expect("encode");
        assert_eq!(encoded, payload);
        assert_eq!(codec.decode(&encoded).expect("decode"), payload);
    }

    #[test]
    fn round_trips_an_empty_buffer() {
        let codec = BytesCodec;
        let encoded = codec.encode(&Vec::new()).expect("encode");
        assert!(encoded.is_empty());
        assert!(codec.decode(&encoded).expect("decode").is_empty());
    }
}
