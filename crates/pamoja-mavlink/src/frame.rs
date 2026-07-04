//! The MAVLink frame on the wire: the v1 and v2 packet layouts, with the checksum
//! verified on the way in so a corrupt frame never reaches the application.

use crate::crc::checksum;
use crate::error::{MavlinkError, Result};

/// The start marker of a MAVLink v1 frame.
pub const MAGIC_V1: u8 = 0xFE;
/// The start marker of a MAVLink v2 frame.
pub const MAGIC_V2: u8 = 0xFD;

/// The incompatibility-flag bit that marks a v2 frame as signed.
pub const IFLAG_SIGNED: u8 = 0x01;

/// The largest payload, in bytes, a frame can carry.
pub const MAX_PAYLOAD: usize = 255;
/// The length of a v2 signature block: a link id, a timestamp, and the signature.
pub const SIGNATURE_LEN: usize = 13;

// Header lengths, magic byte included.
const HEADER_V1: usize = 6;
const HEADER_V2: usize = 10;
const CHECKSUM_LEN: usize = 2;

/// The largest a complete frame can be: a v2 header, the largest payload, the checksum,
/// and a signature.
pub const MAX_FRAME: usize = HEADER_V2 + MAX_PAYLOAD + CHECKSUM_LEN + SIGNATURE_LEN;

/// Which MAVLink wire format a frame uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    /// The original 6-byte-header format, start marker `0xFE`.
    V1,
    /// The current format, start marker `0xFD`: a 24-bit message id, flag bytes, and
    /// optional signing.
    V2,
}

/// The addressing fields a sender stamps on every frame.
///
/// A frame says who sent it (a system and a component) and where it sits in that
/// sender's stream (a sequence number that wraps at 256), which lets a receiver detect
/// dropped frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// The sending system's id, such as a vehicle's.
    pub system_id: u8,
    /// The sending component's id within the system, such as an autopilot or a camera.
    pub component_id: u8,
    /// The sender's per-link sequence number for this frame.
    pub sequence: u8,
}

impl Header {
    /// Creates a header for a system, a component, and a sequence number.
    ///
    /// # Arguments
    ///
    /// * `system_id` - the sending system's id.
    /// * `component_id` - the sending component's id.
    /// * `sequence` - the sequence number to stamp on the frame.
    ///
    /// # Returns
    ///
    /// The header.
    pub fn new(system_id: u8, component_id: u8, sequence: u8) -> Self {
        Header {
            system_id,
            component_id,
            sequence,
        }
    }
}

/// An encoded MAVLink frame, held in a fixed buffer so encoding never allocates.
///
/// [`encode_v2`](Frame::encode_v2) and [`encode_v1`](Frame::encode_v1) build a frame to
/// send; [`parse`](Frame::parse) reads one received, verifying its checksum so a frame
/// mangled in transit is rejected rather than misread. Signing a v2 frame is a separate
/// step in [`signing`](crate::signing), since it needs a key and the timestamp the
/// sender chooses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    bytes: [u8; MAX_FRAME],
    len: usize,
}

impl Frame {
    // The byte offset of the payload, by version.
    const fn header_len(version: Version) -> usize {
        match version {
            Version::V1 => HEADER_V1,
            Version::V2 => HEADER_V2,
        }
    }

    /// Builds a v2 frame for a message, computing and appending the checksum.
    ///
    /// Trailing zero bytes of the payload are dropped before transmission as MAVLink 2
    /// requires, except that the first payload byte is always kept.
    ///
    /// # Arguments
    ///
    /// * `header` - the addressing fields to stamp on the frame.
    /// * `msgid` - the 24-bit message id.
    /// * `payload` - the serialized message payload, full length.
    /// * `crc_extra` - the `CRC_EXTRA` seed for `msgid`.
    ///
    /// # Returns
    ///
    /// The frame ready to send.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::PayloadTooLong`] if `payload` is longer than
    /// [`MAX_PAYLOAD`].
    pub fn encode_v2(header: Header, msgid: u32, payload: &[u8], crc_extra: u8) -> Result<Frame> {
        Self::assemble_v2(header, msgid, payload, crc_extra, 0)
    }

