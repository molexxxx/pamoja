//! Property tests for the CAN identifier and length decoding.
//!
//! A J1939 identifier is decoded from a 29-bit CAN id that arrives off the bus, so the
//! decoder must never panic on arbitrary values, and a decoded identifier must recompose
//! to the exact bits it came from.

use proptest::prelude::*;

use pamoja_can::{dlc_to_len, len_to_dlc, CanId, Frame, J1939Id};

proptest! {
    #[test]
    fn j1939_id_round_trips(raw in 0u32..=0x1FFF_FFFF) {
        let id = CanId::extended(raw);
        let decoded = J1939Id::from_id(id).expect("every extended id decodes");
        prop_assert_eq!(decoded.to_id().raw(), raw);
    }

    #[test]
    fn from_id_never_panics(raw in any::<u32>(), standard in any::<u16>()) {
        // Any extended value decodes; a standard id is rejected (returns None), and
        // neither path panics.
        let _ = J1939Id::from_id(CanId::extended(raw));
        prop_assert!(J1939Id::from_id(CanId::standard(standard)).is_none());
    }

    #[test]
    fn from_parts_stays_in_range(
        priority in any::<u8>(),
        pgn in any::<u32>(),
        source in any::<u8>(),
        destination in any::<u8>(),
    ) {
        // Composing from arbitrary fields never panics, and the identifier it produces is
        // a valid extended id that round-trips through a CAN id.
        let id = J1939Id::from_parts(priority, pgn, source, destination);
        let recomposed = J1939Id::from_id(id.to_id()).expect("composed id decodes");
        prop_assert_eq!(recomposed.to_id().raw(), id.to_id().raw());
    }

    #[test]
    fn dlc_and_length_never_panic(dlc in any::<u8>(), len in 0usize..=64) {
        // The CAN-FD length code is not a plain byte count, so both directions are
        // lookups that must be total over their inputs.
        let _ = dlc_to_len(dlc);
        let _ = len_to_dlc(len);
    }

    #[test]
    fn frame_new_never_panics(raw in any::<u32>(), data in prop::collection::vec(any::<u8>(), 0..16)) {
        // A classic CAN frame carries at most eight bytes; more is an error, never a panic.
        let _ = Frame::new(CanId::extended(raw), &data);
    }
}
