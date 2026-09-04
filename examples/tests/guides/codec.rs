//! The codecs guide example; see docs/guides/codec.md.

/// A document moved to the compact form a metered link should carry, and a batch of
/// readings packed for the same link, both checked against the bytes their
/// specifications fix rather than round-tripped against themselves.
#[test]
fn a_document_and_a_batch_packed_for_a_metered_link() {
    // ANCHOR: example
    use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};

    // The same reading in CBOR instead of JSON, half the bytes. 21.5 rides as a
    // half-precision float, the shortest form RFC 8949 allows for it, so these are the
    // bytes the specification fixes rather than one encoder's dialect.
    let reading = br#"{"c":21.5,"ok":true}"#;
    let cbor = json_to_cbor(reading).expect("a valid document");
    assert_eq!(
        cbor,
        [0xA2, 0x61, 0x63, 0xF9, 0x4D, 0x60, 0x62, 0x6F, 0x6B, 0xF5]
    );
    assert_eq!(cbor_to_json(&cbor).expect("a valid document"), reading);

    // A batch packs to a count, then the difference between each sample and the one
    // before it, zigzagged and written as a LEB128 varint. The four small steps cost a
    // byte each; the jump to 900 zigzags to 1776 and costs the two bytes 0xF0 0x0D.
    let samples = [10i64, 11, 13, 12, 900];
    let packed = encode_deltas(&samples);
    assert_eq!(packed, [0x05, 0x14, 0x02, 0x04, 0x01, 0xF0, 0x0D]);
    assert_eq!(decode_deltas(&packed).expect("a valid batch"), samples);

    // A quantizer packs f32 readings the same way, rounding at the scale first. Nothing
    // in the bytes records the scale, so encode and decode have to agree on it.
    let quantizer = Quantizer::new(100.0);
    let readings = [20.0f32, 20.1, 20.2, 20.3];
    let packed_readings = quantizer.encode(&readings);
    assert_eq!(packed_readings, [0x04, 0xA0, 0x1F, 0x14, 0x14, 0x14]);
    let restored = quantizer.decode(&packed_readings).expect("a valid batch");
    for (got, want) in restored.iter().zip(&readings) {
        assert!((got - want).abs() <= 0.01);
    }
    // ANCHOR_END: example
}
