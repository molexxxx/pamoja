//! The manifest: what an update claims about itself, and who vouches for it.
//!
//! The fields are the information elements RFC 9124 marks REQUIRED for a
//! single-payload update, named the way the RFC names them. Nothing here is
//! decorative: each one exists because leaving it out enables a specific attack,
//! and each is checked before an image is written.
//!
//! An [`Envelope`] carries the encoded manifest as an opaque byte string next to
//! the signature over exactly those bytes. Keeping the signed body intact rather
//! than re-encoding it after parsing means the bytes that were verified are the
//! bytes that are read, so no encoding difference can open a gap between them.

use pamoja_security::{DeviceIdentity, PublicIdentity, Signature};

use crate::cbor::{Reader, Writer};
use crate::error::{Refusal, Result};

/// The manifest structure version this crate writes and understands.
pub const STRUCTURE_VERSION: u8 = 1;

/// The length of a vendor or device class identifier, in bytes.
pub const ID_LEN: usize = 16;

/// The length of a payload digest, in bytes.
pub const DIGEST_LEN: usize = 32;

/// A buffer of this size always holds an encoded manifest body.
pub const MANIFEST_MAX: usize = 128;

/// A buffer of this size always holds an encoded envelope.
pub const ENVELOPE_MAX: usize = 224;

/// The length of a signature, in bytes.
const SIGNATURE_LEN: usize = 64;

// Map keys, ascending, so the encoding satisfies the deterministic ordering rule.
const KEY_STRUCTURE_VERSION: u64 = 1;
const KEY_SEQUENCE: u64 = 2;
const KEY_VENDOR: u64 = 3;
const KEY_CLASS: u64 = 4;
const KEY_FORMAT: u64 = 5;
const KEY_STORAGE: u64 = 6;
const KEY_DIGEST: u64 = 7;
const KEY_SIZE: u64 = 8;
const KEY_EXPIRES: u64 = 9;

/// Envelope keys.
const KEY_BODY: u64 = 1;
const KEY_SIGNATURE: u64 = 2;

/// How the payload is encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadFormat {
    /// The payload is the firmware image itself, byte for byte.
    Raw = 1,
}

impl PayloadFormat {
    /// Reads a payload format from its encoded value.
    ///
    /// # Arguments
    ///
    /// * `value` - the encoded discriminant.
    ///
    /// # Returns
    ///
    /// The format.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::UnsupportedVersion`] for a format this build cannot
    /// apply, so an unknown encoding is refused rather than guessed at.
    fn from_value(value: u64) -> Result<Self> {
        match value {
            1 => Ok(Self::Raw),
            _ => Err(Refusal::UnsupportedVersion),
        }
    }
}

/// What an update claims about itself.
///
/// # Examples
///
/// ```
/// use pamoja_security::DeviceIdentity;
/// use pamoja_update::{Envelope, Manifest, PayloadFormat, ENVELOPE_MAX};
///
/// let author = DeviceIdentity::from_seed(&[1u8; 32]);
/// let manifest = Manifest {
///     structure_version: pamoja_update::STRUCTURE_VERSION,
///     sequence: 7,
///     vendor_id: [0xab; 16],
///     class_id: [0xcd; 16],
///     format: PayloadFormat::Raw,
///     storage: 0,
///     digest: [0x11; 32],
///     size: 4096,
///     expires: 0,
/// };
///
/// let mut buf = [0u8; ENVELOPE_MAX];
/// let written = manifest.sign(&author, &mut buf).unwrap();
///
/// let envelope = Envelope::decode(&buf[..written]).unwrap();
/// let checked = envelope.verify(&author.public()).unwrap();
/// assert_eq!(checked.sequence, 7);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// Which iteration of the manifest format this is.
    pub structure_version: u8,
    /// Rises with every release. A device refuses anything not above what it runs,
    /// which is what stops a captured older image being replayed at it.
    pub sequence: u64,
    /// Who built the image.
    pub vendor_id: [u8; ID_LEN],
    /// Which kind of device it is for.
    pub class_id: [u8; ID_LEN],
    /// How the payload is encoded.
    pub format: PayloadFormat,
    /// Which slot the payload belongs in.
    pub storage: u8,
    /// The SHA-256 of the payload. Every other guarantee rests on this one.
    pub digest: [u8; DIGEST_LEN],
    /// The payload's length in bytes, known before a single byte is accepted.
    pub size: u32,
    /// When this release stops being offered, in seconds since the Unix epoch, or
    /// `0` to never expire.
    ///
    /// A sequence number alone cannot protect a device that has been offline for a
    /// long time: an attacker can hand it a release that is genuinely newer than
    /// the one it runs, but old enough to have a known flaw, and the device has no
    /// way to know a better one exists. An expiry bounds how long such a release
    /// stays usable. Setting one requires the device to have a clock.
    pub expires: u64,
}

