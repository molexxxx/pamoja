//! The codecs guide example; see docs/guides/codec.md.

/// A document moved to the compact form a metered link should carry, and a batch of
/// readings packed for the same link, with what each one costs on the wire.
#[test]
fn a_document_and_a_batch_packed_for_a_metered_link() {
    // ANCHOR: example
    use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};

    // The same reading as JSON and as CBOR. Nothing is lost, and 21.5 rides as a
    // half-precision float, the shortest form RFC 8949 allows for it.
    let reading = br#"{"c":21.5,"ok":true}"#;
    let cbor = json_to_cbor(reading).expect("a valid document");
    println!("json      {} bytes", reading.len());
    println!("cbor      {} bytes", cbor.len());

    // A gateway that speaks JSON gets it back unchanged, so the compact form is a
    // transport choice rather than a different data model.
    let restored = cbor_to_json(&cbor).expect("a valid document");
    println!("back to json, unchanged: {}", restored == reading);

    // A batch of readings packs to a count, then the difference between each sample and
    // the one before it. Successive readings differ by very little, so the differences
    // cost about a byte each where the samples would cost eight.
    let samples = [10i64, 11, 13, 12, 900];
    let packed = encode_deltas(&samples);
    let (count, bytes) = (samples.len(), packed.len());
    let unpacked = decode_deltas(&packed).expect("a valid batch");
    println!("batch     {count} samples in {bytes} bytes");
    println!("unpacked  {unpacked:?}");

    // Readings that arrive as floats pack the same way once a scale is chosen. Nothing in
    // the bytes records that scale, so the sender and the receiver have to agree on it.
    let quantizer = Quantizer::new(100.0);
    let celsius = [20.0f32, 20.1, 20.2, 20.3];
    let packed_celsius = quantizer.encode(&celsius);
    let recovered = quantizer.decode(&packed_celsius).expect("a valid batch");
    let (readings, packed_bytes) = (celsius.len(), packed_celsius.len());
    println!("degrees   {readings} readings in {packed_bytes} bytes");
    println!("recovered {recovered:?}");
    // ANCHOR_END: example

    // The bytes each specification fixes are pinned in pamoja-codec's own tests, so a
    // guide can show the program instead of a table of constants.
    assert!(cbor.len() < reading.len());
    assert_eq!(restored, reading);
    assert_eq!(unpacked, samples);
    assert!(packed.len() < samples.len() * 8);
    for (got, want) in recovered.iter().zip(&celsius) {
        assert!((got - want).abs() <= 0.01);
    }
}
