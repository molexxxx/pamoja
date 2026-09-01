//! Conversion between the JSON and CBOR wire formats.
//!
//! [`Codec`](crate::Codec) is generic over the value being carried, which suits
//! Rust callers that have a typed payload. Callers arriving through a language
//! binding do not: they hold a document their own runtime already speaks as JSON,
//! and what they need from this crate is the compact form to put on a metered
//! link. These two functions are that operation, transcoding a whole document
//! between the two formats without a Rust type for it.

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
}
