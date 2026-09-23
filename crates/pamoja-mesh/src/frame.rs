//! The mesh frame: the addressed, hop-limited, checksummed packet on the wire.

use crate::crc::Crc16;
use crate::error::MeshError;

/// The destination that addresses a frame to every node, for flooding the whole mesh.
pub const BROADCAST: u32 = 0xFFFF_FFFF;

/// An addressed mesh packet.
///
/// A frame names where it came from and where it is going, carries a sequence number its
/// origin assigns, counts down a hop limit as it is relayed, and ends with a checksum.
/// The byte layout is fixed and big-endian:
///
/// ```text
/// 0       version
/// 1..=4   source node       (u32)
/// 5..=8   destination node  (u32, BROADCAST for every node)
/// 9..=10  sequence id       (u16)
/// 11      hop limit
/// 12..    payload
/// last 2  checksum          (u16)
/// ```
///
/// The checksum covers every byte except the hop limit, which changes at each relay. So
/// the check is end to end: a node can confirm a flooded packet's payload is intact no
/// matter how many relays forwarded it, and a relay spends a hop without recomputing it.
/// The whole frame lives in a fixed buffer, so neither building nor parsing allocates.
///
/// # Examples
///
/// ```
/// use pamoja_mesh::Frame;
///
/// // A water tank's level sensor, node 10, sends its seventh reading to the pump
/// // controller, node 11.
/// let (tank, pump) = (10, 11);
/// let frame = Frame::new(tank, pump, 7, b"level=62").unwrap();
/// assert_eq!((frame.src(), frame.dst(), frame.id()), (tank, pump, 7));
///
/// // The pump reads the same frame back off the air.
/// let received = Frame::parse(frame.as_bytes()).unwrap();
/// assert_eq!(received.payload(), b"level=62");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    bytes: [u8; Frame::MAX_LEN],
    len: usize,
}

impl Frame {
    /// The largest a mesh frame may be, in bytes, sized to the payload of a connectionless
    /// ESP-NOW frame.
    pub const MAX_LEN: usize = 250;

    /// The fixed header length in bytes: version, source, destination, sequence id, and
    /// hop limit.
    pub const HEADER_LEN: usize = 12;

    /// The non-payload bytes of a frame: the header plus the trailing checksum.
    pub const OVERHEAD: usize = Self::HEADER_LEN + 2;

    /// The largest payload a single frame can carry.
    pub const MAX_PAYLOAD: usize = Self::MAX_LEN - Self::OVERHEAD;

    /// The protocol version this build writes and accepts.
    pub const VERSION: u8 = 1;

    /// The hop limit a newly built frame starts with, enough for a small local mesh.
    pub const DEFAULT_HOP_LIMIT: u8 = 3;

    // The offset of the hop-limit byte, the one byte the checksum does not cover.
    const HOP_LIMIT: usize = 11;

    /// Builds a frame from a source, a destination, a sequence id, and a payload, starting
    /// at [`DEFAULT_HOP_LIMIT`](Frame::DEFAULT_HOP_LIMIT).
    ///
    /// # Arguments
    ///
    /// * `src` - the origin node's address.
    /// * `dst` - the destination node's address, or [`BROADCAST`] for every node.
    /// * `id` - the sequence number the origin assigns, increasing per message; with the
    ///   source it identifies a packet as it floods, for [`dedup_key`](Frame::dedup_key).
    /// * `payload` - the bytes to carry.
    ///
    /// # Returns
    ///
    /// The frame, ready to send.
    ///
    /// # Errors
    ///
    /// Returns [`MeshError::PayloadTooLong`] if `payload` is longer than
    /// [`MAX_PAYLOAD`](Frame::MAX_PAYLOAD).
    pub fn new(src: u32, dst: u32, id: u16, payload: &[u8]) -> Result<Frame, MeshError> {
        if payload.len() > Self::MAX_PAYLOAD {
            return Err(MeshError::PayloadTooLong);
        }
        let len = Self::OVERHEAD + payload.len();
        let mut bytes = [0u8; Self::MAX_LEN];
        bytes[0] = Self::VERSION;
        bytes[1..5].copy_from_slice(&src.to_be_bytes());
        bytes[5..9].copy_from_slice(&dst.to_be_bytes());
        bytes[9..11].copy_from_slice(&id.to_be_bytes());
        bytes[Self::HOP_LIMIT] = Self::DEFAULT_HOP_LIMIT;
        bytes[12..12 + payload.len()].copy_from_slice(payload);
        let crc = Self::checksum(&bytes, len);
        bytes[len - 2..len].copy_from_slice(&crc.to_be_bytes());
        Ok(Frame { bytes, len })
    }

