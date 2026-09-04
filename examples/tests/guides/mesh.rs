//! The mesh framing guide example; see docs/guides/mesh.md.

/// A reading flooded across a mesh: the frame that goes on the air, the duplicate cache that
/// relays it exactly once, the hop limit that ends the flood, and the checksum that refuses
/// what the air mangled.
#[test]
fn a_reading_flooded_across_a_mesh() {
    // ANCHOR: example
    use pamoja_mesh::{crc16, Frame, SeenCache, BROADCAST};

    // A river gauge floods a reading to every node in range. The header is fixed and
    // big-endian: version, source, destination, sequence id, hop limit, then the payload
    // and a checksum over everything but the hop limit.
    let reading = Frame::broadcast(0x1234_5678, 1, b"level=high").expect("the payload fits");
    assert_eq!(reading.dst(), BROADCAST);
    assert_eq!(
        reading.as_bytes(),
        b"\x01\x12\x34\x56\x78\xFF\xFF\xFF\xFF\x00\x01\x03level=high\x33\x35"
    );

    // The checksum is CRC-16/CCITT-FALSE, whose published check value fixes the polynomial
    // and the starting value.
    assert_eq!(crc16(b"123456789"), 0x29B1);

    // A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
    // several times over; the source and sequence id decide which copy is the first.
    let received = Frame::parse(reading.as_bytes()).expect("the checksum matches");
    assert_eq!(received.payload(), b"level=high");
    let mut seen: SeenCache<64> = SeenCache::new();
    assert!(seen.record(received.dedup_key()));
    assert!(!seen.record(received.dedup_key()));

    // Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards
    // the frame without recomputing it and the check stays end to end.
    let forwarded = received.relayed().expect("hops remain");
    assert_eq!(forwarded.hop_limit(), received.hop_limit() - 1);
    let onward = Frame::parse(forwarded.as_bytes()).expect("the checksum still matches");
    assert_eq!(onward.payload(), received.payload());
    assert_eq!(received.with_hop_limit(0).relayed(), None);

    // A payload byte the air mangled fails the checksum rather than reaching the
    // application as a plausible reading.
    let mut mangled = reading.as_bytes().to_vec();
    mangled[12] ^= 0xFF;
    assert!(Frame::parse(&mangled).is_err());
    // ANCHOR_END: example
}
