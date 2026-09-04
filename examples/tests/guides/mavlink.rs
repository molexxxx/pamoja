//! The MAVLink guide example; see docs/guides/mavlink.md.

/// A HEARTBEAT framed for an autopilot and read back off a link that garbles and splits it,
/// checked against the bytes the v2 wire format fixes so the checksum, the seed and the
/// field order are pinned rather than round-tripped against themselves.
#[test]
fn a_heartbeat_reaches_an_autopilot_and_reads_back() {
    // ANCHOR: example
    use pamoja_mavlink::dialect::{self, Heartbeat, Message};
    use pamoja_mavlink::{crc16_mcrf4xx, Frame, Header, Parser, Version};

    // 0x6F91 over "123456789" is the catalogue check value for CRC-16/MCRF4XX, and 50 is
    // the CRC_EXTRA the common dialect publishes for HEARTBEAT.
    assert_eq!(crc16_mcrf4xx(b"123456789"), 0x6F91);
    assert_eq!(Heartbeat::CRC_EXTRA, 50);

    // A HEARTBEAT announcing an onboard controller in an active state. The v2 frame around
    // it is the 0xFD marker, the payload length, two flag bytes, the sequence, the sending
    // system and component, a 24-bit message id, the payload, then the checksum.
    let heartbeat = [0, 0, 0, 0, 18, 0, 0, 4, 3];
    let header = Header::new(1, 1, 7);
    let sent = Frame::encode_v2(header, Heartbeat::ID, &heartbeat, Heartbeat::CRC_EXTRA)
        .expect("a payload within the limit");
    assert_eq!(
        sent.as_bytes(),
        &[
            0xFD, 0x09, 0x00, 0x00, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x12, 0x00, 0x00, 0x04, 0x03, 0x75, 0x3A
        ]
    );

    // A link delivers bytes, not frames. The parser skips whatever does not start one and
    // drops a frame whose checksum fails rather than passing it on.
    let mut mangled = sent.as_bytes().to_vec();
    mangled[14] ^= 0xFF;
    let mut delivered = vec![0x11, 0x22, 0x33];
    delivered.extend_from_slice(&mangled);
    delivered.extend_from_slice(sent.as_bytes());

    let mut parser = Parser::new();
    let received = delivered
        .iter()
        .find_map(|&byte| parser.push_byte(byte, &dialect::crc_extra))
        .expect("the good frame completes");
    assert_eq!(received.version(), Version::V2);
    assert_eq!(received.message_id(), Heartbeat::ID);
    assert_eq!(received.payload(), &heartbeat[..]);
    // ANCHOR_END: example
}
