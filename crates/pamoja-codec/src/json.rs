//! A human-readable JSON codec built on [`serde_json`].

use alloc::string::ToString;
use alloc::vec::Vec;

use serde::de::DeserializeOwned;
use serde::Serialize;

use pamoja_core::{Error, Result};

use crate::Codec;

/// A codec that serializes values to and from JSON.
///
/// JSON trades the compactness of CBOR for human readability, which makes it a
/// good fit for debugging, configuration, and interop with services that speak
/// JSON. The codec works for any type that implements [`serde::Serialize`] and
/// [`serde::de::DeserializeOwned`].
///
/// # Examples
///
/// ```
/// use pamoja_codec::{Codec, JsonCodec};
///
/// let codec = JsonCodec;
/// let encoded = codec.encode(&vec![1, 2, 3]).unwrap();
/// assert_eq!(encoded, b"[1,2,3]");
/// let decoded: Vec<i32> = codec.decode(&encoded).unwrap();
/// assert_eq!(decoded, vec![1, 2, 3]);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct JsonCodec;

impl<T> Codec<T> for JsonCodec
where
    T: Serialize + DeserializeOwned,
{
    fn encode(&self, value: &T) -> Result<Vec<u8>> {
        serde_json::to_vec(value).map_err(|error| Error::Codec(error.to_string()))
    }

    fn decode(&self, bytes: &[u8]) -> Result<T> {
        serde_json::from_slice(bytes).map_err(|error| Error::Codec(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_map() {
        use std::collections::BTreeMap;

        let codec = JsonCodec;
        let mut value = BTreeMap::new();
        value.insert("sensor".to_owned(), 21i64);
        let encoded = codec.encode(&value).expect("encode");
        let decoded: BTreeMap<String, i64> = codec.decode(&encoded).expect("decode");
        assert_eq!(decoded, value);
    }

    #[test]
    fn decoding_invalid_json_is_a_codec_error() {
        let codec = JsonCodec;
        let result: Result<Vec<i32>> = codec.decode(b"not json");
        assert!(matches!(result, Err(Error::Codec(_))));
    }
}
