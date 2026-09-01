//! The slice of CBOR a manifest needs, written and read in place.
//!
//! Only three of the eight RFC 8949 major types appear in a manifest: unsigned
//! integers, byte strings, and one map. Encoding just those keeps this to a few
//! hundred bytes of code with no allocator, which is what lets the same parser run
//! in a bootloader as on a server. `ciborium`, which the rest of the SDK uses for
//! general CBOR, needs `alloc` and does not build for the bare-metal target.
//!
//! Reading enforces the deterministic encoding rules of RFC 8949 section 4.2.1:
//! an argument must use the shortest form that fits, and map keys must ascend.
//! A manifest therefore has exactly one valid encoding, so there is no room to
//! re-spell one into a different byte string that means the same thing.

use crate::error::{Refusal, Result};

/// Major type 0: an unsigned integer.
const MAJOR_UINT: u8 = 0;

/// Major type 2: a byte string.
const MAJOR_BYTES: u8 = 2;

/// Major type 5: a map of pairs.
const MAJOR_MAP: u8 = 5;

/// Writes the manifest subset of CBOR into a caller-provided buffer.
pub(crate) struct Writer<'a> {
    buf: &'a mut [u8],
    at: usize,
}

impl<'a> Writer<'a> {
    /// Starts writing at the beginning of `buf`.
    pub(crate) fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, at: 0 }
    }

    /// Writes a map header announcing `pairs` key-value pairs.
    pub(crate) fn map(&mut self, pairs: u64) -> Result<()> {
        self.head(MAJOR_MAP, pairs)
    }

    /// Writes an unsigned integer.
    pub(crate) fn uint(&mut self, value: u64) -> Result<()> {
        self.head(MAJOR_UINT, value)
    }

    /// Writes a byte string.
    pub(crate) fn bytes(&mut self, value: &[u8]) -> Result<()> {
        self.head(MAJOR_BYTES, value.len() as u64)?;
        self.raw(value)
    }

    /// Returns how many bytes were written.
    pub(crate) fn finish(self) -> usize {
        self.at
    }

    /// Writes an initial byte and its argument in the shortest form that fits.
    fn head(&mut self, major: u8, argument: u64) -> Result<()> {
        let tag = major << 5;
        match argument {
            0..=23 => self.raw(&[tag | argument as u8]),
            24..=0xff => self.raw(&[tag | 24, argument as u8]),
            0x100..=0xffff => {
                let value = (argument as u16).to_be_bytes();
                self.raw(&[tag | 25, value[0], value[1]])
            }
            0x1_0000..=0xffff_ffff => {
                let value = (argument as u32).to_be_bytes();
                self.raw(&[tag | 26])?;
                self.raw(&value)
            }
            _ => {
                let value = argument.to_be_bytes();
                self.raw(&[tag | 27])?;
                self.raw(&value)
            }
        }
    }

    /// Appends bytes, refusing to run past the end of the buffer.
    fn raw(&mut self, bytes: &[u8]) -> Result<()> {
        let end = self.at + bytes.len();
        if end > self.buf.len() {
            return Err(Refusal::Malformed);
        }
        self.buf[self.at..end].copy_from_slice(bytes);
        self.at = end;
        Ok(())
    }
}

