//! The serial framing guide example; see docs/guides/serial.md.

/// The two byte-stuffing framings a UART stream carries packets with, and the streaming
/// decoder a read loop uses when a read returns an arbitrary chunk rather than a packet.
#[test]
fn slip_and_cobs_frames_on_a_byte_stream() {
    // ANCHOR: example
    use pamoja_serial::{cobs, slip};

    // A UART carries bytes, not packets, so a framing has to mark where one packet ends.
    // SLIP reserves two byte values for that, and the crate names both: END closes a
    // frame, ESC carries a byte that would otherwise look like one. The hard case is a
    // payload that already contains them, so this one does.
    let mut payload = b"lvl=".to_vec();
    payload.push(slip::END);
    payload.push(slip::ESC);
    let mut framed = [0u8; slip::max_encoded_len(8)];
    let n = slip::encode(&payload, &mut framed).expect("room for the frame");
    println!("slip      {} payload bytes framed as {n}", payload.len());

    // Decoding gives the payload back unchanged, reserved bytes and all.
    let mut restored = [0u8; 8];
    let m = slip::decode(&framed[..n], &mut restored).expect("a well-formed frame");
    println!("slip      decoded back to {m} bytes");

    // COBS trades that escaping for one code byte per run of up to 254 non-zero bytes,
    // each run led by its own length, so a frame never grows by more than a byte per 254.
    // Zero is the delimiter, and COBS is what takes it out of the data.
    let mut packet = b"lvl=".to_vec();
    packet.push(cobs::DELIMITER);
    packet.extend_from_slice(b"7");
    let mut cobs_framed = [0u8; cobs::max_encoded_len(8)];
    let framed_len = cobs::encode(&packet, &mut cobs_framed).expect("room for the frame");
    let packet_len = packet.len();
    println!("cobs      {packet_len} payload bytes framed as {framed_len}");

    // A serial read returns whatever arrived, which is rarely one whole frame. This chunk
    // holds two good frames with a truncated one between them; the decoder hands over the
    // good ones and discards only the bad frame.
    let mut chunk = Vec::new();
    chunk.extend_from_slice(b"ok");
    chunk.push(slip::END);
    chunk.push(slip::ESC); // a frame that ends before its escape pair completes
    chunk.push(slip::END);
    chunk.extend_from_slice(b"go");
    chunk.push(slip::END);

    let mut decoder: slip::SlipDecoder<16> = slip::SlipDecoder::new();
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut discarded = 0;
    for &byte in &chunk {
        match decoder.push(byte) {
            Ok(Some(complete)) => frames.push(complete.to_vec()),
            Ok(None) => {}
            Err(_) => discarded += 1,
        }
    }
    for frame in &frames {
        println!("received  {}", String::from_utf8_lossy(frame));
    }
    println!("discarded {discarded} frame the stream mangled");
    // ANCHOR_END: example

    // The frames RFC 1055 and the COBS paper fix are pinned in the crate's own tests, so
    // a guide asserts behaviour instead.
    assert!(n > payload.len());
    assert!(framed_len > packet.len());
    assert_eq!(&restored[..m], &payload[..]);
    assert_eq!(frames, [b"ok".to_vec(), b"go".to_vec()]);
    assert_eq!(discarded, 1);
}
