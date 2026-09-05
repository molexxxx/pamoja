//! The first example on the README and the site: a reading taken off a wire on a field
//! node, sent over a link, and checked on the gateway that receives it, with nothing
//! plugged in and nothing running.

/// The whole path a reading travels, in the order it travels it, so the crates the front
/// page names are shown working together rather than one at a time.
#[tokio::test]
async fn a_reading_travels_from_a_node_to_a_gateway() {
    // ANCHOR: example
    use pamoja_codec::{decode_deltas, encode_deltas};
    use pamoja_core::Transport;
    use pamoja_kit::Smoother;
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_security::{DeviceIdentity, PublicIdentity};
    use pamoja_sensors::ds18b20::{temperature_from_celsius, Resolution, Scratchpad};

    // The link. A loopback broker stands in for MQTT or CoAP, so this runs with no network
    // and nothing listening. Point the node at a real transport and nothing below changes.
    let broker = LoopbackBroker::new();
    let mut node = LoopbackTransport::new(broker.clone());
    let mut gateway = LoopbackTransport::new(broker);
    node.connect().await.expect("the node connects");
    gateway.connect().await.expect("the gateway connects");
    let topic = "sensors/1/temperature";
    gateway.subscribe(topic).await.expect("the gateway listens");

    // The device's identity is provisioned once and never leaves it. The gateway is told
    // only the public half, which is how it recognises this device later.
    let device = DeviceIdentity::from_seed(&[7u8; 32]);
    let known = PublicIdentity::from_bytes(&device.public().to_bytes()).expect("a valid key");
    println!("gateway trusts device {}", known.fingerprint());

    // A stand-in for the thermometer. On a running node these nine bytes arrive from the
    // 1-Wire bus; here the library builds what a part at 25.0625 C would send.
    let off_the_bus = Scratchpad::new(
        temperature_from_celsius(25.0625, Resolution::Bits12),
        Resolution::Bits12,
        75,
        -10,
    )
    .to_bytes();

    // On the node. The part checksums every read, so a value mangled on a long run is an
    // error rather than a plausible temperature a couple of degrees off.
    let celsius = Scratchpad::parse(&off_the_bus)
        .expect("the thermometer's checksum matches")
        .temperature_celsius();
    println!("read      {celsius:.4} C");

    // Readings jitter, so smooth them, and send a batch rather than one at a time.
    // Successive readings differ by very little, so the differences cost a fraction of
    // what the readings would on a link that charges by the byte.
    let mut smoother = Smoother::new(0.5);
    let batch: Vec<i64> = [celsius, celsius + 0.5, celsius + 0.4]
        .into_iter()
        .map(|sample| (smoother.update(sample) * 100.0).round() as i64)
        .collect();
    let packed = encode_deltas(&batch);
    let (readings, bytes) = (batch.len(), packed.len());
    println!("packed    {readings} readings into {bytes} bytes");

    // Sign the batch and send it. The signature travels with the payload as one message,
    // so there is nothing to keep together and split correctly at the far end.
    let message = device.sign_message(&packed);
    node.send(topic, &message)
        .await
        .expect("the node publishes");

    // On the gateway. Verifying returns the payload, so a reading that was altered on the
    // way, or signed by some other device, never reaches the code that unpacks it.
    let received = gateway
        .recv()
        .await
        .expect("a delivery")
        .expect("a message");
    match known.verify_message(&received.payload) {
        Ok(payload) => {
            let readings = decode_deltas(payload).expect("a valid batch");
            println!("gateway   accepted {readings:?} in hundredths of a degree");
        }
        Err(error) => println!("gateway   rejected the reading: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(celsius, 25.0625);
    assert_eq!(batch, [2506, 2531, 2539]);
    assert!(packed.len() < batch.len() * 8);
    assert_eq!(received.topic, topic);
    let payload = known
        .verify_message(&received.payload)
        .expect("the gateway trusts this device");
    assert_eq!(decode_deltas(payload).expect("a valid batch"), batch);

    // A message edited in transit does not verify, so the gateway never unpacks it.
    let mut edited = received.payload.clone();
    *edited.last_mut().expect("a payload byte") ^= 0xFF;
    assert!(known.verify_message(&edited).is_err());
}