    // Builds a v2 frame with the given incompatibility flags. Signing sets IFLAG_SIGNED
    // here so the flag is covered by the checksum, then fills the signature block.
    pub(crate) fn assemble_v2(
        header: Header,
        msgid: u32,
        payload: &[u8],
        crc_extra: u8,
        incompat_flags: u8,
    ) -> Result<Frame> {
        if payload.len() > MAX_PAYLOAD {
            return Err(MavlinkError::PayloadTooLong);
        }
        let plen = truncated_len(payload);
        let signed = incompat_flags & IFLAG_SIGNED != 0;
        let total = HEADER_V2 + plen + CHECKSUM_LEN + if signed { SIGNATURE_LEN } else { 0 };

        let mut bytes = [0u8; MAX_FRAME];
        bytes[0] = MAGIC_V2;
        bytes[1] = plen as u8;
        bytes[2] = incompat_flags;
        bytes[3] = 0;
        bytes[4] = header.sequence;
        bytes[5] = header.system_id;
        bytes[6] = header.component_id;
        bytes[7] = msgid as u8;
        bytes[8] = (msgid >> 8) as u8;
        bytes[9] = (msgid >> 16) as u8;
        bytes[HEADER_V2..HEADER_V2 + plen].copy_from_slice(&payload[..plen]);

        let crc = checksum(&bytes[1..HEADER_V2 + plen], crc_extra);
        bytes[HEADER_V2 + plen..HEADER_V2 + plen + CHECKSUM_LEN]
            .copy_from_slice(&crc.to_le_bytes());

        Ok(Frame { bytes, len: total })
    }

    /// Builds a v1 frame for a message, computing and appending the checksum.
    ///
    /// # Arguments
    ///
    /// * `header` - the addressing fields to stamp on the frame.
    /// * `msgid` - the message id, which must fit a single byte for v1.
    /// * `payload` - the serialized message payload.
    /// * `crc_extra` - the `CRC_EXTRA` seed for `msgid`.
    ///
    /// # Returns
    ///
    /// The frame ready to send.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::PayloadTooLong`] if `payload` is longer than [`MAX_PAYLOAD`],
    /// or [`MavlinkError::UnknownMessage`] if `msgid` does not fit a single byte.
    pub fn encode_v1(header: Header, msgid: u32, payload: &[u8], crc_extra: u8) -> Result<Frame> {
        if payload.len() > MAX_PAYLOAD {
            return Err(MavlinkError::PayloadTooLong);
        }
        if msgid > 0xFF {
            return Err(MavlinkError::UnknownMessage(msgid));
        }
        let plen = payload.len();
        let total = HEADER_V1 + plen + CHECKSUM_LEN;

        let mut bytes = [0u8; MAX_FRAME];
        bytes[0] = MAGIC_V1;
        bytes[1] = plen as u8;
        bytes[2] = header.sequence;
        bytes[3] = header.system_id;
        bytes[4] = header.component_id;
        bytes[5] = msgid as u8;
        bytes[HEADER_V1..HEADER_V1 + plen].copy_from_slice(payload);

        let crc = checksum(&bytes[1..HEADER_V1 + plen], crc_extra);
        bytes[HEADER_V1 + plen..HEADER_V1 + plen + CHECKSUM_LEN]
            .copy_from_slice(&crc.to_le_bytes());

        Ok(Frame { bytes, len: total })
    }