impl Manifest {
    /// Encodes the manifest body, which is the part a signature covers.
    ///
    /// # Arguments
    ///
    /// * `buf` - the destination, at least [`MANIFEST_MAX`] bytes.
    ///
    /// # Returns
    ///
    /// How many bytes were written.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Malformed`] if `buf` is too small.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        let mut writer = Writer::new(buf);
        writer.map(9)?;

        writer.uint(KEY_STRUCTURE_VERSION)?;
        writer.uint(u64::from(self.structure_version))?;
        writer.uint(KEY_SEQUENCE)?;
        writer.uint(self.sequence)?;
        writer.uint(KEY_VENDOR)?;
        writer.bytes(&self.vendor_id)?;
        writer.uint(KEY_CLASS)?;
        writer.bytes(&self.class_id)?;
        writer.uint(KEY_FORMAT)?;
        writer.uint(self.format as u64)?;
        writer.uint(KEY_STORAGE)?;
        writer.uint(u64::from(self.storage))?;
        writer.uint(KEY_DIGEST)?;
        writer.bytes(&self.digest)?;
        writer.uint(KEY_SIZE)?;
        writer.uint(u64::from(self.size))?;
        writer.uint(KEY_EXPIRES)?;
        writer.uint(self.expires)?;

        Ok(writer.finish())
    }

    /// Decodes a manifest body.
    ///
    /// # Arguments
    ///
    /// * `bytes` - an encoded manifest body.
    ///
    /// # Returns
    ///
    /// The manifest.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Malformed`] if the encoding is not a well-formed
    /// manifest with its keys in order, or [`Refusal::UnsupportedVersion`] if it
    /// announces a structure version or payload format this build cannot apply.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(bytes);
        if reader.map()? != 9 {
            return Err(Refusal::Malformed);
        }

        let structure_version = read_key(&mut reader, KEY_STRUCTURE_VERSION)?;
        let structure_version = u8::try_from(structure_version).map_err(|_| Refusal::Malformed)?;
        // Refuse a newer structure before reading further: the fields after this
        // point only mean what this build thinks they mean at a version it knows.
        if structure_version != STRUCTURE_VERSION {
            return Err(Refusal::UnsupportedVersion);
        }

        let sequence = read_key(&mut reader, KEY_SEQUENCE)?;

        expect_key(&mut reader, KEY_VENDOR)?;
        let vendor_id = read_id(&mut reader)?;
        expect_key(&mut reader, KEY_CLASS)?;
        let class_id = read_id(&mut reader)?;

        let format = PayloadFormat::from_value(read_key(&mut reader, KEY_FORMAT)?)?;
        let storage = read_key(&mut reader, KEY_STORAGE)?;
        let storage = u8::try_from(storage).map_err(|_| Refusal::Malformed)?;

        expect_key(&mut reader, KEY_DIGEST)?;
        let digest_bytes = reader.bytes()?;
        let digest = <[u8; DIGEST_LEN]>::try_from(digest_bytes).map_err(|_| Refusal::Malformed)?;

        let size = read_key(&mut reader, KEY_SIZE)?;
        let size = u32::try_from(size).map_err(|_| Refusal::Malformed)?;

        let expires = read_key(&mut reader, KEY_EXPIRES)?;

        // Trailing bytes would mean the signed body carries something this parser
        // never looked at, so they are refused rather than ignored.
        if reader.position() != bytes.len() {
            return Err(Refusal::Malformed);
        }

        Ok(Self {
            structure_version,
            sequence,
            vendor_id,
            class_id,
            format,
            storage,
            digest,
            size,
            expires,
        })
    }

    /// Encodes the manifest and signs it, producing an envelope.
    ///
    /// # Arguments
    ///
    /// * `author` - the identity releasing the update.
    /// * `buf` - the destination, at least [`ENVELOPE_MAX`] bytes.
    ///
    /// # Returns
    ///
    /// How many bytes of `buf` the envelope occupies.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Malformed`] if `buf` is too small.
    pub fn sign(&self, author: &DeviceIdentity, buf: &mut [u8]) -> Result<usize> {
        let mut body = [0u8; MANIFEST_MAX];
        let body_len = self.encode(&mut body)?;
        seal(&body[..body_len], author, buf)
    }
}

/// A manifest body next to the signature over exactly those bytes.
#[derive(Clone, Copy, Debug)]
pub struct Envelope<'a> {
    body: &'a [u8],
    signature: [u8; SIGNATURE_LEN],
}