    /// Builds a frame addressed to every node, for flooding the whole mesh.
    ///
    /// # Arguments
    ///
    /// * `src` - the origin node's address.
    /// * `id` - the sequence number the origin assigns.
    /// * `payload` - the bytes to carry.
    ///
    /// # Returns
    ///
    /// The broadcast frame, ready to send.
    ///
    /// # Errors
    ///
    /// Returns [`MeshError::PayloadTooLong`] if `payload` is longer than
    /// [`MAX_PAYLOAD`](Frame::MAX_PAYLOAD).
    pub fn broadcast(src: u32, id: u16, payload: &[u8]) -> Result<Frame, MeshError> {
        Self::new(src, BROADCAST, id, payload)
    }

    /// Sets the hop limit, the number of further relays the frame is allowed.
    ///
    /// The checksum does not cover the hop limit, so this needs no recomputation and
    /// leaves a parsed frame still valid.
    ///
    /// # Arguments
    ///
    /// * `hop_limit` - the new hop limit. `0` means no node should relay the frame further.
    ///
    /// # Returns
    ///
    /// The frame with the hop limit set, for chaining.
    pub fn with_hop_limit(mut self, hop_limit: u8) -> Frame {
        self.bytes[Self::HOP_LIMIT] = hop_limit;
        self
    }

    /// Parses a received frame, verifying its version and checksum.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the raw frame as it came off the radio.
    ///
    /// # Returns
    ///
    /// The validated frame.
    ///
    /// # Errors
    ///
    /// Returns [`MeshError::FrameTooShort`] or [`MeshError::FrameTooLong`] if the length is
    /// outside a frame's bounds, [`MeshError::UnsupportedVersion`] if the version byte is
    /// not [`VERSION`](Frame::VERSION), or [`MeshError::CrcMismatch`] if the checksum does
    /// not match the contents.
    pub fn parse(bytes: &[u8]) -> Result<Frame, MeshError> {
        if bytes.len() < Self::OVERHEAD {
            return Err(MeshError::FrameTooShort);
        }
        if bytes.len() > Self::MAX_LEN {
            return Err(MeshError::FrameTooLong);
        }
        if bytes[0] != Self::VERSION {
            return Err(MeshError::UnsupportedVersion(bytes[0]));
        }
        let len = bytes.len();
        let expected = Self::checksum(bytes, len);
        let found = u16::from_be_bytes([bytes[len - 2], bytes[len - 1]]);
        if expected != found {
            return Err(MeshError::CrcMismatch { expected, found });
        }
        let mut buffer = [0u8; Self::MAX_LEN];
        buffer[..len].copy_from_slice(bytes);
        Ok(Frame { bytes: buffer, len })
    }

    // The checksum over every byte except the mutable hop limit and the checksum field:
    // the leading header bytes, then the payload.
    fn checksum(bytes: &[u8], len: usize) -> u16 {
        let mut crc = Crc16::new();
        crc.update(&bytes[..Self::HOP_LIMIT]);
        crc.update(&bytes[12..len - 2]);
        crc.finish()
    }

    /// Returns the protocol version.
    ///
    /// # Returns
    ///
    /// The version byte.
    pub fn version(&self) -> u8 {
        self.bytes[0]
    }

