//! The mesh framing guide example; see docs/guides/mesh.md.
//!
//! Run: `cargo run -p pamoja-examples --example mesh`

use std::error::Error;

/// A reading flooded across a mesh: the frame that goes on the air, the duplicate cache that
/// relays it exactly once, the hop limit that ends the flood, and the checksum that refuses
/// what the air mangled. Then the same flood down a valley of nodes, and what goes wrong
/// with a memory too small and a payload too large.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_mesh::{Frame, SeenCache, BROADCAST};

    // A river gauge floods a level reading to every node in range. The header is fixed
    // and big-endian: version, source, destination, sequence id, hop limit, then the
    // payload and a checksum over everything but the hop limit.
    const RIVER_GAUGE: u32 = 305_419_896;
    let reading = Frame::broadcast(RIVER_GAUGE, 1, b"level=high")?;
    let to = if reading.dst() == BROADCAST {
        "every node in range"
    } else {
        "one node"
    };
    println!(
        "sent      {} bytes to {to}, hop limit {}",
        reading.as_bytes().len(),
        reading.hop_limit()
    );

    // A neighbor hears it. Every node in range rebroadcasts, so the same packet arrives
    // several times over; the source and sequence id decide which copy is the first.
    let received = Frame::parse(reading.as_bytes())?;
    println!("payload   {}", String::from_utf8_lossy(received.payload()));
    let mut seen: SeenCache<64> = SeenCache::new();
    let first = seen.record(received.dedup_key());
    let again = seen.record(received.dedup_key());
    if first && !again {
        println!("dedup     the first copy is relayed, and the second is dropped");
    }

    // Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards
    // the frame without recomputing it and the check stays end to end.
    let forwarded = received.relayed().expect("hops remain");
    let onward = Frame::parse(forwarded.as_bytes())?;
    println!(
        "relayed   hop limit {}, and the checksum still holds: {}",
        forwarded.hop_limit(),
        String::from_utf8_lossy(onward.payload())
    );

    // A frame that has run out of hops is not relayed again, which is what ends the flood.
    if received.with_hop_limit(0).relayed().is_none() {
        println!("spent     at hop limit 0 the frame goes no further");
    }

    // A payload byte the air mangled fails the checksum rather than reaching the
    // application as a plausible reading. The header is a fixed width, so the first byte
    // past it is the first byte of the reading itself.
    let mut mangled = reading.as_bytes().to_vec();
    mangled[Frame::HEADER_LEN] ^= 0xFF;
    match Frame::parse(&mangled) {
        Ok(_) => println!("a mangled frame was accepted, which should never happen"),
        Err(error) => println!("mangled   rejected: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(received.payload(), b"level=high");
    assert!(first && !again);
    assert_eq!(forwarded.hop_limit(), received.hop_limit() - 1);
    assert_eq!(onward.payload(), received.payload());

    // ANCHOR: flood
    // Six nodes down a river valley, each in range of its neighbors only. The gauge at the
    // top, node 0, floods a reading, and every node that hears a packet for the first time
    // takes it and relays it while hops remain. Copies that come back up the valley are
    // echoes, which the memory of each node drops.
    let nodes = 6;
    let mut memory = vec![SeenCache::<64>::new(); nodes];
    let flooded = Frame::broadcast(0, 1, b"level=high")?;
    memory[0].record(flooded.dedup_key());
    let mut on_the_air = std::collections::VecDeque::from([(0usize, flooded)]);
    let (mut delivered, mut relays, mut echoes, mut farthest) = (0, 0, 0, 0);
    while let Some((from, frame)) = on_the_air.pop_front() {
        for node in [from.checked_sub(1), Some(from + 1)].into_iter().flatten() {
            if node >= nodes {
                continue;
            }
            let heard = Frame::parse(frame.as_bytes())?;
            if !memory[node].record(heard.dedup_key()) {
                echoes += 1;
                continue;
            }
            delivered += 1;
            farthest = farthest.max(node);
            if let Some(onward) = heard.relayed() {
                relays += 1;
                on_the_air.push_back((node, onward));
            }
        }
    }
    println!(
        "flood     {delivered} nodes took the reading, {relays} relayed it, {echoes} echoes were dropped"
    );
    println!(
        "reach     node {farthest} was the farthest, {farthest} hops out, and node {} never heard it",
        nodes - 1
    );

    // A node whose memory holds two packets hears the reading, then two packets from
    // other nodes, then a late copy of the reading by a longer path. It has forgotten the
    // reading by then and relays it again; on a busy mesh that repeats without end.
    let mut small: SeenCache<2> = SeenCache::new();
    let rain = Frame::broadcast(7, 1, b"rain=4mm")?;
    let wind = Frame::broadcast(8, 1, b"wind=12")?;
    small.record(flooded.dedup_key());
    small.record(rain.dedup_key());
    small.record(wind.dedup_key());
    if small.record(flooded.dedup_key()) {
        println!("forgot    a memory of two packets relays the late copy again");
    }

    // A payload one byte past what a frame carries is refused before it is built. A frame
    // is sized to the 250 bytes ESP-NOW carries, less the header and the checksum.
    match Frame::broadcast(0, 2, &[0u8; Frame::MAX_PAYLOAD + 1]) {
        Ok(_) => println!("an oversized payload was framed, which should never happen"),
        Err(error) => println!(
            "too long  {} bytes refused: {error}",
            Frame::MAX_PAYLOAD + 1
        ),
    }
    // ANCHOR_END: flood

    assert_eq!(farthest, usize::from(Frame::DEFAULT_HOP_LIMIT) + 1);
    Ok(())
}
