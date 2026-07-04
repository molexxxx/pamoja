//! Property tests for the LoRaWAN data frame on the untrusted-input boundary.
//!
//! A frame decodes bytes that arrive over the air, so the decoder must never panic on
//! arbitrary input, and a frame the session encodes must decode back to the same payload
//! under the same keys.

use proptest::prelude::*;

use pamoja_lorawan::{Session, Uplink};

// A LoRaWAN payload fits inside the 256-byte PHYPayload alongside the header and MIC.
const MAX_TEST_PAYLOAD: usize = 200;

proptest! {
    #[test]
    fn uplink_round_trips(
        dev_addr in any::<u32>(),
        nwk in any::<[u8; 16]>(),
        app in any::<[u8; 16]>(),
        fcnt in any::<u32>(),
        fport in 1u8..=223,
        payload in prop::collection::vec(any::<u8>(), 0..=MAX_TEST_PAYLOAD),
    ) {
        let session = Session::new(dev_addr, nwk, app);
        let frame = session
            .encode_uplink(&Uplink::new(fcnt, fport, &payload))
            .expect("payload fits a frame");
        let rx = session
            .decode(frame.as_bytes(), fcnt)
            .expect("a frame this session encoded decodes");
        prop_assert_eq!(rx.payload(), &payload[..]);
    }

    #[test]
    fn decode_never_panics(
        dev_addr in any::<u32>(),
        nwk in any::<[u8; 16]>(),
        app in any::<[u8; 16]>(),
        fcnt in any::<u32>(),
        bytes in prop::collection::vec(any::<u8>(), 0..300),
    ) {
        // A short frame, a bad MIC, or a wrong-direction frame is an error, never a panic.
        let session = Session::new(dev_addr, nwk, app);
        let _ = session.decode(&bytes, fcnt);
    }

    #[test]
    fn a_wrong_key_is_rejected(
        nwk in any::<[u8; 16]>(),
        app in any::<[u8; 16]>(),
        payload in prop::collection::vec(any::<u8>(), 1..=64),
    ) {
        // The MIC binds a frame to its network session key, so a receiver holding a
        // different key must not accept it.
        let sender = Session::new(0x2601_1BDA, nwk, app);
        let mut other_nwk = nwk;
        other_nwk[0] ^= 1;
        let receiver = Session::new(0x2601_1BDA, other_nwk, app);
        let frame = sender
            .encode_uplink(&Uplink::new(1, 1, &payload))
            .expect("payload fits a frame");
        prop_assert!(receiver.decode(frame.as_bytes(), 1).is_err());
    }
}
