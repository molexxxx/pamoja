//! Property tests for MAVLink frame parsing on the untrusted-input boundary.
//!
//! A frame parses bytes that arrive off a serial, UDP, or TCP link from a flight stack,
//! so the parser must never panic on arbitrary input, and a frame the encoder produces
//! must parse back to the same header, message id, and payload under the same CRC_EXTRA.

use proptest::prelude::*;

use pamoja_mavlink::{Frame, Header};

proptest! {
    #[test]
    fn v2_frame_round_trips(
        system_id in any::<u8>(),
        component_id in any::<u8>(),
        sequence in any::<u8>(),
        msgid in 0u32..=0x00FF_FFFF,
        crc_extra in any::<u8>(),
        mut payload in prop::collection::vec(any::<u8>(), 0..255),
    ) {
        // MAVLink 2 truncates trailing zero payload bytes on the wire; a non-zero final
        // byte makes truncation a no-op so the round trip is exact.
        if let Some(last) = payload.last_mut() {
            if *last == 0 {
                *last = 1;
            }
        }
        let header = Header::new(system_id, component_id, sequence);
        let frame = Frame::encode_v2(header, msgid, &payload, crc_extra).expect("payload fits");
        let parsed = Frame::parse(frame.as_bytes(), crc_extra).expect("a built frame parses");
        prop_assert_eq!(parsed.message_id(), msgid);
        prop_assert_eq!(parsed.system_id(), system_id);
        prop_assert_eq!(parsed.component_id(), component_id);
        prop_assert_eq!(parsed.sequence(), sequence);
        prop_assert_eq!(parsed.payload(), &payload[..]);
    }

    #[test]
    fn parse_never_panics(
        bytes in prop::collection::vec(any::<u8>(), 0..320),
        crc_extra in any::<u8>(),
    ) {
        // A bad magic byte, a short frame, or a checksum mismatch is an error, never a panic.
        let _ = Frame::parse(&bytes, crc_extra);
    }

    #[test]
    fn a_wrong_crc_extra_is_rejected(
        msgid in 0u32..=0x00FF_FFFF,
        crc_extra in any::<u8>(),
        payload in prop::collection::vec(any::<u8>(), 1..64),
    ) {
        // CRC_EXTRA folds the message definition into the checksum, so parsing with the
        // wrong seed must fail rather than accept a mismatched message.
        let header = Header::new(1, 1, 0);
        let frame = Frame::encode_v2(header, msgid, &payload, crc_extra).expect("payload fits");
        prop_assert!(Frame::parse(frame.as_bytes(), crc_extra.wrapping_add(1)).is_err());
    }
}