    /// Parses a received frame, verifying its checksum.
    ///
    /// The message id is read from the header and its `CRC_EXTRA` is resolved through
    /// `crc_extra_for`, so a frame whose message is unknown is rejected rather than
    /// accepted unchecked. The signature of a signed v2 frame is preserved but not
    /// verified here; pass the parsed frame to a [`Verifier`](crate::signing::Verifier).
    ///
    /// # Arguments
    ///
    /// * `bytes` - the raw frame as it came off the link, from the start marker onward.
    /// * `crc_extra_for` - resolves a message id to its `CRC_EXTRA`, or `None` if unknown.
    ///
    /// # Returns
    ///
    /// The validated frame.
    ///
    /// # Errors
    ///
    /// Returns [`MavlinkError::FrameTooShort`] if the bytes cannot hold a header,
    /// [`MavlinkError::BadMagic`] if the start marker is unrecognized,
    /// [`MavlinkError::Truncated`] if the frame is shorter than its length field
    /// promises, [`MavlinkError::UnknownMessage`] if the message id has no `CRC_EXTRA`,
    /// or [`MavlinkError::CrcMismatch`] if the checksum does not verify.
    pub fn parse_with<F>(bytes: &[u8], crc_extra_for: F) -> Result<Frame>
    where
        F: FnOnce(u32) -> Option<u8>,
    {
        if bytes.is_empty() {
            return Err(MavlinkError::FrameTooShort);
        }
        let version = match bytes[0] {
            MAGIC_V1 => Version::V1,
            MAGIC_V2 => Version::V2,
            other => return Err(MavlinkError::BadMagic(other)),
        };
        let header_len = Self::header_len(version);
        if bytes.len() < header_len {
            return Err(MavlinkError::FrameTooShort);
        }
        let plen = bytes[1] as usize;
        let signed = version == Version::V2 && bytes[2] & IFLAG_SIGNED != 0;
        let total = header_len + plen + CHECKSUM_LEN + if signed { SIGNATURE_LEN } else { 0 };
        if bytes.len() < total {
            return Err(MavlinkError::Truncated);
        }

        let msgid = match version {
            Version::V1 => u32::from(bytes[5]),
            Version::V2 => {
                u32::from(bytes[7]) | u32::from(bytes[8]) << 8 | u32::from(bytes[9]) << 16
            }
        };
        let crc_extra = crc_extra_for(msgid).ok_or(MavlinkError::UnknownMessage(msgid))?;

        let crc_at = header_len + plen;
        let expected = checksum(&bytes[1..crc_at], crc_extra);
        let found = u16::from_le_bytes([bytes[crc_at], bytes[crc_at + 1]]);
        if expected != found {
            return Err(MavlinkError::CrcMismatch { expected, found });
        }

        let mut buffer = [0u8; MAX_FRAME];
        buffer[..total].copy_from_slice(&bytes[..total]);
        Ok(Frame {
            bytes: buffer,
            len: total,
        })
    }

    /// Parses a received frame whose `CRC_EXTRA` is already known.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the raw frame as it came off the link.
    /// * `crc_extra` - the `CRC_EXTRA` for the frame's message id.
    ///
    /// # Returns
    ///
    /// The validated frame.
    ///
    /// # Errors
    ///
    /// As [`parse_with`](Frame::parse_with), except the message id is always resolvable.
    pub fn parse(bytes: &[u8], crc_extra: u8) -> Result<Frame> {
        Self::parse_with(bytes, |_| Some(crc_extra))
    }

    /// Returns which wire format the frame uses.
    ///
    /// # Returns
    ///
    /// [`Version::V1`] or [`Version::V2`].
    pub fn version(&self) -> Version {
        if self.bytes[0] == MAGIC_V2 {
            Version::V2
        } else {
            Version::V1
        }
    }

    /// Returns the sequence number the sender stamped on the frame.
    ///
    /// # Returns
    ///
    /// The sequence number.
    pub fn sequence(&self) -> u8 {
        match self.version() {
            Version::V1 => self.bytes[2],
            Version::V2 => self.bytes[4],
        }
    }

    /// Returns the sending system's id.
    ///
    /// # Returns
    ///
    /// The system id.
    pub fn system_id(&self) -> u8 {
        match self.version() {
            Version::V1 => self.bytes[3],
            Version::V2 => self.bytes[5],
        }
    }

    /// Returns the sending component's id.
    ///
    /// # Returns
    ///
    /// The component id.
    pub fn component_id(&self) -> u8 {
        match self.version() {
            Version::V1 => self.bytes[4],
            Version::V2 => self.bytes[6],
        }
    }