impl<'a> Envelope<'a> {
    /// Decodes an envelope, borrowing the signed body from the input.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the encoded envelope.
    ///
    /// # Returns
    ///
    /// The envelope, whose body is not yet trusted.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Malformed`] if the encoding is not a well-formed
    /// envelope.
    pub fn decode(bytes: &'a [u8]) -> Result<Self> {
        let mut reader = Reader::new(bytes);
        if reader.map()? != 2 {
            return Err(Refusal::Malformed);
        }

        expect_key(&mut reader, KEY_BODY)?;
        let body = reader.bytes()?;
        expect_key(&mut reader, KEY_SIGNATURE)?;
        let signature =
            <[u8; SIGNATURE_LEN]>::try_from(reader.bytes()?).map_err(|_| Refusal::Malformed)?;

        if reader.position() != bytes.len() {
            return Err(Refusal::Malformed);
        }

        Ok(Self { body, signature })
    }

    /// Checks the signature and returns the manifest it vouches for.
    ///
    /// The signature is checked before the body is interpreted, so nothing an
    /// unknown author wrote reaches the parser.
    ///
    /// # Arguments
    ///
    /// * `author` - the public key the device trusts to release updates.
    ///
    /// # Returns
    ///
    /// The manifest, now known to be from `author` and unaltered.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Signature`] if the signature is not this author's over
    /// this body, or a decoding refusal if the body is not a valid manifest.
    pub fn verify(&self, author: &PublicIdentity) -> Result<Manifest> {
        Manifest::decode(self.verified_body(author)?)
    }

    /// Checks the signature and returns the bytes it covers.
    ///
    /// An envelope carries whatever its author signed, which is a manifest in the
    /// usual case and a delegation when authority is being handed on. Checking the
    /// signature separately from reading the body lets both share one envelope
    /// shape without either having to know about the other.
    ///
    /// # Arguments
    ///
    /// * `signer` - the public key the body must be signed by.
    ///
    /// # Returns
    ///
    /// The signed bytes, now known to be from `signer` and unaltered.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Signature`] if the signature is not this signer's over
    /// this body.
    pub fn verified_body(&self, signer: &PublicIdentity) -> Result<&'a [u8]> {
        let signature = Signature::from_bytes(&self.signature);
        signer
            .verify(self.body, &signature)
            .map_err(|_| Refusal::Signature)?;
        Ok(self.body)
    }

    /// Returns the signed body, which is not yet known to be authentic.
    ///
    /// # Returns
    ///
    /// The encoded manifest the signature covers.
    pub fn body(&self) -> &'a [u8] {
        self.body
    }
}

/// Wraps a signed body and its signature into an envelope.
///
/// # Arguments
///
/// * `body` - the bytes to sign.
/// * `signer` - the identity vouching for them.
/// * `buf` - the destination.
///
/// # Returns
///
/// How many bytes of `buf` the envelope occupies.
///
/// # Errors
///
/// Returns [`Refusal::Malformed`] if `buf` is too small.
pub(crate) fn seal(body: &[u8], signer: &DeviceIdentity, buf: &mut [u8]) -> Result<usize> {
    let signature = signer.sign(body);
    let mut writer = Writer::new(buf);
    writer.map(2)?;
    writer.uint(KEY_BODY)?;
    writer.bytes(body)?;
    writer.uint(KEY_SIGNATURE)?;
    writer.bytes(&signature.to_bytes())?;
    Ok(writer.finish())
}

/// Reads an expected key and refuses anything else, holding the map to its order.
pub(crate) fn expect_key(reader: &mut Reader<'_>, key: u64) -> Result<()> {
    if reader.uint()? != key {
        return Err(Refusal::Malformed);
    }
    Ok(())
}

/// Reads an expected key and the unsigned integer that follows it.
pub(crate) fn read_key(reader: &mut Reader<'_>, key: u64) -> Result<u64> {
    expect_key(reader, key)?;
    reader.uint()
}

