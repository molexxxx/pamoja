//! The mesh framing guide example; see docs/guides/mesh.md.

/// A reading flooded across a mesh: the frame that goes on the air, the duplicate cache
/// that relays it exactly once, the hop limit that ends the flood, and the checksum that
/// refuses what the air mangled.
#[test]
fn a_reading_flooded_across_a_mesh() {
    // ANCHOR: example
    use pamoja_mesh::{Frame, SeenCache, BROADCAST};

    // A river gauge floods a level reading to every node in range. The header is fixed
    // and big-endian: version, source, destination, sequence id, hop limit, then the
    // payload and a checksum over everything but the hop limit.
    const RIVER_GAUGE: u32 = 305_419_896;
    let reading = Frame::broadcast(RIVER_GAUGE, 1, b"level=high").expect("the payload fits");
    let on_the_air = reading.as_bytes().len();
    let to_everyone = reading.dst() == BROADCAST;
    println!("sent      {on_the_air} bytes to every node in range");
    println!("addressed to broadcast: {to_everyone}");

    // A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
    // several times over; the source and sequence id decide which copy is the first.
    let received = Frame::parse(reading.as_bytes()).expect("the checksum matches");
    println!("payload   {}", String::from_utf8_lossy(received.payload()));

    let mut seen: SeenCache<64> = SeenCache::new();
    let first = seen.record(received.dedup_key());
    let again = seen.record(received.dedup_key());
    println!("first copy relayed: {first}, second copy relayed: {again}");

    // Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards
    // the frame without recomputing it and the check stays end to end.
    let forwarded = received.relayed().expect("hops remain");
    println!("relayed   hop limit {}", forwarded.hop_limit());
    let onward = Frame::parse(forwarded.as_bytes()).expect("the checksum still matches");
    println!("onward    {}", String::from_utf8_lossy(onward.payload()));

    // A frame that has run out of hops is not relayed again, which is what ends the flood.
    match received.with_hop_limit(0).relayed() {
        Some(_) => println!("a spent frame was relayed, which should never happen"),
        None => println!("spent     hop limit reached, the flood stops here"),
    }

    // A payload byte the air mangled fails the checksum rather than reaching the
    // application as a plausible reading. The header is a fixed width, so the first
    // byte past it is the first byte of the reading itself.
    let mut mangled = reading.as_bytes().to_vec();
    mangled[Frame::HEADER_LEN] ^= 0xFF;
    match Frame::parse(&mangled) {
        Ok(_) => println!("a mangled frame was accepted, which should never happen"),
        Err(error) => println!("mangled   rejected: {error}"),
    }
    // ANCHOR_END: example

    // The frame layout and the CRC check value are pinned once, in the crate's own tests
    // and the generated conformance vectors, so a guide asserts behaviour instead.
    assert_eq!(received.payload(), b"level=high");
    assert!(first);
    assert!(!again);
    assert_eq!(forwarded.hop_limit(), received.hop_limit() - 1);
    assert_eq!(onward.payload(), received.payload());
    assert_eq!(received.with_hop_limit(0).relayed(), None);
    assert!(Frame::parse(&mangled).is_err());
}
