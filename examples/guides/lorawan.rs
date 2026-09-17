//! The LoRaWAN activation guide example; see docs/guides/lorawan.md.
//!
//! Run: `cargo run -p pamoja-examples --example lorawan`

use std::error::Error;

/// A network admitting a device: the join it grants, the session keys neither side sends,
/// the first uplink read at the far end, and a forged accept getting nowhere.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_lorawan::{Device, JoinGrant, Uplink};

    // The root key is provisioned into the device at the factory and known to the network
    // server. It is the only secret either side starts with; any 16 bytes stand in here.
    let app_key = [7u8; 16];

    // The device asks to join with a nonce it has not used before, which is what stops an
    // old accept being replayed at it.
    let dev_nonce = 1;
    let node = Device::new([0; 8], [0; 8], app_key);

    // The network grants the join. It draws its own nonce, names the network the device is
    // joining, and assigns the address the device will answer to from then on.
    let app_nonce = 2;
    let net_id = 19;
    let dev_addr = 0x2601_2E43;
    let grant = JoinGrant::new(app_nonce, net_id, dev_addr);
    let accept = grant.accept(&app_key, dev_nonce);
    println!(
        "granted   address {dev_addr:#010X} in a {}-byte accept",
        accept.as_bytes().len()
    );

    // The device verifies it against the root key. A join accept carries no device
    // identifier, so only that key decides whether it is for this device.
    let joined = node
        .accept_join(accept.as_bytes(), dev_nonce)
        .expect("the accept verifies under the root key");
    println!(
        "joined    the device took address {:#010X}",
        joined.dev_addr()
    );

    // Neither side transmits a session key. Both derive the same pair from the root key
    // and the two nonces, so the network reads what the device sends without ever having
    // been told how.
    let network = grant.session(&app_key, dev_nonce);
    let reading = Uplink::new(1, 1, b"level=high");
    let uplink = joined
        .session()
        .encode_uplink(&reading)
        .expect("a payload that fits one frame");
    let received = network
        .decode(uplink.as_bytes(), 1)
        .expect("the message integrity code verifies under the derived key");
    println!(
        "uplink    the network read {}",
        String::from_utf8_lossy(received.payload())
    );

    // A single byte changed in the air fails that check, so no one else can admit the
    // device or put words in its mouth.
    let mut forged = accept.as_bytes().to_vec();
    forged[1] ^= 0xFF;
    match node.accept_join(&forged, dev_nonce) {
        Ok(_) => println!("a forged accept was taken, which should never happen"),
        Err(error) => println!("forged    accept refused: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(joined.dev_addr(), dev_addr);
    assert_eq!(received.payload(), b"level=high");
    assert!(node.accept_join(&forged, dev_nonce).is_err());

    device()
}

/// A device that runs the whole exchange itself: joining on a North American network,
/// sending a reading, hearing the answer, and picking up after a power cut without joining
/// again.
fn device() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: device
    use pamoja_lora::region::Region;
    use pamoja_lorawan::device::{DeviceError, EndDevice, Heard, ReceiveWindow, Settings};
    use pamoja_lorawan::{Device, Downlink, JoinGrant};

    let app_key = [7u8; 16];
    let dev_eui = [0x70, 0xB3, 0xD5, 0x7E, 0xD0, 0x05, 0x12, 0x34];
    let join_eui = [0; 8];

    // The device owns no radio and no clock. It takes the time in microseconds and says what
    // to put on the air, so the same code runs over an SX1276, an SX1262, or nothing at all.
    // This one's radio puts out 2 to 20 dBm, and its seed would come from the radio's noise.
    let plan = Region::Us915.plan();
    let settings = Settings::new(2, 20).with_seed(1);
    let mut node = EndDevice::new(plan, Device::new(dev_eui, join_eui, app_key), settings)?;

    // A US915 device joins in passes over the band, one 125 kHz channel from each group of
    // eight. The accept is due on the downlink channel that join channel is answered on.
    let request = node.join(1, 0)?;
    println!(
        "join      {:.1} MHz at DR{}, {} dBm, {} ms on air; the accept is due {} s later on {:.1} MHz",
        f64::from(request.frequency_hz) / 1e6,
        request.data_rate,
        request.output_dbm,
        request.airtime_us / 1000,
        request.rx1.delay_us / 1_000_000,
        f64::from(request.rx1.frequency_hz) / 1e6,
    );

    // The network answers, and the device takes its address and session from the accept.
    let network = JoinGrant::new(2, 19, 0x2601_2E43);
    if let Heard::Joined { dev_addr } = node.heard(network.accept(&app_key, 1).as_bytes(), 7)? {
        println!("joined    as {dev_addr:#010X}");
    }

    // A confirmed reading. While it waits on its windows, the device refuses to send another.
    let reading = node.send(2, b"21.5", true, 10_000_000)?;
    println!(
        "uplink    {:.1} MHz at DR{}; the answer is due {} s later on {:.1} MHz",
        f64::from(reading.frequency_hz) / 1e6,
        reading.data_rate,
        reading.rx1.delay_us / 1_000_000,
        f64::from(reading.rx1.frequency_hz) / 1e6,
    );
    if let Err(DeviceError::Busy) = node.send(2, b"21.6", false, 10_000_000) {
        println!("busy      the reading before still waits on its windows");
    }

    // The network acknowledges it in the first window and sends a setting back on the same
    // port. Naming the window holds the frame to the length that window's data rate carries.
    let answer = network
        .session(&app_key, 1)
        .encode_downlink(&Downlink::new(0, 2, b"set=19.0").with_ack())?;
    if let Heard::Data(delivery) = node.heard_in(ReceiveWindow::Rx1, answer.as_bytes(), 7)? {
        println!(
            "downlink  acknowledged: {}, port {} says {}",
            delivery.acknowledged(),
            delivery.port().unwrap_or(0),
            String::from_utf8_lossy(delivery.payload()),
        );
    }

    // Before sleeping, the device saves what it settled with the network. After the power
    // cut a fresh device resumes it on a clock that starts over, and sends its next reading
    // with no join.
    let saved = node.save(12_000_000)?;
    let mut woken = EndDevice::new(plan, Device::new(dev_eui, join_eui, app_key), settings)?;
    woken.resume(&saved, 0)?;
    let next = woken.send(2, b"21.7", false, 5_000_000)?;
    println!(
        "resumed   {} saved bytes; the next reading goes out as uplink {} without joining again",
        saved.as_bytes().len(),
        woken.fcnt_up() - 1,
    );
    // ANCHOR_END: device

    assert!(next.carries_payload);
    assert_eq!(woken.dev_addr(), Some(0x2601_2E43));
    Ok(())
}
