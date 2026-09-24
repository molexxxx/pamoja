//! The LoRaWAN gateway guide example; see docs/guides/gateway.md.
//!
//! Run: `cargo run -p pamoja-examples --example gateway`

use std::error::Error;

fn main() -> std::result::Result<(), Box<dyn Error>> {
    a_gateway_and_its_server_trade_datagrams()?;
    a_device_joins_a_site_and_is_answered()?;
    a_station_session_from_both_sides()?;
    a_network_server_beside_the_gateway()?;
    Ok(())
}

/// A gateway holds its downlink path open, forwards a reading with its own counts, and is
/// asked to transmit twice, once in time and once too late, as the Semtech packet forwarder
/// protocol carries each of those.
fn a_gateway_and_its_server_trade_datagrams() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_gateway::udp::{Eui, Packet, Rxpk, Stat, TxStatus, Txpk, Uplink};
    use pamoja_lora::LinkSettings;
    use pamoja_lorawan::{Session, Uplink as Reading};

    // A gateway on a Raspberry Pi, whose identifier is written from its network interface.
    let gateway = Eui::from_hex("b827ebfffe010203").expect("sixteen hexadecimal digits");

    // Every few seconds it sends a PULL_DATA, which holds a path open through whatever
    // translates its address, so the server has somewhere to send a downlink. The server
    // answers each one, and a gateway that stops hearing answers knows the path is gone.
    let pull = Packet::PullData {
        token: 0x7a01,
        gateway,
    };
    let held = pull.acknowledgment().expect("a PULL_DATA is acknowledged");
    println!(
        "pull      {} bytes out and {} back hold the downlink path open",
        pull.to_bytes().len(),
        held.to_bytes().len()
    );

    // A node sends a reading, and the gateway hears it on 868.1 MHz at SF9, near the edge of
    // its range. It forwards the frame as it arrived, with the levels, the concentrator's own
    // timestamp, and its counts since the last report. It holds no key and reads none of it.
    let node = Session::new(0x2601_0001, [0x44; 16], [0x55; 16]);
    let frame = node.encode_uplink(&Reading::new(7, 2, b"21.5"))?;
    let heard = Rxpk::new(
        868_100_000,
        LinkSettings::new(9, 125_000),
        frame.as_bytes().to_vec(),
    )
    .with_rssi_dbm(-97)
    .with_snr_db(-3.2)
    .with_timestamp_us(3_512_348_611);
    let counts = Stat::new()
        .with_counts(2, 1, 1)
        .with_acknowledged_percent(100.0);
    let push = Packet::PushData {
        token: 0x1234,
        gateway,
        uplink: Uplink {
            packets: vec![heard],
            status: Some(counts),
        },
    };
    let datagram = push.to_bytes();
    println!(
        "push      a reading and the gateway's counts, {} bytes, token {:04x}",
        datagram.len(),
        push.token()
    );

    // The server reads it. The frequency is in hertz, the datarate identifier is the link
    // settings, and the payload is bytes, so nothing is decoded by hand.
    let Packet::PushData { uplink, .. } = Packet::parse(&datagram)? else {
        panic!("a PUSH_DATA parses as one");
    };
    let received = &uplink.packets[0];
    let link = received.modulation.link().expect("a LoRa packet");
    let snr = received.snr_db.expect("a LoRa packet has one");
    println!(
        "heard     {} Hz at SF{}, {} kHz, {} dBm, SNR {:.1} dB, CRC {}, {} bytes",
        received.frequency_hz,
        link.spreading_factor(),
        link.bandwidth_hz() / 1_000,
        received.rssi_dbm.round_db(),
        f64::from(snr.hundredths()) / 100.0,
        format!("{:?}", received.crc).to_lowercase(),
        received.payload.len()
    );
    let report = uplink.status.as_ref().expect("the counts rode along");
    println!(
        "counts    {} received, {} with a good CRC, {} forwarded, {:.1}% acknowledged",
        report.received, report.received_ok, report.forwarded, report.acknowledged_percent
    );

    // It is acknowledged at once, by token, before anything in it is read.
    let ack = Packet::parse(&datagram)?
        .acknowledgment()
        .expect("a PUSH_DATA is acknowledged");
    println!(
        "ack       token {:04x} acknowledged in {} bytes",
        ack.token(),
        ack.to_bytes().len()
    );

    // An answer goes back in a PULL_RESP, timed in the concentrator's own microseconds for
    // the device's first receive window, a second after the uplink ended, with the inverted
    // polarity a LoRaWAN device listens for.
    let answer = node.encode_downlink(&pamoja_lorawan::Downlink::new(0, 2, b"ok"))?;
    let window = Txpk::at(
        3_513_348_611,
        868_100_000,
        LinkSettings::new(9, 125_000),
        answer.as_bytes().to_vec(),
    )
    .with_power_dbm(14)
    .with_inverted_polarity(true);
    let pull_resp = Packet::PullResp {
        token: 0x00ab,
        transmit: window,
    };
    let Packet::PullResp { transmit, .. } = Packet::parse(&pull_resp.to_bytes())? else {
        panic!("a PULL_RESP parses as one");
    };
    let iq = if transmit.invert_polarity {
        "IQ inverted"
    } else {
        "IQ upright"
    };
    println!(
        "downlink  at {} us on {} Hz, {} dBm, {iq}",
        transmit.timestamp_us.expect("timed for a window"),
        transmit.frequency_hz,
        transmit.power_dbm
    );

    // The gateway answers each PULL_RESP with a TX_ACK saying what became of it: scheduled,
    // or refused with a reason, such as a window that had already passed.
    for status in [TxStatus::None, TxStatus::TooLate] {
        let reported = Packet::TxAck {
            token: 0x00ab,
            gateway,
            status,
        };
        let Packet::TxAck { status, .. } = Packet::parse(&reported.to_bytes())? else {
            panic!("a TX_ACK parses as one");
        };
        let meaning = if status.scheduled() {
            "it goes out in the device's window"
        } else {
            "it was not sent"
        };
        println!("txack     {status}: {meaning}");
    }
    // ANCHOR_END: example

    assert_eq!(held.to_bytes().len(), 4);
    assert_eq!(received.payload, frame.as_bytes());
    assert_eq!(ack.token(), 0x1234);
    assert_eq!(transmit.timestamp_us, Some(3_513_348_611));

    Ok(())
}