    /// Returns the source node's address.
    ///
    /// # Returns
    ///
    /// The address of the node that originated the frame.
    pub fn src(&self) -> u32 {
        u32::from_be_bytes([self.bytes[1], self.bytes[2], self.bytes[3], self.bytes[4]])
    }

    /// Returns the destination node's address.
    ///
    /// # Returns
    ///
    /// The address of the destination node, or [`BROADCAST`] for every node.
    pub fn dst(&self) -> u32 {
        u32::from_be_bytes([self.bytes[5], self.bytes[6], self.bytes[7], self.bytes[8]])
    }

    /// Returns the sequence id the origin assigned.
    ///
    /// # Returns
    ///
    /// The sequence number.
    pub fn id(&self) -> u16 {
        u16::from_be_bytes([self.bytes[9], self.bytes[10]])
    }

    /// Returns the remaining hop limit.
    ///
    /// # Returns
    ///
    /// The number of further relays the frame is allowed.
    pub fn hop_limit(&self) -> u8 {
        self.bytes[Self::HOP_LIMIT]
    }

    /// Returns the payload.
    ///
    /// # Returns
    ///
    /// The carried bytes, without the header or checksum.
    pub fn payload(&self) -> &[u8] {
        &self.bytes[12..self.len - 2]
    }

    /// Returns the whole frame, checksum included, ready for the radio.
    ///
    /// # Returns
    ///
    /// The frame as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Reports whether the frame is addressed to every node.
    ///
    /// # Returns
    ///
    /// `true` if the destination is [`BROADCAST`].
    pub fn is_broadcast(&self) -> bool {
        self.dst() == BROADCAST
    }

    /// Returns the key that identifies this packet as it floods: its source and sequence
    /// id.
    ///
    /// # Returns
    ///
    /// The `(source, id)` pair, for a [`SeenCache`](crate::SeenCache).
    pub fn dedup_key(&self) -> (u32, u16) {
        (self.src(), self.id())
    }