    /// Returns the message id the frame carries.
    ///
    /// # Returns
    ///
    /// The message id: 0-255 for v1, up to 24 bits for v2.
    pub fn message_id(&self) -> u32 {
        match self.version() {
            Version::V1 => u32::from(self.bytes[5]),
            Version::V2 => {
                u32::from(self.bytes[7])
                    | u32::from(self.bytes[8]) << 8
                    | u32::from(self.bytes[9]) << 16
            }
        }
    }

    /// Returns the incompatibility flags of a v2 frame, or `0` for a v1 frame.
    ///
    /// # Returns
    ///
    /// The incompatibility flags byte.
    pub fn incompat_flags(&self) -> u8 {
        match self.version() {
            Version::V1 => 0,
            Version::V2 => self.bytes[2],
        }
    }

    /// Reports whether the frame is a signed v2 frame.
    ///
    /// # Returns
    ///
    /// `true` if the frame carries a signature.
    pub fn is_signed(&self) -> bool {
        self.version() == Version::V2 && self.incompat_flags() & IFLAG_SIGNED != 0
    }

    /// Returns the payload as it was carried, after any MAVLink 2 truncation.
    ///
    /// # Returns
    ///
    /// The payload bytes.
    pub fn payload(&self) -> &[u8] {
        let start = Self::header_len(self.version());
        let plen = self.bytes[1] as usize;
        &self.bytes[start..start + plen]
    }

    /// Returns the 13-byte signature block of a signed frame.
    ///
    /// # Returns
    ///
    /// The signature block, or [`None`] if the frame is not signed.
    pub fn signature(&self) -> Option<&[u8; SIGNATURE_LEN]> {
        if !self.is_signed() {
            return None;
        }
        let start = HEADER_V2 + self.bytes[1] as usize + CHECKSUM_LEN;
        self.bytes[start..start + SIGNATURE_LEN].try_into().ok()
    }

    /// Returns the whole frame, ready for the link.
    ///
    /// # Returns
    ///
    /// The frame as a byte slice, signature included if present.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    // The bytes a signature is computed over, with the message-specific CRC: the header
    // (start marker included), the payload, and the two checksum bytes.
    pub(crate) fn signed_region(&self) -> &[u8] {
        let start = HEADER_V2 + self.bytes[1] as usize + CHECKSUM_LEN;
        &self.bytes[..start]
    }

    // Writes the signature block of a signed frame, after the checksum.
    pub(crate) fn signature_mut(&mut self) -> &mut [u8; SIGNATURE_LEN] {
        let start = HEADER_V2 + self.bytes[1] as usize + CHECKSUM_LEN;
        (&mut self.bytes[start..start + SIGNATURE_LEN])
            .try_into()
            .expect("a signed frame reserves a full signature block")
    }
}

