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
