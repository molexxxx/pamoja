//! The serial framing guide example; see docs/guides/serial.md.

/// The two byte-stuffing framings a UART stream carries packets with, checked against the
/// frames RFC 1055 and the COBS paper fix, and the streaming decoder a read loop uses.
#[test]
fn slip_and_cobs_frames_on_a_byte_stream() {
    // ANCHOR: example
    use pamoja_serial::{cobs, slip};

    // SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a payload
    // carrying either goes out as the two-byte pair RFC 1055 fixes for it.
    let payload = [0x01, 0xC0, 0xDB, 0x02];
    let mut frame = [0u8; slip::max_encoded_len(4)];
    let n = slip::encode(&payload, &mut frame).expect("room for the frame");
    assert_eq!(&frame[..n], &[0x01, 0xDB, 0xDC, 0xDB, 0xDD, 0x02, 0xC0]);
    let mut restored = [0u8; 4];
    let m = slip::decode(&frame[..n], &mut restored).expect("a well-formed frame");
    assert_eq!(&restored[..m], &payload);

    // COBS trades that escaping for one code byte per run of up to 254 non-zero bytes,
    // each run led by its own length. This is the worked example from the COBS paper.
    let packet = [0x11, 0x22, 0x00, 0x33];
    let mut framed = [0u8; cobs::max_encoded_len(4)];
    let n = cobs::encode(&packet, &mut framed).expect("room for the frame");
    assert_eq!(&framed[..n], &[0x03, 0x11, 0x22, 0x02, 0x33, 0x00]);

    // A serial read returns an arbitrary chunk rather than a packet. This one holds two
    // frames with a truncated one between them, and the decoder drops only the bad frame.
    let mut decoder: slip::SlipDecoder<16> = slip::SlipDecoder::new();
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut discarded = 0;
    for &byte in &[b'o', b'k', 0xC0, 0xDB, 0xC0, b'g', b'o', 0xC0] {
        match decoder.push(byte) {
            Ok(Some(complete)) => frames.push(complete.to_vec()),
            Ok(None) => {}
            Err(_) => discarded += 1,
        }
    }
    assert_eq!(frames, [b"ok".to_vec(), b"go".to_vec()]);
    assert_eq!(discarded, 1);
    // ANCHOR_END: example
}
