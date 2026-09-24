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

/// A sensor out of the gateway's reach and the relay next door that carries it: the relay
/// joins, the network tells it to trust the sensor, the sensor wakes it, and the answer comes
/// back through the window a relayed device keeps open.
fn relay() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: relay
    use pamoja_gateway::network::{Event, Network, Registration};
    use pamoja_gateway::udp::Rxpk;
    use pamoja_lora::region::Region;
    use pamoja_lorawan::device::{EndDevice, Heard, ReceiveWindow, Settings};
    use pamoja_lorawan::relay::{
        CadPeriodicity, CadToRx, Relay, RelayConfig, RelayHeard, RelaySettings, Wake, XtalAccuracy,
    };
    use pamoja_lorawan::Device;

    // One site: a gateway on a hill, a relay on a rooftop in range of it, and a sensor in a
    // cellar the gateway cannot hear at all.
    let app_key = [7u8; 16];
    let relay_eui = 0x70B3_D57E_D005_0001u64.to_be_bytes();
    let sensor_eui = 0x70B3_D57E_D005_0002u64.to_be_bytes();
    let plan = Region::Eu868.plan();
    let settings = Settings::new(2, 14)
        .with_tuning_range(863_000_000, 870_000_000)
        .with_seed(1);
    let mut site = Network::new(plan, 0x00_002A).with_first_dev_addr(0x2601_0001);
    site.register(Registration::new(relay_eui, [0; 8], app_key));
    site.register(Registration::new(sensor_eui, [0; 8], app_key));
    let heard_at = |transmission: &pamoja_lorawan::device::Transmission, at_us: u32| {
        Rxpk::new(
            transmission.frequency_hz,
            transmission.link,
            transmission.frame.as_bytes().to_vec(),
        )
        .with_timestamp_us(at_us)
    };

    // The relay is an end device that also listens for others, so it joins the ordinary way.
    let device = EndDevice::new(plan, Device::new(relay_eui, [0; 8], app_key), settings)?;
    let mut rooftop = Relay::new(
        device,
        RelaySettings::new(XtalAccuracy::Ppm20, CadToRx::Symbols4),
    );
    let join = rooftop.device_mut().join(1, 1_000_000)?;
    let Event::Joined { accept, .. } = site.uplink(&heard_at(&join, 1_000_000))? else {
        panic!("the relay is registered");
    };
    rooftop.heard_in(ReceiveWindow::Rx1, &accept.payload, 7)?;
    let relay_addr = rooftop.device().dev_addr().expect("an address");
    println!("relay     joined as {relay_addr:#010X}");

    // The sensor joins too. Its own uplinks never reach the gateway, but its join does,
    // because the cellar door is open while it is installed.
    let mut cellar = EndDevice::new(plan, Device::new(sensor_eui, [0; 8], app_key), settings)?;
    let sensor_join = cellar.join(2, 20_000_000)?;
    let Event::Joined { accept, .. } = site.uplink(&heard_at(&sensor_join, 20_000_000))? else {
        panic!("the sensor is registered");
    };
    cellar.heard_in(ReceiveWindow::Rx1, &accept.payload, 7)?;
    let sensor_addr = cellar.dev_addr().expect("an address");

    // The network hands the relay the key that lets it verify the sensor's wake-up frames,
    // in a command riding on the relay's own downlink.
    let empty = rooftop.device_mut().send_empty(40_000_000)?;
    let Event::Data { slot, .. } = site.uplink(&heard_at(&empty, 40_000_000))? else {
        panic!("the relay's own uplink");
    };
    let trust = site.trust_command(sensor_addr, 0, 63, 0)?;
    let configure = site.command(relay_addr, slot, &[trust])?;
    rooftop.heard_in(ReceiveWindow::Rx1, &configure.payload, 7)?;
    println!("trusted   the relay now forwards for {sensor_addr:#010X}");

    // It scans once a second on the region's wake-on-radio channel.
    rooftop.start(RelayConfig::new(
        CadPeriodicity::Ms1000,
        rooftop.region_channel(0).expect("a channel"),
    ))?;
    let scan = rooftop.next_scan(60_000_000).expect("a scan");
    println!(
        "scan      {:.1} MHz at DR{} every second",
        f64::from(scan.carrier.frequency_hz) / 1e6,
        scan.carrier.data_rate,
    );

    // The sensor turns relay mode on. Its uplink now goes out behind a frame whose preamble
    // spans a whole scan period, because it does not yet know when the relay listens.
    cellar.use_relay(true);
    let reading = cellar.send(2, b"21.5", false, 61_000_000)?;
    let exchange = reading.relay.expect("a wake-on-radio exchange");
    println!(
        "wake      {} bytes with a {}-symbol preamble, {} ms before the uplink",
        exchange.wake_up.frame().len(),
        exchange.wake_up.link.preamble_symbols(),
        (exchange.uplink_start_us - exchange.wake_up.start_us) / 1000,
    );

    // The relay hears it, knows the device, and answers with when it scanned, so every frame
    // after this one carries only the preamble the two clocks could have drifted apart.
    let Wake::Uplink {
        acknowledgment: Some(ack),
        listen: Some(listen),
        ..
    } = rooftop.heard_wor(
        &scan,
        exchange.wake_up.frame(),
        -90,
        4,
        scan.start_us + 500_000,
    )?
    else {
        panic!("the relay knows this device");
    };
    let said = cellar.heard_wor_ack(&ack.frame)?;
    println!(
        "ack       the relay scans every {} ms and forwards at DR{}",
        said.cad_periodicity.period_us() / 1000,
        said.relay_data_rate,
    );

    // The uplink follows, and the relay wraps it in one of its own on port 226.
    let due_us =
        rooftop.heard_uplink(reading.frame.as_bytes(), -88, 6, listen.start_us + 100_000)?;
    let forwarded = rooftop.forward(due_us)?;
    let Event::Data {
        dev_addr,
        payload,
        slot,
        relay: Some(relayed),
        ..
    } = site.uplink(&heard_at(&forwarded, due_us as u32))?
    else {
        panic!("a forwarded uplink");
    };
    println!(
        "forwarded {} from {dev_addr:#010X}, heard by {:#010X} at {} dBm",
        String::from_utf8_lossy(&payload),
        relayed.relay,
        relayed.metadata.rssi_dbm,
    );

    // The answer goes back the same way: the network answers the sensor, the relay unwraps it
    // and sends it on, and the sensor hears it in the window it keeps for a relay.
    let answer = site.answer(sensor_addr, slot, 2, b"set=19.0")?;
    let RelayHeard::Downlink { downlink, .. } =
        rooftop.heard_in(ReceiveWindow::Rx1, &answer.payload, 7)?
    else {
        panic!("a downlink for the sensor");
    };
    if let Heard::Data(delivery) = cellar.heard_in(ReceiveWindow::Rxr, downlink.frame(), 7)? {
        println!(
            "downlink  port {} says {}, {} s after the uplink",
            delivery.port().unwrap_or(0),
            String::from_utf8_lossy(delivery.payload()),
            exchange.rxr.delay_us / 1_000_000,
        );
    }
    // ANCHOR_END: relay

    Ok(())
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
    let dev_eui = 0x70B3_D57E_D005_1234u64.to_be_bytes();
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
        let reading_was = if delivery.acknowledged() {
            "acknowledged"
        } else {
            "not acknowledged"
        };
        println!(
            "downlink  the reading was {reading_was}, and port {} says {}",
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

    relay()
}