/// Reads a vendor or class identifier.
fn read_id(reader: &mut Reader<'_>) -> Result<[u8; ID_LEN]> {
    <[u8; ID_LEN]>::try_from(reader.bytes()?).map_err(|_| Refusal::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A manifest the tests can vary one field of at a time.
    fn sample() -> Manifest {
        Manifest {
            structure_version: STRUCTURE_VERSION,
            sequence: 42,
            vendor_id: [0xab; ID_LEN],
            class_id: [0xcd; ID_LEN],
            format: PayloadFormat::Raw,
            storage: 1,
            digest: [0x5a; DIGEST_LEN],
            size: 65_536,
            expires: 0,
        }
    }

    #[test]
    fn a_manifest_round_trips() {
        let manifest = sample();
        let mut buf = [0u8; MANIFEST_MAX];
        let written = manifest.encode(&mut buf).expect("encode");
        assert_eq!(Manifest::decode(&buf[..written]).expect("decode"), manifest);
    }

    #[test]
    fn the_encoding_fits_the_documented_buffer_sizes() {
        let mut body = [0u8; MANIFEST_MAX];
        assert!(sample().encode(&mut body).expect("encode") <= MANIFEST_MAX);

        let author = DeviceIdentity::from_seed(&[3u8; 32]);
        let mut envelope = [0u8; ENVELOPE_MAX];
        assert!(sample().sign(&author, &mut envelope).expect("sign") <= ENVELOPE_MAX);
    }

    #[test]
    fn a_signed_envelope_verifies_against_its_author() {
        let author = DeviceIdentity::from_seed(&[3u8; 32]);
        let mut buf = [0u8; ENVELOPE_MAX];
        let written = sample().sign(&author, &mut buf).expect("sign");

        let envelope = Envelope::decode(&buf[..written]).expect("decode");
        assert_eq!(envelope.verify(&author.public()).expect("verify"), sample());
    }

    #[test]
    fn a_different_author_is_refused() {
        let author = DeviceIdentity::from_seed(&[3u8; 32]);
        let impostor = DeviceIdentity::from_seed(&[4u8; 32]);
        let mut buf = [0u8; ENVELOPE_MAX];
        let written = sample().sign(&author, &mut buf).expect("sign");

        let envelope = Envelope::decode(&buf[..written]).expect("decode");
        assert_eq!(
            envelope.verify(&impostor.public()),
            Err(Refusal::Signature),
            "an envelope signed by someone else is not this device's update"
        );
    }

    #[test]
    fn altering_the_body_breaks_the_signature() {
        let author = DeviceIdentity::from_seed(&[3u8; 32]);
        let mut buf = [0u8; ENVELOPE_MAX];
        let written = sample().sign(&author, &mut buf).expect("sign");

        // Flip a bit inside the signed body, which begins after the envelope map
        // header, the body key, and the byte-string header.
        buf[6] ^= 0x01;

        let envelope = Envelope::decode(&buf[..written]).expect("decode");
        assert_eq!(envelope.verify(&author.public()), Err(Refusal::Signature));
    }

    #[test]
    fn a_newer_structure_version_is_refused() {
        let mut manifest = sample();
        manifest.structure_version = STRUCTURE_VERSION + 1;
        let mut buf = [0u8; MANIFEST_MAX];
        let written = manifest.encode(&mut buf).expect("encode");
        assert_eq!(
            Manifest::decode(&buf[..written]),
            Err(Refusal::UnsupportedVersion)
        );
    }

    #[test]
    fn an_unknown_payload_format_is_refused() {
        let mut buf = [0u8; MANIFEST_MAX];
        let written = sample().encode(&mut buf).expect("encode");
        // The format value sits immediately after its key; rewrite it to one this
        // build has no way to apply.
        let at = buf[..written]
            .windows(2)
            .position(|pair| pair == [KEY_FORMAT as u8, PayloadFormat::Raw as u8])
            .expect("the format pair");
        buf[at + 1] = 9;
        assert_eq!(
            Manifest::decode(&buf[..written]),
            Err(Refusal::UnsupportedVersion)
        );
    }

    #[test]
    fn a_reordered_map_is_refused() {
        // Swap the first two keys, which breaks the ascending order the
        // deterministic encoding requires.
        let mut buf = [0u8; MANIFEST_MAX];
        let written = sample().encode(&mut buf).expect("encode");
        buf[1] = KEY_SEQUENCE as u8;
        assert_eq!(Manifest::decode(&buf[..written]), Err(Refusal::Malformed));
    }

    #[test]
    fn trailing_bytes_are_refused() {
        let mut buf = [0u8; MANIFEST_MAX];
        let written = sample().encode(&mut buf).expect("encode");
        // A byte the parser never reads must not ride along inside a signed body.
        assert_eq!(
            Manifest::decode(&buf[..written + 1]),
            Err(Refusal::Malformed)
        );
    }

    #[test]
    fn a_truncated_manifest_is_refused() {
        let mut buf = [0u8; MANIFEST_MAX];
        let written = sample().encode(&mut buf).expect("encode");
        assert!(Manifest::decode(&buf[..written - 1]).is_err());
    }

    #[test]
    fn a_buffer_too_small_to_hold_the_manifest_is_refused() {
        let mut buf = [0u8; 8];
        assert_eq!(sample().encode(&mut buf), Err(Refusal::Malformed));
    }
}
