//! Property tests for the serial framing on the untrusted-input boundary.
//!
//! SLIP and COBS decode bytes that arrive off a UART from another device, so the
//! decoders must never panic on arbitrary input, and every payload must survive an
//! encode/decode round trip unchanged.

use proptest::prelude::*;

use pamoja_serial::cobs::{self, CobsDecoder};
use pamoja_serial::slip::{self, SlipDecoder};
use pamoja_serial::SerialError;

/// Payloads up to a kilobyte, the working size for a serial packet.
fn payload() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..1024)
}

proptest! {
    #[test]
    fn slip_round_trips(payload in payload()) {
        // Worst case each byte is escaped to two, plus the trailing delimiter.
        let mut framed = vec![0u8; payload.len() * 2 + 2];
        let n = slip::encode(&payload, &mut framed).expect("encode fits");
        let mut decoded = vec![0u8; payload.len()];
        let m = slip::decode(&framed[..n], &mut decoded).expect("decode succeeds");
        prop_assert_eq!(&decoded[..m], &payload[..]);
    }

    #[test]
    fn cobs_round_trips(payload in payload()) {
        // COBS adds at most one overhead byte per 254 of payload, plus the delimiter.
        let mut framed = vec![0u8; payload.len() + payload.len() / 254 + 3];
        let n = cobs::encode(&payload, &mut framed).expect("encode fits");
        let mut decoded = vec![0u8; payload.len()];
        let m = cobs::decode(&framed[..n], &mut decoded).expect("decode succeeds");
        prop_assert_eq!(&decoded[..m], &payload[..]);
    }

    #[test]
    fn slip_decode_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        // A decode never expands, so an output the size of the input always suffices;
        // the decoder returns an error on malformed input but must not panic.
        let mut out = vec![0u8; bytes.len().max(1)];
        let _: Result<usize, SerialError> = slip::decode(&bytes, &mut out);
    }

    #[test]
    fn cobs_decode_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        let mut out = vec![0u8; bytes.len().max(1)];
        let _: Result<usize, SerialError> = cobs::decode(&bytes, &mut out);
    }

    #[test]
    fn streaming_decoders_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..2048)) {
        // The streaming decoders reassemble frames byte by byte from the wire; a frame
        // larger than the fixed buffer is an error, never a panic.
        let mut slip = SlipDecoder::<256>::new();
        let mut cobs = CobsDecoder::<256>::new();
        for &byte in &bytes {
            let _ = slip.push(byte);
            let _ = cobs.push(byte);
        }
    }
}
