//! The LoRaWAN gateway guide example; see docs/guides/gateway.md.

/// One packet forwarded to a network server and acknowledged, then one sent back for the
/// gateway to transmit and reported on, as the Semtech packet forwarder protocol carries them.
#[test]
fn a_packet_forwarded_and_a_downlink_answered() {
    // ANCHOR: example
    use pamoja_gateway::udp::{Eui, Packet, Rxpk, TxStatus, Txpk, Uplink};
    use pamoja_lora::LinkSettings;

    // A gateway on a Raspberry Pi, whose identifier is written from its network interface.
    let gateway = Eui::from_hex("b827ebfffe010203").expect("sixteen hexadecimal digits");
    let link = LinkSettings::new(7, 125_000);

    // It heard a packet on 868.1 MHz, and forwards it with the levels it was heard at and the
    // concentrator's own timestamp of the reception.
    let heard = Rxpk::new(868_100_000, link, b"TEST_PACKET_1234".to_vec())
        .with_rssi_dbm(-35)
        .with_snr_db(5.1)
        .with_timestamp_us(3_512_348_611);
    let datagram = Packet::PushData {
        token: 0x1234,
        gateway,
        uplink: Uplink::from(heard),
    }
    .to_bytes();
    println!("push      {} bytes, token {:04x}", datagram.len(), 0x1234);

    // The server reads it. Nothing about the packet has to be decoded by hand: the frequency
    // is in hertz, the datarate identifier is the link settings, and the payload is bytes.
    let Packet::PushData { uplink, .. } = Packet::parse(&datagram).expect("it is well formed")
    else {
        panic!("a PUSH_DATA parses as one");
    };
    let received = &uplink.packets[0];
    let modulation = received.modulation.link().expect("a LoRa packet");
    println!(
        "heard     {} Hz at SF{}, {} kHz, {} dBm, SNR {} dB, {} bytes",
        received.frequency_hz,
        modulation.spreading_factor(),
        modulation.bandwidth_hz() / 1_000,
        received.rssi_dbm.round_db(),
        f64::from(received.snr_db.expect("a LoRa packet has one").hundredths()) / 100.0,
        received.payload.len()
    );

    // Every uplink is acknowledged at once, by token, before anything is processed.
    let acknowledgment = Packet::parse(&datagram)
        .expect("it is well formed")
        .acknowledgment()
        .expect("a PUSH_DATA is acknowledged");
    println!("ack       {} bytes", acknowledgment.to_bytes().len());

    // Later the server sends one back, at the concentrator timestamp that hits the device's
    // receive window, with the inverted polarity a LoRaWAN device listens for.
    let downlink = Packet::PullResp {
        token: 0x00AB,
        transmit: Txpk::at(3_513_348_611, 869_525_000, link, b"downlink".to_vec())
            .with_power_dbm(27)
            .with_inverted_polarity(true)
            .without_crc(),
    };
    let Packet::PullResp { transmit, .. } =
        Packet::parse(&downlink.to_bytes()).expect("it is well formed")
    else {
        panic!("a PULL_RESP parses as one");
    };
    println!(
        "downlink  {} Hz at {} dBm, inverted IQ {}",
        transmit.frequency_hz, transmit.power_dbm, transmit.invert_polarity
    );

    // The gateway answers with what became of it. A packet already scheduled in that window is
    // refused rather than dropped silently.
    let refused = Packet::TxAck {
        token: 0x00AB,
        gateway,
        status: TxStatus::CollisionPacket,
    };
    let Packet::TxAck { status, .. } =
        Packet::parse(&refused.to_bytes()).expect("it is well formed")
    else {
        panic!("a TX_ACK parses as one");
    };
    println!("txack     {status}, scheduled {}", status.scheduled());
    // ANCHOR_END: example

    assert_eq!(received.payload, b"TEST_PACKET_1234");
    assert_eq!(acknowledgment.to_bytes(), [2, 0x12, 0x34, 0x01]);
    assert!(!status.scheduled());
}
/// A device joining a site, and the reading it sends once it has, as the network side of one
/// gateway admits and answers them.
#[test]
fn a_device_joins_a_site_and_is_answered() {
    // ANCHOR: network
    use pamoja_gateway::network::{Event, Network, Registration};
    use pamoja_gateway::udp::Rxpk;
    use pamoja_lora::region::Region;
    use pamoja_lora::LinkSettings;
    use pamoja_lorawan::{Device, Uplink};

    // One site, on the band it operates in, admitting one device it was told about.
    let dev_eui = [0x11; 8];
    let app_eui = [0x22; 8];
    let app_key = [0x33; 16];
    let mut site = Network::new(Region::Eu868.plan(), 0x00_00_2A).with_first_dev_addr(0x2601_0001);
    site.register(Registration::new(dev_eui, app_eui, app_key));

    // The gateway forwards a join request it heard. Nothing about the device is known here
    // beyond the key it was provisioned with, which is what verifies the request.
    let link = LinkSettings::new(7, 125_000);
    let device = Device::new(dev_eui, app_eui, app_key);
    let request = device.join_request(0x0102);
    let heard =
        Rxpk::new(868_100_000, link, request.as_bytes().to_vec()).with_timestamp_us(1_000_000);
    let Event::Joined {
        dev_addr, accept, ..
    } = site.uplink(&heard).expect("the request verifies")
    else {
        panic!("a join request is admitted");
    };
    println!(
        "joined    {dev_addr:#010x} at {} us, inverted IQ {}",
        accept.timestamp_us.expect("the accept is scheduled"),
        accept.invert_polarity
    );

    // The device reads the accept and sends a reading. The site decrypts it and says where an
    // answer goes, which is the uplink window plus the delay the region recommends.
    let session = device
        .accept_join(&accept.payload, 0x0102)
        .expect("the accept verifies")
        .session();
    let sent = session
        .encode_uplink(&Uplink::new(0, 2, b"21.5"))
        .expect("it fits one frame");
    let carried =
        Rxpk::new(868_100_000, link, sent.as_bytes().to_vec()).with_timestamp_us(9_000_000);
    let Event::Data {
        fcnt,
        payload,
        slot,
        ..
    } = site.uplink(&carried).expect("the frame verifies")
    else {
        panic!("a data frame is read");
    };
    println!(
        "uplink    frame {fcnt}, {} bytes, answer at {} us on {} Hz",
        payload.len(),
        slot.timestamp_us,
        slot.frequency_hz
    );

    // The answer goes out in that window, encrypted with the session the join granted.
    let downlink = site
        .answer(dev_addr, slot, 2, b"ok")
        .expect("the session is held");
    println!(
        "downlink  {} bytes at {} us",
        downlink.payload.len(),
        downlink.timestamp_us.expect("the downlink is scheduled")
    );

    // A gateway hears every network in range, and a frame from one this site never granted is
    // reported rather than refused.
    let stranger = pamoja_lorawan::Session::new(0x1234_5678, [9; 16], [8; 16])
        .encode_uplink(&Uplink::new(0, 1, b"hello"))
        .expect("it fits one frame");
    let event = site
        .uplink(&Rxpk::new(868_100_000, link, stranger.as_bytes().to_vec()))
        .expect("a frame from elsewhere is not an error");
    let Event::Foreign {
        dev_addr: heard_from,
    } = event
    else {
        panic!("a frame from another network is reported as one");
    };
    println!("foreign   {heard_from:#010x} belongs to another network");
    // ANCHOR_END: network

    assert_eq!(payload, b"21.5");
    assert_eq!(slot.timestamp_us, 10_000_000);
    assert_eq!(heard_from, 0x1234_5678);
}
