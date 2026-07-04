//! Property tests for the mesh frame on the untrusted-input boundary.
//!
//! A frame parses bytes that arrive over a radio link from an unknown neighbour, so the
//! parser must never panic on arbitrary input, and every frame built by the encoder must
//! parse back to the same header and payload.

use proptest::prelude::*;

use pamoja_mesh::Frame;

proptest! {
    #[test]
    fn frame_round_trips(
        src in any::<u32>(),
        dst in any::<u32>(),
        id in any::<u16>(),
        hop_limit in 1u8..=15,
        payload in prop::collection::vec(any::<u8>(), 0..=Frame::MAX_PAYLOAD),
    ) {
        let frame = Frame::new(src, dst, id, &payload)
            .expect("payload within MAX_PAYLOAD")
            .with_hop_limit(hop_limit);
        let parsed = Frame::parse(frame.as_bytes()).expect("a built frame parses");
        prop_assert_eq!(parsed.src(), src);
        prop_assert_eq!(parsed.dst(), dst);
        prop_assert_eq!(parsed.id(), id);
        prop_assert_eq!(parsed.hop_limit(), hop_limit);
        prop_assert_eq!(parsed.payload(), &payload[..]);
    }

    #[test]
    fn parse_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        // Malformed bytes (bad version, wrong length, CRC mismatch) are an error, never
        // a panic.
        let _ = Frame::parse(&bytes);
    }

    #[test]
    fn oversized_payload_is_rejected(
        payload in prop::collection::vec(any::<u8>(), (Frame::MAX_PAYLOAD + 1)..(Frame::MAX_PAYLOAD + 64)),
    ) {
        prop_assert!(Frame::new(1, 2, 3, &payload).is_err());
    }
}