// The payload length after dropping trailing zero bytes. A non-empty payload keeps at
// least one byte, as MAVLink 2 requires; an empty payload stays empty. The `len > 1`
// guard supplies both, and never exceeds the payload length.
fn truncated_len(payload: &[u8]) -> usize {
    let mut len = payload.len();
    while len > 1 && payload[len - 1] == 0 {
        len -= 1;
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    // HEARTBEAT, the frame every MAVLink node emits, used to anchor the layout.
    const HEARTBEAT_ID: u32 = 0;
    const HEARTBEAT_CRC_EXTRA: u8 = 50;

    #[test]
    fn a_v2_frame_round_trips_through_parse() {
        let header = Header::new(1, 1, 7);
        let payload = [0x06, 0x08, 0x00, 0x00, 0x00, 0x02, 0x03, 0x59, 0x03];
        let frame = Frame::encode_v2(header, HEARTBEAT_ID, &payload, HEARTBEAT_CRC_EXTRA).unwrap();
        let parsed = Frame::parse(frame.as_bytes(), HEARTBEAT_CRC_EXTRA).unwrap();
        assert_eq!(parsed.version(), Version::V2);
        assert_eq!(parsed.message_id(), HEARTBEAT_ID);
        assert_eq!(parsed.system_id(), 1);
        assert_eq!(parsed.sequence(), 7);
        assert_eq!(parsed.payload(), &payload);
    }

    #[test]
    fn a_v1_frame_round_trips_through_parse() {
        let header = Header::new(1, 1, 0);
        let payload = [0x06, 0x08, 0x00, 0x00, 0x00, 0x02, 0x03, 0x59, 0x03];
        let frame = Frame::encode_v1(header, HEARTBEAT_ID, &payload, HEARTBEAT_CRC_EXTRA).unwrap();
        let parsed = Frame::parse(frame.as_bytes(), HEARTBEAT_CRC_EXTRA).unwrap();
        assert_eq!(parsed.version(), Version::V1);
        assert_eq!(parsed.message_id(), HEARTBEAT_ID);
        assert_eq!(parsed.payload(), &payload);
    }

    #[test]
    fn the_v2_header_is_laid_out_as_the_spec_requires() {
        let header = Header::new(0x2A, 0xBE, 0x10);
        let frame = Frame::encode_v2(header, 0x0A0B0C, &[1, 2, 3], 0).unwrap();
        let bytes = frame.as_bytes();
        assert_eq!(bytes[0], MAGIC_V2);
        assert_eq!(bytes[1], 3); // payload length
        assert_eq!(bytes[2], 0); // incompat flags
        assert_eq!(bytes[3], 0); // compat flags
        assert_eq!(bytes[4], 0x10); // sequence
        assert_eq!(bytes[5], 0x2A); // system id
        assert_eq!(bytes[6], 0xBE); // component id
        assert_eq!(&bytes[7..10], &[0x0C, 0x0B, 0x0A]); // msgid, little-endian
    }

    #[test]
    fn trailing_zero_bytes_are_truncated_but_the_first_is_kept() {
        let header = Header::new(1, 1, 0);
        // A payload of all zeros truncates to a single byte, never to nothing.
        let frame = Frame::encode_v2(header, 0, &[0, 0, 0, 0], 50).unwrap();
        assert_eq!(frame.payload(), &[0]);

        // Trailing zeros are dropped; an interior zero is preserved.
        let frame = Frame::encode_v2(header, 0, &[1, 0, 2, 0, 0], 50).unwrap();
        assert_eq!(frame.payload(), &[1, 0, 2]);
    }

    #[test]
    fn a_corrupt_checksum_is_rejected() {
        let header = Header::new(1, 1, 0);
        let frame = Frame::encode_v2(header, 0, &[1, 2, 3], 50).unwrap();
        let mut bytes = frame.as_bytes().to_vec();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        assert!(matches!(
            Frame::parse(&bytes, 50),
            Err(MavlinkError::CrcMismatch { .. })
        ));
    }

    #[test]
    fn the_wrong_crc_extra_is_rejected() {
        // A receiver that disagrees about the message shape folds in a different seed and
        // rejects the frame, which is the whole point of CRC_EXTRA.
        let header = Header::new(1, 1, 0);
        let frame = Frame::encode_v2(header, 0, &[1, 2, 3], 50).unwrap();
        assert!(matches!(
            Frame::parse(frame.as_bytes(), 51),
            Err(MavlinkError::CrcMismatch { .. })
        ));
    }

    #[test]
    fn an_unknown_message_is_rejected() {
        let header = Header::new(1, 1, 0);
        let frame = Frame::encode_v2(header, 999, &[1, 2, 3], 50).unwrap();
        assert_eq!(
            Frame::parse_with(frame.as_bytes(), |_| None),
            Err(MavlinkError::UnknownMessage(999))
        );
    }

    #[test]
    fn a_truncated_frame_is_rejected() {
        let header = Header::new(1, 1, 0);
        let frame = Frame::encode_v2(header, 0, &[1, 2, 3, 4, 5], 50).unwrap();
        let bytes = frame.as_bytes();
        assert_eq!(
            Frame::parse(&bytes[..bytes.len() - 1], 50),
            Err(MavlinkError::Truncated)
        );
    }

    #[test]
    fn an_unrecognized_start_marker_is_rejected() {
        assert_eq!(
            Frame::parse(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07], 50),
            Err(MavlinkError::BadMagic(0x00))
        );
    }
}
