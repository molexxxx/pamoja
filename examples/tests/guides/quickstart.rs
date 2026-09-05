//! The first example on the README and the site: one field node's reading taken off a
//! wire, smoothed, signed, and packed for a link that charges by the byte, start to
//! finish with nothing plugged in.

/// The whole path a reading travels on a node, in the order it travels it, so the four
/// crates the front page names are shown working together rather than one at a time.
#[test]
fn a_reading_is_decoded_smoothed_signed_and_packed() {
    // ANCHOR: example
    use pamoja_codec::{decode_deltas, encode_deltas};
    use pamoja_kit::Smoother;
    use pamoja_security::DeviceIdentity;
    use pamoja_sensors::ds18b20::{temperature_from_celsius, Resolution, Scratchpad};

    // A stand-in for the thermometer. On a running node these nine bytes arrive from the
    // 1-Wire bus; here the library builds what a part sitting at 25.0625 C would send, so
    // the program runs with nothing plugged in.
    let off_the_bus = Scratchpad::new(
        temperature_from_celsius(25.0625, Resolution::Bits12),
        Resolution::Bits12,
        75,
        -10,
    )
    .to_bytes();

    // Everything below is the node's own code, and none of it cares where the bytes came
    // from. The part checksums every read, so a value mangled on a long run comes back as
    // an error instead of a plausible temperature a couple of degrees off.
    let celsius = Scratchpad::parse(&off_the_bus)
        .expect("the thermometer's checksum matches")
        .temperature_celsius();
    println!("read      {celsius:.4} C");

    // Readings jitter. A smoother follows the trend without keeping a history to do it,
    // which matters on a part with kilobytes of RAM.
    let mut smoother = Smoother::new(0.5);
    smoother.update(celsius);
    let smoothed = smoother.update(celsius + 1.0);
    println!("smoothed  {smoothed:.4} C");

    // Sign it, so the gateway can tell this device's readings from anyone else's.
    let device = DeviceIdentity::from_seed(&[7u8; 32]);
    let reading = format!("{smoothed:.2}");
    let signature = device.sign(reading.as_bytes());
    match device.public().verify(reading.as_bytes(), &signature) {
        Ok(()) => println!("signed    {reading} C, and the signature checks out"),
        Err(error) => println!("rejected  {error}"),
    }

    // Send a batch rather than a reading at a time. Successive samples differ by very
    // little, so writing down the differences costs a fraction of eight bytes each.
    let batch = [2506i64, 2507, 2509, 2508, 2510];
    let packed = encode_deltas(&batch);
    let (readings, bytes) = (batch.len(), packed.len());
    println!("packed    {readings} readings into {bytes} bytes");
    // ANCHOR_END: example

    assert_eq!(celsius, 25.0625);
    assert!(smoothed > celsius && smoothed < celsius + 1.0);
    assert!(device
        .public()
        .verify(reading.as_bytes(), &signature)
        .is_ok());
    assert!(packed.len() < batch.len() * 8);
    assert_eq!(decode_deltas(&packed).expect("a valid batch"), batch);
}
