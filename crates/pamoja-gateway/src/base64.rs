//! The base64 encoding of RFC 4648, which the protocol carries every radio payload in.
//!
//! The packet forwarder protocol asks for padded base64 on the way up and allows it to be
//! omitted on the way down, and gateways in the field send both. [`encode`] pads, as the
//! protocol's own examples do for uplinks, and [`decode`] accepts a string with or without
//! padding, so a datagram from either side is read the same way.
//!
//! # Examples
//!
//! ```
//! use pamoja_gateway::base64;
//!
//! assert_eq!(base64::encode(b"foobar"), "Zm9vYmFy");
//! assert_eq!(base64::encode(b"fo"), "Zm8=");
//! assert_eq!(base64::decode("Zm8=").as_deref(), Ok(&b"fo"[..]));
//! assert_eq!(base64::decode("Zm8").as_deref(), Ok(&b"fo"[..]));
//! ```

use std::fmt;

/// The alphabet of RFC 4648 section 4, in the order the standard gives it.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The character that pads the last group to four.
const PAD: u8 = b'=';

/// Why a string is not base64.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// A character outside the alphabet, with its position.
    Character {
        /// Where it is in the string, counted in bytes.
        at: usize,
        /// The byte itself.
        byte: u8,
    },
    /// A group of one character, which no number of bytes encodes to.
    Length(usize),
    /// Padding before the end of the string, at this position.
    Padding(usize),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Character { at, byte } => {
                write!(f, "byte {at} is {byte:#04x}, which is not base64")
            }
            DecodeError::Length(len) => {
                write!(
                    f,
                    "{len} base64 characters leave a group of one, which encodes nothing"
                )
            }
            DecodeError::Padding(at) => write!(f, "padding at byte {at} is before the end"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Encodes bytes as padded base64.
///
/// # Arguments
///
/// * `bytes` - the bytes to encode.
///
/// # Returns
///
/// The base64 string, padded to a multiple of four characters.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let mut packed = 0u32;
        for (index, byte) in group.iter().enumerate() {
            packed |= u32::from(*byte) << (16 - 8 * index);
        }
        let characters = group.len() + 1;
        for index in 0..characters {
            let sextet = (packed >> (18 - 6 * index)) & 0x3F;
            out.push(char::from(ALPHABET[sextet as usize]));
        }
        for _ in characters..4 {
            out.push(char::from(PAD));
        }
    }
    out
}

/// Decodes base64, with or without its padding.
///
/// # Arguments
///
/// * `text` - the base64 string.
///
/// # Returns
///
/// The bytes it encodes.
///
/// # Errors
///
/// Returns [`DecodeError`] for a character outside the alphabet, padding before the end, or a
/// trailing group of one character.
pub fn decode(text: &str) -> Result<Vec<u8>, DecodeError> {
    let bytes = text.as_bytes();
    let body = match bytes.iter().position(|byte| *byte == PAD) {
        Some(at) => {
            if bytes[at..].iter().any(|byte| *byte != PAD) || bytes.len() - at > 2 {
                return Err(DecodeError::Padding(at));
            }
            &bytes[..at]
        }
        None => bytes,
    };
    if body.len() % 4 == 1 {
        return Err(DecodeError::Length(body.len()));
    }

    let mut out = Vec::with_capacity(body.len() / 4 * 3);
    for (group, characters) in body.chunks(4).enumerate() {
        let mut packed = 0u32;
        for (index, byte) in characters.iter().enumerate() {
            let sextet = sextet(*byte).ok_or(DecodeError::Character {
                at: group * 4 + index,
                byte: *byte,
            })?;
            packed |= u32::from(sextet) << (18 - 6 * index);
        }
        for index in 0..characters.len() - 1 {
            out.push(((packed >> (16 - 8 * index)) & 0xFF) as u8);
        }
    }
    Ok(out)
}

/// Returns the value of a base64 character.
fn sextet(byte: u8) -> Option<u8> {
    ALPHABET
        .iter()
        .position(|character| *character == byte)
        .map(|value| value as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test vectors of RFC 4648 section 10.
    const VECTORS: [(&str, &str); 7] = [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ];

    #[test]
    fn the_rfc_vectors_encode_and_decode() {
        for (plain, encoded) in VECTORS {
            assert_eq!(encode(plain.as_bytes()), encoded, "encoding {plain:?}");
            assert_eq!(
                decode(encoded).expect("the vector decodes"),
                plain.as_bytes(),
                "decoding {encoded:?}"
            );
        }
    }

    #[test]
    fn padding_is_optional_on_the_way_in() {
        for (plain, encoded) in VECTORS {
            let stripped = encoded.trim_end_matches('=');
            assert_eq!(
                decode(stripped).expect("an unpadded vector decodes"),
                plain.as_bytes(),
                "decoding {stripped:?}"
            );
        }
    }

    #[test]
    fn the_protocols_own_example_payload_decodes() {
        // From the PUSH_DATA example of the protocol, which sends its payload padded.
        let heard = decode("VEVTVF9QQUNLRVRfMTIzNA==").expect("the example decodes");
        assert_eq!(heard, b"TEST_PACKET_1234");
    }

    #[test]
    fn every_byte_survives_a_round_trip() {
        let all: Vec<u8> = (0..=255).collect();
        for len in 0..=all.len() {
            let bytes = &all[..len];
            let round = decode(&encode(bytes)).expect("what we encoded decodes");
            assert_eq!(round, bytes, "{len} bytes");
        }
    }

    #[test]
    fn what_is_not_base64_is_refused() {
        assert_eq!(
            decode("Zm9v!g=="),
            Err(DecodeError::Character { at: 4, byte: b'!' })
        );
        assert_eq!(decode("Zg=a"), Err(DecodeError::Padding(2)));
        assert_eq!(decode("Zm9vY"), Err(DecodeError::Length(5)));
        assert!(decode("Zm9vYg==").is_ok(), "a padded group is still good");
    }
}