    /// Returns the frame to forward one hop further, with a hop spent.
    ///
    /// # Returns
    ///
    /// The same frame with its hop limit reduced by one, or [`None`] if the hop limit is
    /// already `0` and the frame must not be relayed further.
    pub fn relayed(&self) -> Option<Frame> {
        let hop_limit = self.hop_limit();
        if hop_limit == 0 {
            return None;
        }
        let mut forwarded = *self;
        forwarded.bytes[Self::HOP_LIMIT] = hop_limit - 1;
        Some(forwarded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_then_parse_round_trips() {
        let frame = Frame::new(0x0102_0304, 0x0506_0708, 0x090A, b"payload").unwrap();
        let parsed = Frame::parse(frame.as_bytes()).unwrap();
        assert_eq!(parsed.version(), Frame::VERSION);
        assert_eq!(parsed.src(), 0x0102_0304);
        assert_eq!(parsed.dst(), 0x0506_0708);
        assert_eq!(parsed.id(), 0x090A);
        assert_eq!(parsed.hop_limit(), Frame::DEFAULT_HOP_LIMIT);
        assert_eq!(parsed.payload(), b"payload");
    }

    #[test]
    fn an_empty_payload_round_trips() {
        let frame = Frame::new(1, 2, 3, b"").unwrap();
        assert_eq!(frame.as_bytes().len(), Frame::OVERHEAD);
        let parsed = Frame::parse(frame.as_bytes()).unwrap();
        assert_eq!(parsed.payload(), b"");
    }

    #[test]
    fn broadcast_is_addressed_to_every_node() {
        let frame = Frame::broadcast(0x42, 1, b"hi").unwrap();
        assert_eq!(frame.dst(), BROADCAST);
        assert!(frame.is_broadcast());
        assert!(!Frame::new(0x42, 0x43, 1, b"hi").unwrap().is_broadcast());
    }

    #[test]
    fn the_largest_payload_fits_and_a_larger_one_does_not() {
        let big = [0u8; Frame::MAX_PAYLOAD];
        let frame = Frame::new(1, 2, 3, &big).unwrap();
        assert_eq!(frame.as_bytes().len(), Frame::MAX_LEN);

        let too_big = [0u8; Frame::MAX_PAYLOAD + 1];
        assert_eq!(
            Frame::new(1, 2, 3, &too_big),
            Err(MeshError::PayloadTooLong)
        );
    }

    #[test]
    fn parse_rejects_a_short_frame() {
        let short = [0u8; Frame::OVERHEAD - 1];
        assert_eq!(Frame::parse(&short), Err(MeshError::FrameTooShort));
    }

    #[test]
    fn parse_rejects_an_oversized_frame() {
        let big = [0u8; Frame::MAX_LEN + 1];
        assert_eq!(Frame::parse(&big), Err(MeshError::FrameTooLong));
    }

    #[test]
    fn parse_rejects_an_unknown_version() {
        let mut bytes = Frame::new(1, 2, 3, b"x").unwrap().as_bytes().to_vec();
        bytes[0] = 0xFF;
        assert_eq!(
            Frame::parse(&bytes),
            Err(MeshError::UnsupportedVersion(0xFF))
        );
    }

    #[test]
    fn parse_rejects_a_corrupt_payload() {
        let mut bytes = Frame::new(1, 2, 3, b"data").unwrap().as_bytes().to_vec();
        bytes[12] ^= 0xFF; // flip a payload byte
        assert!(matches!(
            Frame::parse(&bytes),
            Err(MeshError::CrcMismatch { .. })
        ));
    }

    #[test]
    fn the_checksum_ignores_the_hop_limit() {
        // Changing only the hop-limit byte must not break the end-to-end checksum, which
        // is what lets a relay spend a hop without recomputing it.
        let frame = Frame::new(1, 2, 3, b"data").unwrap();
        let mut bytes = frame.as_bytes().to_vec();
        bytes[11] = 99;
        let parsed = Frame::parse(&bytes).unwrap();
        assert_eq!(parsed.hop_limit(), 99);
    }

    #[test]
    fn with_hop_limit_leaves_the_frame_valid() {
        let frame = Frame::new(1, 2, 3, b"data").unwrap().with_hop_limit(7);
        assert_eq!(frame.hop_limit(), 7);
        assert_eq!(Frame::parse(frame.as_bytes()).unwrap().hop_limit(), 7);
    }

    #[test]
    fn relaying_spends_a_hop_and_keeps_everything_else() {
        let frame = Frame::new(0xAA, 0xBB, 5, b"flood")
            .unwrap()
            .with_hop_limit(2);
        let forwarded = frame.relayed().unwrap();
        assert_eq!(forwarded.hop_limit(), 1);
        assert_eq!(forwarded.src(), frame.src());
        assert_eq!(forwarded.dst(), frame.dst());
        assert_eq!(forwarded.id(), frame.id());
        assert_eq!(forwarded.payload(), frame.payload());
        // The forwarded frame is still valid on the wire.
        assert!(Frame::parse(forwarded.as_bytes()).is_ok());
    }

    #[test]
    fn a_frame_out_of_hops_is_not_relayed() {
        let frame = Frame::new(1, 2, 3, b"x").unwrap().with_hop_limit(0);
        assert_eq!(frame.relayed(), None);
    }

    #[test]
    fn dedup_key_is_source_and_id() {
        let frame = Frame::new(0xDEAD_BEEF, 2, 0x1234, b"x").unwrap();
        assert_eq!(frame.dedup_key(), (0xDEAD_BEEF, 0x1234));
    }

    #[test]
    fn the_largest_payload_round_trips() {
        let payload = [0xCD; Frame::MAX_PAYLOAD];
        let frame = Frame::new(1, 2, 3, &payload).unwrap();
        assert_eq!(frame.as_bytes().len(), Frame::MAX_LEN);
        let parsed = Frame::parse(frame.as_bytes()).unwrap();
        assert_eq!(parsed.payload(), &payload[..]);
    }
}
