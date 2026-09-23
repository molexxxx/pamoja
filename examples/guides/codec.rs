//! The codecs guide example; see docs/guides/codec.md.
//!
//! Run: `cargo run -p pamoja-examples --example codec`

use std::error::Error;

/// A snow gauge on a ridge fitting what it reports into the smallest LoRaWAN uplink: one
/// reading as a document, then six hours of depths and battery voltages as packed batches.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};
    use pamoja_lora::region::Region;

    // The gauge reports over LoRaWAN in the US915 plan, at the slowest data rate because
    // it reaches farthest. An uplink there carries only a few bytes of payload.
    let budget = Region::Us915
        .plan()
        .max_payload(0, false)
        .expect("the slowest data rate carries a payload")
        .application as usize;
    let fits = |bytes: usize| {
        if bytes <= budget {
            "fits one uplink"
        } else {
            "too big for one uplink"
        }
    };
    println!("uplink    carries {budget} bytes at the slowest US915 data rate");

    // One reading as the JSON a web service would take. CBOR carries the same document
    // in fewer bytes, but every key name still rides along with every reading.
    let reading = br#"{"depth_cm":142.5,"air_c":-6.5,"battery_mv":3712}"#;
    let cbor = json_to_cbor(reading)?;
    println!("json      {} bytes, {}", reading.len(), fits(reading.len()));
    println!("cbor      {} bytes, {}", cbor.len(), fits(cbor.len()));
    let restored = cbor_to_json(&cbor)?;
    println!("cbor      reads back as {}", String::from_utf8(restored)?);

    // A batch the gauge and the server agree on needs no key names. Six hourly depths,
    // kept to the millimeter, pack to a count, the first depth, and five small steps.
    let quantizer = Quantizer::new(10.0);
    let depths = [142.5, 143.8, 145.2, 146.0, 145.7, 145.5];
    let depth_batch = quantizer.encode(&depths)?;
    let (count, size) = (depths.len(), depth_batch.len());
    println!("depths    {count} readings in {size} bytes, {}", fits(size));
    let depths_back: Vec<String> = quantizer
        .decode(&depth_batch)?
        .iter()
        .map(|depth| format!("{depth:.1}"))
        .collect();
    println!("depths    read back as {}", depths_back.join(", "));

    // Battery millivolts are whole numbers already, so they pack with no scale, and a
    // falling voltage packs as small as a rising one.
    let battery = [3712, 3709, 3705, 3702, 3698, 3695];
    let battery_batch = encode_deltas(&battery);
    let (count, size) = (battery.len(), battery_batch.len());
    println!("battery   {count} readings in {size} bytes, {}", fits(size));
    let battery_back: Vec<String> = decode_deltas(&battery_batch)?
        .iter()
        .map(i64::to_string)
        .collect();
    println!("battery   reads back as {}", battery_back.join(", "));

    // Heavy snowfall can swallow the sensor's echo, leaving no depth at all. The
    // quantizer refuses the batch rather than send the gap as a depth.
    let refused = quantizer
        .encode(&[145.5, f32::NAN])
        .expect_err("a missing depth");
    println!("depths    refused a batch with a missing depth: {refused}");
    // ANCHOR_END: example

    assert!(cbor.len() < reading.len());
    assert!(cbor.len() > budget);
    assert!(depth_batch.len() <= budget && battery_batch.len() <= budget);
    assert!(
        cbor_to_json(&cbor)?.starts_with(br#"{"air_c""#),
        "keys come back sorted"
    );
    for (got, sent) in quantizer.decode(&depth_batch)?.iter().zip(&depths) {
        assert!((got - sent).abs() <= 0.05);
    }
    assert_eq!(decode_deltas(&battery_batch)?, battery);

    Ok(())
}