/// A device joining a site, the reading it sends once it has, a copy of that reading played
/// back, and a frame from another network, as the network side of one gateway sees them.
fn a_device_joins_a_site_and_is_answered() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: network
    use pamoja_gateway::network::{Event, Network, Registration};
    use pamoja_gateway::udp::{Eui, Rxpk};
    use pamoja_lora::region::Region;
    use pamoja_lorawan::{Device, Session, Uplink};

    // One site, on the band it operates in, admitting one device it was told about: its EUI
    // from its label, the application it joins, and the root key it was provisioned with.
    let dev_eui = Eui::from_hex("70b3d57ed0001234")
        .expect("the device's EUI")
        .bytes();
    let join_eui = Eui::from_hex("70b3d57ed0000000")
        .expect("the application's EUI")
        .bytes();
    let app_key = [0x2b; 16];
    let mut site = Network::new(Region::Eu868.plan(), 0x00_00_13).with_first_dev_addr(0x2601_0001);
    site.register(Registration::new(dev_eui, join_eui, app_key));

    // The gateway forwards a join request it heard. Nothing about the device is known here
    // beyond the key, which is what verifies the request, and the accept is timed for the
    // join window, five seconds after the request.
    let eu868 = Region::Eu868.plan();
    let dr5 = eu868.link_settings(5).expect("DR5 is a LoRa data rate");
    let device = Device::new(dev_eui, join_eui, app_key);
    let request = device.join_request(0x0102);
    let heard_at = 1_000_000;
    let heard =
        Rxpk::new(868_100_000, dr5, request.as_bytes().to_vec()).with_timestamp_us(heard_at);
    let Event::Joined {
        dev_addr, accept, ..
    } = site.uplink(&heard)?
    else {
        panic!("a join request is admitted");
    };
    let accepted_at = accept.timestamp_us.expect("the accept is scheduled");
    println!(
        "joined    {dev_addr:#010x}, accepted at {accepted_at} us, {} s after the request",
        (accepted_at - heard_at) / 1_000_000
    );

    // The device reads the accept and sends a reading. The site decrypts it and says where
    // an answer goes: the uplink's own channel, a second after it ended.
    let session = device.accept_join(&accept.payload, 0x0102)?.session();
    let sent = session.encode_uplink(&Uplink::new(0, 2, b"21.5"))?;
    let carried =
        Rxpk::new(868_100_000, dr5, sent.as_bytes().to_vec()).with_timestamp_us(9_000_000);
    let Event::Data {
        fcnt,
        fport,
        payload,
        slot,
        ..
    } = site.uplink(&carried)?
    else {
        panic!("a data frame is read");
    };
    println!(
        "uplink    frame {fcnt} on port {} says {}, answer at {} us on {} Hz",
        fport.expect("a reading has a port"),
        String::from_utf8_lossy(&payload),
        slot.timestamp_us,
        slot.frequency_hz
    );

    // The answer goes out in that window, encrypted with the session the join granted.
    let answer = site.answer(dev_addr, slot, 2, b"ok")?;
    println!(
        "answer    {} bytes at {} us",
        answer.payload.len(),
        answer.timestamp_us.expect("the answer is scheduled")
    );

    // The same frame again, as a replay would send it, is refused: its counter was seen.
    let replayed = site.uplink(&carried).expect_err("a counter is taken once");
    println!("replay    {replayed}");

    // A gateway hears every network in range, and a frame from one this site never granted
    // is reported as another network's rather than refused.
    let elsewhere = Session::new(0x1234_5678, [9; 16], [8; 16]);
    let overheard = elsewhere.encode_uplink(&Uplink::new(0, 1, b"hello"))?;
    let Event::Foreign { dev_addr: theirs } =
        site.uplink(&Rxpk::new(868_300_000, dr5, overheard.as_bytes().to_vec()))?
    else {
        panic!("a frame from another network is reported as one");
    };
    println!("foreign   {theirs:#010x} belongs to another network");
    // ANCHOR_END: network

    assert_eq!(payload, b"21.5");
    assert_eq!(slot.timestamp_us, 10_000_000);
    assert_eq!(theirs, 0x1234_5678);

    Ok(())
}

