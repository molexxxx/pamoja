//! Property tests for the Modbus RTU ADU on the untrusted-input boundary.
//!
//! An ADU parses bytes that arrive off an RS485 line from a field sensor, so the parser
//! must never panic on arbitrary input, and every ADU the builder produces must parse
//! back to the same address and PDU with its CRC intact.

use proptest::prelude::*;

use pamoja_modbus::Adu;

proptest! {
    #[test]
    fn adu_round_trips(
        address in any::<u8>(),
        // A PDU is a function code plus its data, and must fit inside the 256-byte frame
        // alongside the address and the two CRC bytes.
        pdu in prop::collection::vec(any::<u8>(), 1..=253),
    ) {
        let adu = Adu::from_pdu(address, &pdu).expect("PDU within a frame");
        let parsed = Adu::parse(adu.as_bytes()).expect("a built ADU parses");
        prop_assert_eq!(parsed.address(), address);
        prop_assert_eq!(parsed.function_code(), pdu[0]);
        prop_assert_eq!(parsed.pdu(), &pdu[..]);
    }

    #[test]
    fn parse_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..320)) {
        // A short frame, an oversized one, or a CRC mismatch is an error, never a panic.
        let _ = Adu::parse(&bytes);
    }

    #[test]
    fn corrupting_a_byte_fails_the_crc(
        address in any::<u8>(),
        pdu in prop::collection::vec(any::<u8>(), 1..=64),
        flip in any::<u8>(),
    ) {
        let adu = Adu::from_pdu(address, &pdu).expect("PDU within a frame");
        let mut bytes = adu.as_bytes().to_vec();
        // Flip a bit in the address/PDU region; the CRC must no longer validate.
        let index = (flip as usize) % (bytes.len() - 2);
        bytes[index] ^= 1;
        prop_assert!(Adu::parse(&bytes).is_err());
    }
}