/// Reads the manifest subset of CBOR, rejecting anything not deterministically encoded.
pub(crate) struct Reader<'a> {
    buf: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// Starts reading at the beginning of `buf`.
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Self { buf, at: 0 }
    }

    /// Reads a map header, returning how many pairs follow.
    pub(crate) fn map(&mut self) -> Result<u64> {
        self.head(MAJOR_MAP)
    }

    /// Reads an unsigned integer.
    pub(crate) fn uint(&mut self) -> Result<u64> {
        self.head(MAJOR_UINT)
    }

    /// Reads a byte string, borrowing it from the input.
    pub(crate) fn bytes(&mut self) -> Result<&'a [u8]> {
        let len = self.head(MAJOR_BYTES)? as usize;
        let end = self.at.checked_add(len).ok_or(Refusal::Malformed)?;
        if end > self.buf.len() {
            return Err(Refusal::Malformed);
        }
        let value = &self.buf[self.at..end];
        self.at = end;
        Ok(value)
    }

    /// Returns how many bytes have been consumed.
    pub(crate) fn position(&self) -> usize {
        self.at
    }

    /// Reads an initial byte of the expected major type and returns its argument.
    ///
    /// An argument spelled longer than it needs to be is refused, so the encoding
    /// stays one-to-one with the value.
    fn head(&mut self, expected: u8) -> Result<u64> {
        let initial = *self.buf.get(self.at).ok_or(Refusal::Malformed)?;
        self.at += 1;

        if initial >> 5 != expected {
            return Err(Refusal::Malformed);
        }

        let info = initial & 0x1f;
        let (argument, shortest) = match info {
            0..=23 => (u64::from(info), true),
            24 => {
                let value = u64::from(self.take::<1>()?[0]);
                (value, value >= 24)
            }
            25 => {
                let value = u64::from(u16::from_be_bytes(self.take::<2>()?));
                (value, value > 0xff)
            }
            26 => {
                let value = u64::from(u32::from_be_bytes(self.take::<4>()?));
                (value, value > 0xffff)
            }
            27 => {
                let value = u64::from_be_bytes(self.take::<8>()?);
                (value, value > 0xffff_ffff)
            }
            // 28 to 30 are reserved, and 31 is the indefinite length this subset
            // has no use for: a manifest's shape is known before it is written.
            _ => return Err(Refusal::Malformed),
        };

        if !shortest {
            return Err(Refusal::Malformed);
        }
        Ok(argument)
    }

    /// Reads a fixed number of bytes as an array.
    fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self.at + N;
        if end > self.buf.len() {
            return Err(Refusal::Malformed);
        }
        let mut out = [0u8; N];
        out.copy_from_slice(&self.buf[self.at..end]);
        self.at = end;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes one unsigned integer and returns the bytes written.
    fn encode_uint(value: u64) -> ([u8; 16], usize) {
        let mut buf = [0u8; 16];
        let written = {
            let mut writer = Writer::new(&mut buf);
            writer.uint(value).expect("write");
            writer.finish()
        };
        (buf, written)
    }

    #[test]
    fn integers_match_the_rfc_8949_appendix_a_vectors() {
        // Each pair is taken from the examples table in RFC 8949 Appendix A.
        for (value, expected) in [
            (0u64, [0x00].as_slice()),
            (1, &[0x01]),
            (10, &[0x0a]),
            (23, &[0x17]),
            (24, &[0x18, 0x18]),
            (25, &[0x18, 0x19]),
            (100, &[0x18, 0x64]),
            (1000, &[0x19, 0x03, 0xe8]),
            (1_000_000, &[0x1a, 0x00, 0x0f, 0x42, 0x40]),
            (
                1_000_000_000_000,
                &[0x1b, 0x00, 0x00, 0x00, 0xe8, 0xd4, 0xa5, 0x10, 0x00],
            ),
        ] {
            let (buf, written) = encode_uint(value);
            assert_eq!(&buf[..written], expected, "encoding {value}");

            let mut reader = Reader::new(&buf[..written]);
            assert_eq!(reader.uint().expect("read"), value);
        }
    }

    #[test]
    fn byte_strings_and_maps_match_the_rfc_vectors() {
        let mut buf = [0u8; 16];

        let written = {
            let mut writer = Writer::new(&mut buf);
            writer.bytes(&[]).expect("write");
            writer.finish()
        };
        assert_eq!(&buf[..written], &[0x40], "the empty byte string");

        let written = {
            let mut writer = Writer::new(&mut buf);
            writer.bytes(&[0x01, 0x02, 0x03, 0x04]).expect("write");
            writer.finish()
        };
        assert_eq!(&buf[..written], &[0x44, 0x01, 0x02, 0x03, 0x04]);

        let written = {
            let mut writer = Writer::new(&mut buf);
            writer.map(0).expect("write");
            writer.finish()
        };
        assert_eq!(&buf[..written], &[0xa0], "the empty map");

        let written = {
            let mut writer = Writer::new(&mut buf);
            writer.map(2).expect("write");
            for value in [1u64, 2, 3, 4] {
                writer.uint(value).expect("write");
            }
            writer.finish()
        };
        assert_eq!(&buf[..written], &[0xa2, 0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn a_longer_spelling_than_needed_is_refused() {
        // 0 written with a one-byte argument instead of packed into the initial
        // byte. It decodes to the same number, so accepting it would give a value
        // two encodings.
        assert!(Reader::new(&[0x18, 0x00]).uint().is_err());
        // 23 is the largest value the initial byte can carry on its own.
        assert!(Reader::new(&[0x18, 0x17]).uint().is_err());
        assert!(Reader::new(&[0x19, 0x00, 0xff]).uint().is_err());
        assert!(Reader::new(&[0x1a, 0x00, 0x00, 0xff, 0xff]).uint().is_err());
        // The shortest spelling of each is accepted.
        assert_eq!(Reader::new(&[0x00]).uint().expect("read"), 0);
        assert_eq!(Reader::new(&[0x18, 0x18]).uint().expect("read"), 24);
    }

    #[test]
    fn an_indefinite_length_is_refused() {
        // Major type 2, additional information 31.
        assert!(Reader::new(&[0x5f]).bytes().is_err());
    }

    #[test]
    fn the_wrong_major_type_is_refused() {
        // A byte string where an integer belongs.
        assert!(Reader::new(&[0x40]).uint().is_err());
    }

    #[test]
    fn a_truncated_item_is_refused() {
        assert!(Reader::new(&[]).uint().is_err());
        assert!(Reader::new(&[0x19, 0x03]).uint().is_err());
        // A byte string claiming four bytes but carrying two.
        assert!(Reader::new(&[0x44, 0x01, 0x02]).bytes().is_err());
    }

    #[test]
    fn writing_past_the_buffer_is_refused() {
        let mut buf = [0u8; 2];
        let mut writer = Writer::new(&mut buf);
        assert!(writer.bytes(&[1, 2, 3, 4]).is_err());
    }
}
