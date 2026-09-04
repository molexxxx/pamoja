//! The first example on the README and the site: a reading off a wire, smoothed,
//! signed, and packed for a metered link, with nothing plugged in.

#[test]
fn a_reading_is_decoded_smoothed_signed_and_packed() {
    // ANCHOR: example
    use pamoja_codec::{decode_deltas, encode_deltas};
    use pamoja_kit::Smoother;
    use pamoja_security::DeviceIdentity;
    use pamoja_sensors::ds18b20::{crc8, Scratchpad};

    // The nine bytes a DS18B20 sends, CRC last; a bad CRC is a rejected read.
    let mut scratchpad = [0x91, 0x01, 0x4b, 0x46, 0x7f, 0xff, 0x0c, 0x10, 0x00];
    scratchpad[8] = crc8(&scratchpad[..8]);
    let celsius = Scratchpad::parse(&scratchpad)
        .expect("the CRC matches")
        .temperature_celsius();
    assert_eq!(celsius, 25.0625);

    // Smooth the noise out of successive readings.
    let mut smoother = Smoother::new(0.5);
    smoother.update(celsius);
    let smoothed = smoother.update(celsius + 1.0);
    assert!(smoothed > celsius && smoothed < celsius + 1.0);

    // Sign the reading so a gateway can prove which device sent it.
    let device = DeviceIdentity::from_seed(&[7u8; 32]);
    let payload = format!("{smoothed:.2}");
    let signature = device.sign(payload.as_bytes());
    let verified = device.public().verify(payload.as_bytes(), &signature);
    assert!(verified.is_ok());

    // Pack a batch of readings for a link where every byte costs money.
    let samples = [2506i64, 2507, 2509, 2508, 2510];
    let packed = encode_deltas(&samples);
    assert!(packed.len() < samples.len() * 8);
    assert_eq!(decode_deltas(&packed).expect("a valid batch"), samples);
    // ANCHOR_END: example
}