/// A Basics Station session, played from both sides: the station finds its network server,
/// says what it is, and reports a frame it heard; the server answers it in a receive window;
/// and the station reports that the answer went out.
fn a_station_session_from_both_sides() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: station
    use pamoja_gateway::station::{
        Discovery, Levels, Message, Router, Xtime, DISCOVERY_PATH, PROTOCOL_VERSION,
    };
    use pamoja_gateway::udp::Eui;
    use pamoja_lora::region::Region;
    use pamoja_lorawan::{Downlink, Session, Uplink};

    // The station asks its configured address where its network server is, naming itself.
    // The server reads who asked, and sends it to the websocket its session runs on.
    let station = Eui::from_hex("b827ebfffe010203").expect("sixteen hexadecimal digits");
    let asking = Discovery::new(station).to_json();
    println!("ask       {DISCOVERY_PATH} {asking}");
    let asked = Discovery::from_json(asking.as_bytes())?.router;
    let muxs = Eui::from_hex("0000000000000001").expect("sixteen hexadecimal digits");
    let answer = Router::accepted(asked, muxs, "ws://lns.example.invalid:3001/router").to_json();
    let uri = Router::from_json(answer.as_bytes())?
        .uri
        .expect("an accepted station is sent somewhere");
    println!("open      {uri}");

    // Once the websocket is open the station speaks first, saying what it is.
    let hello = Message::Version {
        station: "pamoja".to_owned(),
        firmware: env!("CARGO_PKG_VERSION").to_owned(),
        package: "pamoja-gateway".to_owned(),
        model: "linux".to_owned(),
        protocol: PROTOCOL_VERSION,
        features: "gps".to_owned(),
    }
    .to_json();
    if let Message::Version {
        station,
        firmware,
        model,
        protocol,
        ..
    } = Message::from_json(hello.as_bytes())?
    {
        println!("version   {station} {firmware} on {model}, protocol {protocol}");
    }

    // The radio hears a node's reading 3512.348611 seconds into the station's first run. A
    // station holds no key, so it splits the frame into the fields the protocol names and
    // lets the server judge them, with its own clock for the moment it arrived.
    let node = Session::new(0x2601_0001, [0x44; 16], [0x55; 16]);
    let frame = node.encode_uplink(&Uplink::new(7, 2, b"21.5"))?;
    let heard_at = Xtime::new(0, 1, 3_512_348_611).expect("in range");
    let levels = Levels {
        rctx: 0,
        xtime: heard_at.value(),
        gpstime: None,
        rssi: -97.0,
        snr: -3.2,
    };
    let updf = Message::heard(frame.as_bytes(), 5, 868_100_000, levels)?.to_json();
    let Message::Uplink {
        dev_addr,
        fcnt,
        fport,
        payload,
        data_rate,
        frequency_hz,
        levels,
        ..
    } = Message::from_json(updf.as_bytes())?
    else {
        panic!("a data frame going up is read as one");
    };
    println!(
        "updf      {dev_addr:#010x} counter {fcnt} on port {}, DR{data_rate}, {} bytes still encrypted",
        fport.expect("a reading has a port"),
        payload.len()
    );

    // The server answers in the receive windows the region gives: the first at the uplink's
    // own rate and channel, the second where the plan fixes it. It hands the station's clock
    // back untouched, so the station can time the answer from the moment it heard the uplink.
    let eu868 = Region::Eu868.plan();
    let rx1_rate = eu868
        .rx1_data_rate(data_rate, 0)
        .expect("DR5 has a first window");
    let (rx2_hz, rx2_rate) = eu868.rx2();
    let reply = node.encode_downlink(&Downlink::new(0, 2, b"ok"))?;
    let dnmsg = Message::Downlink {
        dev_eui: Eui::from_hex("70b3d57ed0001234").expect("the device's EUI"),
        class: 0,
        diid: 1,
        pdu: reply.as_bytes().to_vec(),
        rx_delay: Some(1),
        rx1: Some((rx1_rate, frequency_hz)),
        rx2: Some((rx2_rate, rx2_hz)),
        ping_slot: None,
        priority: 0,
        xtime: Some(levels.xtime),
        rctx: Some(levels.rctx),
        gpstime: None,
    }
    .to_json();
    let Message::Downlink {
        diid,
        dev_eui,
        rx_delay,
        rx1,
        rx2,
        xtime,
        rctx,
        ..
    } = Message::from_json(dnmsg.as_bytes())?
    else {
        panic!("a downlink is read as one");
    };
    let (first_rate, first_hz) = rx1.expect("a first window");
    let (second_rate, second_hz) = rx2.expect("a second window");
    println!(
        "dnmsg     RX1 DR{first_rate} on {first_hz} Hz or RX2 DR{second_rate} on {second_hz} Hz, {} s after the uplink",
        rx_delay.unwrap_or(1)
    );

    // The station opens the first window a second after the uplink on its own clock, puts
    // the answer on the air, and reports it by the identifier the server gave it.
    let uplink_at = Xtime::of(xtime.expect("the uplink's clock came back"));
    let sent_at = Xtime::new(
        uplink_at.unit,
        uplink_at.session,
        uplink_at.micros + u64::from(rx_delay.unwrap_or(1)) * 1_000_000,
    )
    .expect("in range");
    let dntxed = Message::Transmitted {
        diid,
        dev_eui,
        rctx: rctx.unwrap_or(0),
        xtime: sent_at.value(),
        txtime: sent_at.micros as f64 / 1e6,
        gpstime: None,
    }
    .to_json();
    if let Message::Transmitted { diid, xtime, .. } = Message::from_json(dntxed.as_bytes())? {
        let went = Xtime::of(xtime);
        println!(
            "dntxed    downlink {diid} went out at {} us of run {}",
            went.micros, went.session
        );
    }
    // ANCHOR_END: station

    assert_eq!(asked, station);
    assert_eq!(dev_addr, 0x2601_0001);
    assert_eq!(fcnt, 7);
    assert_ne!(payload.as_slice(), b"21.5");
    assert_eq!(sent_at.micros, 3_513_348_611);

    Ok(())
}

