//! Conversion between the JSON and CBOR wire formats.
//!
//! [`Codec`](crate::Codec) is generic over the value being carried, which suits
//! Rust callers that have a typed payload. Callers arriving through a language
//! binding do not: they hold a document their own runtime already speaks as JSON,
//! and what they need from this crate is the compact form to put on a metered
//! link. These two functions are that operation, transcoding a whole document
//! between the two formats without a Rust type for it.
//!
//! Object keys come back in sorted order rather than the order they were written
//! in, because the intermediate value holds them in a sorted map. That makes the
//! output canonical, which is what a signed or deduplicated payload wants, but it
//! means a round trip is faithful to the document's content and not to its
//! original byte layout.

use pamoja_core::{Error, Result};

/// Converts a JSON document into its CBOR encoding.
///
/// # Arguments
///
/// * `json` - the UTF-8 JSON document to convert.
///
/// # Returns
///
/// The CBOR encoding of the same document, which is typically a good deal
/// smaller and is what a constrained device or metered link should carry.
///
/// # Errors
///
/// Returns [`Error::Codec`] if `json` is not a valid JSON document, or if the
/// document cannot be written as CBOR.
///
/// # Examples
///
/// ```
/// use pamoja_codec::json_to_cbor;
///
/// let cbor = json_to_cbor(br#"{"c":21.5}"#).unwrap();
/// assert!(cbor.len() < br#"{"c":21.5}"#.len());
/// ```
pub fn json_to_cbor(json: &[u8]) -> Result<Vec<u8>> {
    let value: serde_json::Value =
        serde_json::from_slice(json).map_err(|error| Error::Codec(error.to_string()))?;
    let mut buffer = Vec::new();
    ciborium::into_writer(&value, &mut buffer).map_err(|error| Error::Codec(error.to_string()))?;
    Ok(buffer)
}

/// Converts a CBOR document into its JSON encoding.
///
/// # Arguments
///
/// * `cbor` - the CBOR document to convert.
///
/// # Returns
///
/// The UTF-8 JSON encoding of the same document, suitable for handing back to a
/// runtime that reads JSON natively.
///
/// # Errors
///
/// Returns [`Error::Codec`] if `cbor` is not a valid CBOR document, or if it
/// holds a construct with no JSON equivalent, such as a non-string map key.
///
/// # Examples
///
/// ```
/// use pamoja_codec::{cbor_to_json, json_to_cbor};
///
/// let cbor = json_to_cbor(br#"{"c":21.5}"#).unwrap();
/// assert_eq!(cbor_to_json(&cbor).unwrap(), br#"{"c":21.5}"#);
/// ```
pub fn cbor_to_json(cbor: &[u8]) -> Result<Vec<u8>> {
    let value: ciborium::Value =
        ciborium::from_reader(cbor).map_err(|error| Error::Codec(error.to_string()))?;
    let value: serde_json::Value = value
        .deserialized()
        .map_err(|error| Error::Codec(error.to_string()))?;
    serde_json::to_vec(&value).map_err(|error| Error::Codec(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_keys_come_back_sorted() {
        // The intermediate value holds keys in a sorted map, so the output is
        // canonical rather than a replay of the input's byte order.
        let cbor = json_to_cbor(br#"{"c":21.5,"a":1}"#).expect("to cbor");
        assert_eq!(
            cbor_to_json(&cbor).expect("to json"),
            br#"{"a":1,"c":21.5}"#
        );
    }

    #[test]
    fn round_trips_a_document() {
        let json = br#"{"battery":88,"id":"probe-1","reading":21.5}"#;
        let cbor = json_to_cbor(json).expect("to cbor");
        assert_eq!(cbor_to_json(&cbor).expect("to json"), json);
    }

    #[test]
    fn cbor_is_smaller_than_the_json_it_came_from() {
        let json = br#"{"a":1,"b":2,"c":3,"d":4,"e":5}"#;
        let cbor = json_to_cbor(json).expect("to cbor");
        assert!(cbor.len() < json.len());
    }

    #[test]
    fn round_trips_nested_and_empty_containers() {
        let json = br#"{"empty":{},"list":[1,[2,3],{"deep":true}],"none":null}"#;
        let cbor = json_to_cbor(json).expect("to cbor");
        assert_eq!(cbor_to_json(&cbor).expect("to json"), json);
    }

    #[test]
    fn invalid_json_is_a_codec_error() {
        assert!(matches!(json_to_cbor(b"not json"), Err(Error::Codec(_))));
    }

    #[test]
    fn invalid_cbor_is_a_codec_error() {
        assert!(matches!(cbor_to_json(&[0xff, 0xff]), Err(Error::Codec(_))));
    }

    #[test]
    fn a_non_string_map_key_has_no_json_form() {
        // CBOR allows an integer map key; JSON does not, so the conversion fails
        // rather than inventing a key.
        let mut cbor = Vec::new();
        let value = ciborium::Value::Map(vec![(
            ciborium::Value::Integer(1.into()),
            ciborium::Value::Bool(true),
        )]);
        ciborium::into_writer(&value, &mut cbor).expect("write cbor");
        assert!(matches!(cbor_to_json(&cbor), Err(Error::Codec(_))));
    }

    #[test]
    fn a_document_transcodes_to_the_bytes_rfc_8949_fixes() {
        // RFC 8949 encodes this document as a two-entry map with text keys, and 21.5 in
        // the shortest form it allows, which is a half-precision float. Pinning the bytes
        // catches an encoder that is wrong but self-consistent.
        let json = br#"{"c":21.5,"ok":true}"#;
        let cbor = json_to_cbor(json).expect("a valid document");
        assert_eq!(
            cbor,
            [0xA2, 0x61, 0x63, 0xF9, 0x4D, 0x60, 0x62, 0x6F, 0x6B, 0xF5]
        );
        assert_eq!(cbor_to_json(&cbor).expect("a valid document"), json);
    }
}