/// A network server on the Raspberry Pi the gateway daemon runs on, answering what the daemon
/// forwards over UDP: joins, readings, and the keepalives that hold its downlink path open.
fn a_network_server_beside_the_gateway() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: hardware
    use std::io::ErrorKind;
    use std::net::{SocketAddr, UdpSocket};
    use std::time::Duration;

    use pamoja_gateway::network::{Event, Network, Registration};
    use pamoja_gateway::udp::{CrcStatus, Eui, Packet, DEFAULT_PORT};
    use pamoja_lora::region::Region;

    // The daemon forwards to 127.0.0.1 when it runs on the same Pi. To serve gateways
    // elsewhere, listen on 0.0.0.0 behind a firewall that admits only them: the protocol has
    // no authentication of its own.
    let listen = ("127.0.0.1", DEFAULT_PORT);
    let dev_eui = Eui::from_hex("70b3d57ed0001234")
        .expect("the device's EUI")
        .bytes();
    let join_eui = Eui::from_hex("70b3d57ed0000000")
        .expect("the application's EUI")
        .bytes();
    let mut site = Network::new(Region::Eu868.plan(), 0x00_00_13);
    site.register(Registration::new(dev_eui, join_eui, [0x2b; 16]));

    let Ok(socket) = UdpSocket::bind(listen) else {
        println!("absent    another program holds port {DEFAULT_PORT}");
        return Ok(());
    };

    // A pamoja gateway holds its path open every five seconds, so six seconds of silence
    // means none is running. After that the server keeps answering until a minute passes
    // with nothing heard.
    socket.set_read_timeout(Some(Duration::from_secs(6)))?;
    let mut downlinks: Option<SocketAddr> = None;
    let mut token: u16 = 0;
    let mut buffer = [0u8; 65_535];
    loop {
        let (len, from) = match socket.recv_from(&mut buffer) {
            Ok(arrived) => arrived,
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                if downlinks.is_none() {
                    println!("absent    no gateway reported in, so nothing was answered");
                }
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        socket.set_read_timeout(Some(Duration::from_secs(60)))?;

        let packet = match Packet::parse(&buffer[..len]) {
            Ok(packet) => packet,
            Err(why) => {
                println!("ignored   {why}");
                continue;
            }
        };
        if let Some(ack) = packet.acknowledgment() {
            socket.send_to(&ack.to_bytes(), from)?;
        }

        match packet {
            Packet::PullData { gateway, .. } => {
                if downlinks.replace(from).is_none() {
                    println!("gateway   {gateway} holds its downlink path open");
                }
            }
            Packet::PushData { uplink, .. } => {
                for heard in &uplink.packets {
                    if heard.crc != CrcStatus::Ok {
                        continue;
                    }
                    let transmit = match site.uplink(heard) {
                        Ok(Event::Joined {
                            dev_addr, accept, ..
                        }) => {
                            println!("joined    {dev_addr:#010x}");
                            accept
                        }
                        Ok(Event::Data {
                            dev_addr,
                            payload,
                            slot,
                            ..
                        }) => {
                            println!(
                                "reading   {dev_addr:#010x} says {}",
                                String::from_utf8_lossy(&payload)
                            );
                            site.answer(dev_addr, slot, 2, b"ok")?
                        }
                        Ok(Event::Foreign { .. }) => continue,
                        Err(why) => {
                            println!("refused   {why}");
                            continue;
                        }
                    };
                    if let Some(gateway) = downlinks {
                        token = token.wrapping_add(1);
                        let answer = Packet::PullResp { token, transmit };
                        socket.send_to(&answer.to_bytes(), gateway)?;
                    }
                }
            }
            Packet::TxAck { status, .. } => println!("txack     {status}"),
            _ => {}
        }
    }
    // ANCHOR_END: hardware
}
