# LoRaWAN gateways

A gateway is a radio with an uplink. It hears packets from every node in range,
whatever network they belong to, and hands them to a server that knows what to do
with them. The server hands back the packets to transmit, and the gateway puts them
on the air at the microsecond the device is listening.

The protocol between the two is not LoRaWAN. LoRaWAN is what the packets
themselves carry, encrypted end to end, and the gateway never reads them. What the
gateway speaks is one of two protocols. Semtech's packet forwarder protocol is six
kinds of UDP datagram, deliberately plain, with no authentication and no retries,
which is why it belongs on a private network or inside a tunnel. LoRa Basics
Station replaces it with a conversation over a websocket: the station asks where
its server is, says what it is, and exchanges JSON messages that name their kind.
`pamoja-gateway` speaks both, from both sides, so the same types build a gateway and
the server it talks to. It also holds the network side of one site: it admits a
device's join, decrypts what the device sends, refuses a replay, and works out when
and where to answer.

## What the example does

It has four parts.

The first is the packet forwarder protocol, both ends of it. A gateway on a
Raspberry Pi holds its downlink path open with a PULL_DATA and is answered. It
forwards a node's reading, heard at SF9 near the edge of its range, in a PUSH_DATA
that also carries its own counts, and the server acknowledges it before reading
anything in it. The server sends an answer back in a PULL_RESP, timed for the
device's first receive window, and the gateway reports on it twice: once scheduled,
once too late.

The second is the network side of one site. A device's join request arrives, the
site verifies it with the device's key and times the accept for the join window. The
device's first reading arrives and is decrypted, and the answer is timed a second
later. The same frame played again is refused as a replay, and a frame from another
network is reported as such rather than refused.

The third is a Basics Station session from both sides. The station asks where its
server is and is sent to a websocket, says what it is, and reports a frame it heard
with its own clock. The server answers in the two receive windows the region gives,
handing the station's clock back, and the station reports the moment the answer
went out.

The fourth is a network server on the Raspberry Pi the gateway runs on, answering
the pamoja gateway daemon over UDP. With no gateway running it waits six seconds,
says nobody reported in, and stops.

It proves:

- A PULL_DATA is twelve bytes, the four every datagram starts with and the gateway's
  eight-byte identifier, and the PULL_ACK that holds the path open is four.
- A frame crosses as bytes, a frequency in hertz, and the datarate identifier
  `SF9BW125` as the link settings the airtime and range math takes. The RSSI comes
  to the decibel and the SNR to a tenth, as the protocol carries them.
- A PUSH_DATA is acknowledged by its token in four bytes, and the gateway's counts
  ride along with the packets.
- A downlink is timed in the concentrator's own microseconds, a second after the
  uplink's timestamp, with the inverted IQ a LoRaWAN device listens for.
- A TX_ACK says what became of a downlink in the protocol's own words: `NONE` for
  scheduled, and a reason such as `TOO_LATE` for anything refused.
- A join accept goes out five seconds after the request, and an answer to a reading
  one second after it, the delays RP002-1.0.5 section 3.3 recommends for every
  region.
- A frame at a counter the site has already taken is refused with the counter and
  the address, and a frame for an address no session covers is another network's,
  reported rather than refused.
- A station names itself in the ID6 form, a server that reads the request answers
  with the websocket, and a station reports a frame with its payload still
  encrypted, since it holds no key.
- The server's answer names both windows from the region's plan: the first at the
  uplink's own data rate and channel, DR5 on 868.1 MHz, the second at DR0 on
  869.525 MHz for EU863-870.
- A station clock survives the round trip exactly, and the answer goes out a second
  after the uplink on that clock.
- A server with no gateway reporting in says so after six seconds rather than
  waiting forever.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example gateway" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example gateway</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- gateway" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- gateway</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/gateway.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/gateway.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- gateway" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- gateway</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-gateway` holds each protocol in its own module. `udp::Packet` builds
and parses the six datagrams, carrying an `Rxpk`, a `Stat`, a `Txpk`, and a
`TxStatus`, and an `Eui` names the gateway. `network::Network` is the site: it
takes an `Rxpk` and answers with an `Event`, a join, a reading, or another network's
frame, and `answer` builds the downlink. `station` holds Basics Station:
`Discovery`, `Router`, every `Message` kind, the `Levels` a frame was heard at, and
`Xtime`, the station's clock taken apart. Nothing here opens a socket, so each
refusal comes back as a `Result`: a `ProtocolError` for a datagram or message that is
not the protocol, and a `NetworkError` for a frame the site will not take. The
`daemon` feature adds the gateway itself, which drives a concentrator; the
[gateway board](../boards/gateway.md) page sets it up.

<!-- snippet: examples/guides/gateway.rs#example -->
From [`examples/guides/gateway.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/gateway.rs):

```rust
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
```
<!-- end -->

The network side of the same site, which admits the device and answers it:

<!-- snippet: examples/guides/gateway.rs#network -->
From [`examples/guides/gateway.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/gateway.rs):

```rust
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
```
<!-- end -->

The same exchange over Basics Station, from the station's side and the server's:

<!-- snippet: examples/guides/gateway.rs#station -->
From [`examples/guides/gateway.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/gateway.rs):

```rust
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
```
<!-- end -->

And on a board, the network server on the Raspberry Pi the gateway daemon runs on:

<!-- snippet: examples/guides/gateway.rs#hardware -->
From [`examples/guides/gateway.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/gateway.rs):

```rust
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
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/gateway` builds a datagram from a plain object with `encode`
and reads one with `parse`, naming its kind with `PacketKind`; `acknowledgment`
returns the answer a datagram is owed, and `TxStatus` holds the protocol's words.
`Network` is the site. Basics Station is a set of functions: `stationEncode` writes
a message of any kind from an object with a `kind`, `stationParse` reads one, and
`stationHeard`, `stationDiscovery`, `stationRouterAccepted`, and `stationXtime` build
the rest. A station clock and a downlink's identifier are bigints, because the
session byte an xtime carries puts it past what a number holds exactly. A refusal
throws an `Error` carrying the core's reason.

<!-- snippet: bindings/node/guides/gateway.ts#example -->
From [`bindings/node/guides/gateway.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gateway.ts):

```typescript
import { PacketKind, TxStatus, acknowledgment, encode, parse } from '@pamoja/gateway'
import { link } from '@pamoja/lora'
import { session } from '@pamoja/lorawan'

function aGatewayAndItsServerTradeDatagrams() {
  // A gateway on a Raspberry Pi, whose identifier is written from its network interface.
  const gateway = 'b827ebfffe010203'

  // Every few seconds it sends a PULL_DATA, which holds a path open through whatever
  // translates its address, so the server has somewhere to send a downlink. The server
  // answers each one, and a gateway that stops hearing answers knows the path is gone.
  const pull = encode({ kind: PacketKind.PullData, token: 0x7a01, gateway })
  const held = acknowledgment(parse(pull))!
  console.log(
    `pull      ${pull.length} bytes out and ${encode(held).length} back hold the downlink path open`,
  )

  // A node sends a reading, and the gateway hears it on 868.1 MHz at SF9, near the edge of
  // its range. It forwards the frame as it arrived, with the levels, the concentrator's own
  // timestamp, and its counts since the last report. It holds no key and reads none of it.
  const node = session(0x26010001, Buffer.alloc(16, 0x44), Buffer.alloc(16, 0x55))
  const frame = node.encodeUplink(7, 2, Buffer.from('21.5'))
  const heard = {
    frequencyHz: 868_100_000,
    payload: frame,
    link: link(9, 125_000),
    rssiDbm: -97,
    snrDb: -3.2,
    timestampUs: 3_512_348_611,
  }
  const counts = {
    received: 2,
    receivedOk: 1,
    forwarded: 1,
    acknowledgedPercent: 100,
    downlinks: 0,
    transmitted: 0,
  }
  const datagram = encode({
    kind: PacketKind.PushData,
    token: 0x1234,
    gateway,
    packets: [heard],
    status: counts,
  })
  console.log(
    `push      a reading and the gateway's counts, ${datagram.length} bytes, ` +
      `token ${(0x1234).toString(16).padStart(4, '0')}`,
  )

  // The server reads it. The frequency is in hertz, the datarate identifier is the link
  // settings, and the payload is bytes, so nothing is decoded by hand.
  const forwarded = parse(datagram)
  const received = forwarded.packets![0]
  console.log(
    `heard     ${received.frequencyHz} Hz at SF${received.link!.spreadingFactor}, ` +
      `${received.link!.bandwidthHz / 1000} kHz, ${received.rssiDbm!.toFixed(0)} dBm, ` +
      `SNR ${received.snrDb!.toFixed(1)} dB, CRC ${received.crc!.toLowerCase()}, ` +
      `${received.payload.length} bytes`,
  )
  const report = forwarded.status!
  console.log(
    `counts    ${report.received} received, ${report.receivedOk} with a good CRC, ` +
      `${report.forwarded} forwarded, ${report.acknowledgedPercent!.toFixed(1)}% acknowledged`,
  )

  // It is acknowledged at once, by token, before anything in it is read.
  const ack = acknowledgment(parse(datagram))!
  console.log(
    `ack       token ${ack.token.toString(16).padStart(4, '0')} acknowledged in ` +
      `${encode(ack).length} bytes`,
  )

  // An answer goes back in a PULL_RESP, timed in the concentrator's own microseconds for
  // the device's first receive window, a second after the uplink ended, with the inverted
  // polarity a LoRaWAN device listens for.
  const answer = node.encodeDownlink(0, 2, Buffer.from('ok'))
  const pullResp = encode({
    kind: PacketKind.PullResp,
    token: 0x00ab,
    transmit: {
      frequencyHz: 868_100_000,
      payload: answer,
      link: link(9, 125_000),
      timestampUs: 3_513_348_611,
      powerDbm: 14,
      invertPolarity: true,
    },
  })
  const transmit = parse(pullResp).transmit!
  const iq = transmit.invertPolarity ? 'IQ inverted' : 'IQ upright'
  console.log(
    `downlink  at ${transmit.timestampUs} us on ${transmit.frequencyHz} Hz, ` +
      `${transmit.powerDbm} dBm, ${iq}`,
  )

  // The gateway answers each PULL_RESP with a TX_ACK saying what became of it: scheduled,
  // or refused with a reason, such as a window that had already passed.
  for (const said of [TxStatus.None, TxStatus.TooLate]) {
    const reported = encode({ kind: PacketKind.TxAck, token: 0x00ab, gateway, txStatus: said })
    const status = parse(reported).txStatus!
    const meaning = status === TxStatus.None ? "it goes out in the device's window" : 'it was not sent'
    console.log(`txack     ${status}: ${meaning}`)
  }
  return { held, frame, received, ack, transmit }
}
```
<!-- end -->

The network side of the same site:

<!-- snippet: bindings/node/guides/gateway.ts#network -->
From [`bindings/node/guides/gateway.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gateway.ts):

```typescript
import { Network } from '@pamoja/gateway'
import { LoraRegion, planFor } from '@pamoja/lora'
import { device } from '@pamoja/lorawan'

function aDeviceJoinsASiteAndIsAnswered() {
  // One site, on the band it operates in, admitting one device it was told about: its EUI
  // from its label, the application it joins, and the root key it was provisioned with.
  const devEui = Buffer.from('70b3d57ed0001234', 'hex')
  const joinEui = Buffer.from('70b3d57ed0000000', 'hex')
  const appKey = Buffer.alloc(16, 0x2b)
  const site = new Network(planFor(LoraRegion.Eu868), 0x000013, null, 0x26010001)
  site.register(devEui, joinEui, appKey)

  // The gateway forwards a join request it heard. Nothing about the device is known here
  // beyond the key, which is what verifies the request, and the accept is timed for the
  // join window, five seconds after the request.
  const eu868 = planFor(LoraRegion.Eu868)
  const dr5 = eu868.linkSettings(5)!
  const joiner = device(devEui, joinEui, appKey)
  const heardAt = 1_000_000
  const joined = site.uplink({
    frequencyHz: 868_100_000,
    payload: joiner.joinRequest(0x0102),
    link: dr5,
    timestampUs: heardAt,
  })
  const acceptedAt = joined.accept!.timestampUs!
  console.log(
    `joined    0x${joined.devAddr.toString(16).padStart(8, '0')}, accepted at ${acceptedAt} us, ` +
      `${(acceptedAt - heardAt) / 1_000_000} s after the request`,
  )

  // The device reads the accept and sends a reading. The site decrypts it and says where
  // an answer goes: the uplink's own channel, a second after it ended.
  const granted = joiner.acceptJoin(joined.accept!.payload, 0x0102).session()
  const carried = {
    frequencyHz: 868_100_000,
    payload: granted.encodeUplink(0, 2, Buffer.from('21.5')),
    link: dr5,
    timestampUs: 9_000_000,
  }
  const reading = site.uplink(carried)
  console.log(
    `uplink    frame ${reading.fcnt} on port ${reading.fport} says ${reading.payload!.toString()}, ` +
      `answer at ${reading.slot!.timestampUs} us on ${reading.slot!.frequencyHz} Hz`,
  )

  // The answer goes out in that window, encrypted with the session the join granted.
  const answer = site.answer(reading.devAddr, reading.slot!, 2, Buffer.from('ok'))
  console.log(`answer    ${answer.payload.length} bytes at ${answer.timestampUs} us`)

  // The same frame again, as a replay would send it, is refused: its counter was seen.
  try {
    site.uplink(carried)
    assert.fail('a counter is taken once')
  } catch (refused) {
    console.log(`replay    ${(refused as Error).message}`)
  }

  // A gateway hears every network in range, and a frame from one this site never granted
  // is reported as another network's rather than refused.
  const elsewhere = session(0x12345678, Buffer.alloc(16, 0x09), Buffer.alloc(16, 0x08))
  const stranger = site.uplink({
    frequencyHz: 868_300_000,
    payload: elsewhere.encodeUplink(0, 1, Buffer.from('hello')),
    link: dr5,
  })
  console.log(
    `foreign   0x${stranger.devAddr.toString(16).padStart(8, '0')} belongs to another network`,
  )
  return { reading, stranger }
}
```
<!-- end -->

The Basics Station session:

<!-- snippet: bindings/node/guides/gateway.ts#station -->
From [`bindings/node/guides/gateway.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gateway.ts):

```typescript
import {
  DISCOVERY_PATH,
  STATION_PROTOCOL_VERSION,
  StationKind,
  stationDiscovery,
  stationDiscoveryParse,
  stationEncode,
  stationHeard,
  stationParse,
  stationRouterAccepted,
  stationRouterParse,
  stationXtime,
  stationXtimeParts,
} from '@pamoja/gateway'
import { version } from '@pamoja/core'

function aStationSessionFromBothSides() {
  // The station asks its configured address where its network server is, naming itself.
  // The server reads who asked, and sends it to the websocket its session runs on.
  const station = 'b827ebfffe010203'
  const asking = stationDiscovery(station)
  console.log(`ask       ${DISCOVERY_PATH} ${asking}`)
  const asked = stationDiscoveryParse(asking)
  const answer = stationRouterAccepted(
    asked,
    '0000000000000001',
    'ws://lns.example.invalid:3001/router',
  )
  console.log(`open      ${stationRouterParse(answer).uri}`)

  // Once the websocket is open the station speaks first, saying what it is.
  const hello = stationEncode({
    kind: StationKind.Version,
    station: 'pamoja',
    firmware: version(),
    package: 'pamoja-gateway',
    model: 'linux',
    protocol: STATION_PROTOCOL_VERSION,
    features: 'gps',
  })
  const said = stationParse(hello)
  console.log(`version   ${said.station} ${said.firmware} on ${said.model}, protocol ${said.protocol}`)

  // The radio hears a node's reading 3512.348611 seconds into the station's first run. A
  // station holds no key, so it splits the frame into the fields the protocol names and
  // lets the server judge them, with its own clock for the moment it arrived.
  const node = session(0x26010001, Buffer.alloc(16, 0x44), Buffer.alloc(16, 0x55))
  const frame = node.encodeUplink(7, 2, Buffer.from('21.5'))
  const heardAt = stationXtime(0, 1, 3_512_348_611)
  const updf = stationEncode(
    stationHeard(frame, 5, 868_100_000, { rctx: 0, xtime: heardAt, rssi: -97, snr: -3.2 }),
  )
  const uplink = stationParse(updf)
  console.log(
    `updf      0x${(uplink.devAddr! >>> 0).toString(16).padStart(8, '0')} counter ${uplink.fcnt} ` +
      `on port ${uplink.fport}, DR${uplink.dataRate}, ${uplink.payload!.length} bytes still encrypted`,
  )

  // The server answers in the receive windows the region gives: the first at the uplink's
  // own rate and channel, the second where the plan fixes it. It hands the station's clock
  // back untouched, so the station can time the answer from the moment it heard the uplink.
  const eu868 = planFor(LoraRegion.Eu868)
  const rx1Rate = eu868.rx1DataRate(uplink.dataRate!, 0)!
  const rx2 = eu868.rx2()
  const dnmsg = stationEncode({
    kind: StationKind.Downlink,
    devEui: '70b3d57ed0001234',
    class: 0,
    diid: 1n,
    pdu: node.encodeDownlink(0, 2, Buffer.from('ok')),
    rxDelay: 1,
    rx1: { dataRate: rx1Rate, frequencyHz: uplink.frequencyHz! },
    rx2: { dataRate: rx2.dataRate, frequencyHz: rx2.frequencyHz },
    priority: 0,
    xtime: uplink.levels!.xtime,
    rctx: uplink.levels!.rctx,
  })
  const told = stationParse(dnmsg)
  const delay = told.rxDelay ?? 1
  console.log(
    `dnmsg     RX1 DR${told.rx1!.dataRate} on ${told.rx1!.frequencyHz} Hz or ` +
      `RX2 DR${told.rx2!.dataRate} on ${told.rx2!.frequencyHz} Hz, ${delay} s after the uplink`,
  )

  // The station opens the first window a second after the uplink on its own clock, puts
  // the answer on the air, and reports it by the identifier the server gave it.
  const uplinkAt = stationXtimeParts(told.xtime!)
  const sentAt = stationXtime(uplinkAt.unit, uplinkAt.session, uplinkAt.micros + delay * 1_000_000)
  const dntxed = stationEncode({
    kind: StationKind.Transmitted,
    diid: told.diid,
    devEui: told.devEui,
    rctx: told.rctx ?? 0,
    xtime: sentAt,
    txtime: stationXtimeParts(sentAt).micros / 1e6,
  })
  const reported = stationParse(dntxed)
  const went = stationXtimeParts(reported.xtime!)
  console.log(`dntxed    downlink ${reported.diid} went out at ${went.micros} us of run ${went.session}`)
  return { asked, station, uplink, went }
}
```
<!-- end -->

And the network server beside the gateway daemon, on `node:dgram`:

<!-- snippet: bindings/node/guides/gateway.ts#hardware -->
From [`bindings/node/guides/gateway.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gateway.ts):

```typescript
import { createSocket } from 'node:dgram'
import { DEFAULT_PORT, Crc } from '@pamoja/gateway'

async function aNetworkServerBesideTheGateway(): Promise<void> {
  // The daemon forwards to 127.0.0.1 when it runs on the same Pi. To serve gateways
  // elsewhere, listen on 0.0.0.0 behind a firewall that admits only them: the protocol has
  // no authentication of its own.
  const site = new Network(planFor(LoraRegion.Eu868), 0x000013)
  site.register(
    Buffer.from('70b3d57ed0001234', 'hex'),
    Buffer.from('70b3d57ed0000000', 'hex'),
    Buffer.alloc(16, 0x2b),
  )

  const socket = createSocket('udp4')
  const bound = await new Promise<boolean>((resolve) => {
    socket.once('error', () => resolve(false))
    socket.bind(DEFAULT_PORT, '127.0.0.1', () => resolve(true))
  })
  if (!bound) {
    console.log(`absent    another program holds port ${DEFAULT_PORT}`)
    return
  }

  // A pamoja gateway holds its path open every five seconds, so six seconds of silence
  // means none is running. After that the server keeps answering until a minute passes
  // with nothing heard.
  let downlinks: { address: string; port: number } | null = null
  let token = 0
  const next = (waitMs: number) =>
    new Promise<{ datagram: Buffer; from: { address: string; port: number } } | null>((resolve) => {
      const timer = setTimeout(() => {
        socket.removeAllListeners('message')
        resolve(null)
      }, waitMs)
      socket.once('message', (datagram, from) => {
        clearTimeout(timer)
        resolve({ datagram, from })
      })
    })

  for (let arrived = await next(6_000); arrived; arrived = await next(60_000)) {
    const { datagram, from } = arrived
    let packet
    try {
      packet = parse(datagram)
    } catch (why) {
      console.log(`ignored   ${(why as Error).message}`)
      continue
    }
    const ack = acknowledgment(packet)
    if (ack) {
      socket.send(encode(ack), from.port, from.address)
    }

    if (packet.kind === PacketKind.PullData) {
      if (!downlinks) {
        console.log(`gateway   ${packet.gateway} holds its downlink path open`)
      }
      downlinks = from
    } else if (packet.kind === PacketKind.PushData) {
      for (const heard of packet.packets ?? []) {
        if (heard.crc !== Crc.Ok) {
          continue
        }
        let transmit
        try {
          const event = site.uplink(heard)
          if (event.outcome === 'Joined') {
            console.log(`joined    0x${event.devAddr.toString(16).padStart(8, '0')}`)
            transmit = event.accept!
          } else if (event.outcome === 'Data') {
            console.log(
              `reading   0x${event.devAddr.toString(16).padStart(8, '0')} says ${event.payload!.toString()}`,
            )
            transmit = site.answer(event.devAddr, event.slot!, 2, Buffer.from('ok'))
          } else {
            continue
          }
        } catch (why) {
          console.log(`refused   ${(why as Error).message}`)
          continue
        }
        if (downlinks) {
          token = (token + 1) & 0xffff
          socket.send(
            encode({ kind: PacketKind.PullResp, token, transmit }),
            downlinks.port,
            downlinks.address,
          )
        }
      }
    } else if (packet.kind === PacketKind.TxAck) {
      console.log(`txack     ${packet.txStatus}`)
    }
  }
  if (!downlinks) {
    console.log('absent    no gateway reported in, so nothing was answered')
  }
  socket.close()
}
```
<!-- end -->

## Python

In Python, `pamoja.gateway` has a class for each thing a datagram carries,
`Packet`, `Rxpk`, `Stat`, and `Txpk`, with `encode`, `parse`, and `acknowledgment`
over them; `PacketKind`, `TxStatus`, and `Crc` are string enums of the protocol's
names. `Network` is the site. A Basics Station message of any kind is a
`StationMessage` built with the keyword arguments its kind uses and written with
`station_encode`, beside `station_heard`, `station_router_accepted`, and
`station_xtime`. A refusal by the core raises `PamojaError`, and an argument of the
wrong shape, such as an identifier that is not eight bytes, raises `ValueError`.

<!-- snippet: bindings/python/guides/gateway.py#example -->
From [`bindings/python/guides/gateway.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gateway.py):

```python
from pamoja.gateway import (
    Packet,
    PacketKind,
    Rxpk,
    Stat,
    Txpk,
    TxStatus,
    acknowledgment,
    encode,
    parse,
)
from pamoja.lora import link
from pamoja.lorawan import session

# A gateway on a Raspberry Pi, whose identifier is written from its network interface.
gateway = "b827ebfffe010203"

# Every few seconds it sends a PULL_DATA, which holds a path open through whatever translates
# its address, so the server has somewhere to send a downlink. The server answers each one,
# and a gateway that stops hearing answers knows the path is gone.
pull = encode(Packet(PacketKind.PULL_DATA, 0x7A01, gateway=gateway))
held = acknowledgment(parse(pull))
print(f"pull      {len(pull)} bytes out and {len(encode(held))} back hold the downlink path open")

# A node sends a reading, and the gateway hears it on 868.1 MHz at SF9, near the edge of its
# range. It forwards the frame as it arrived, with the levels, the concentrator's own
# timestamp, and its counts since the last report. It holds no key and reads none of it.
node = session(0x26010001, bytes([0x44]) * 16, bytes([0x55]) * 16)
frame = node.encode_uplink(7, 2, b"21.5")
heard = Rxpk(
    868_100_000,
    frame,
    link=link(9, 125_000),
    rssi_dbm=-97,
    snr_db=-3.2,
    timestamp_us=3_512_348_611,
)
counts = Stat(received=2, received_ok=1, forwarded=1, acknowledged_percent=100.0)
datagram = encode(Packet(PacketKind.PUSH_DATA, 0x1234, gateway=gateway, packets=[heard], status=counts))
print(f"push      a reading and the gateway's counts, {len(datagram)} bytes, token {0x1234:04x}")

# The server reads it. The frequency is in hertz, the datarate identifier is the link
# settings, and the payload is bytes, so nothing is decoded by hand.
forwarded = parse(datagram)
received = forwarded.packets[0]
print(
    f"heard     {received.frequency_hz} Hz at SF{received.link.spreading_factor}, "
    f"{received.link.bandwidth_hz // 1000} kHz, {received.rssi_dbm:.0f} dBm, "
    f"SNR {received.snr_db:.1f} dB, CRC {received.crc.lower()}, {len(received.payload)} bytes"
)
report = forwarded.status
print(
    f"counts    {report.received} received, {report.received_ok} with a good CRC, "
    f"{report.forwarded} forwarded, {report.acknowledged_percent:.1f}% acknowledged"
)

# It is acknowledged at once, by token, before anything in it is read.
ack = acknowledgment(parse(datagram))
print(f"ack       token {ack.token:04x} acknowledged in {len(encode(ack))} bytes")

# An answer goes back in a PULL_RESP, timed in the concentrator's own microseconds for the
# device's first receive window, a second after the uplink ended, with the inverted polarity
# a LoRaWAN device listens for.
answer = node.encode_downlink(0, 2, b"ok")
window = Txpk(
    868_100_000,
    answer,
    link=link(9, 125_000),
    timestamp_us=3_513_348_611,
    power_dbm=14,
    invert_polarity=True,
)
transmit = parse(encode(Packet(PacketKind.PULL_RESP, 0x00AB, transmit=window))).transmit
iq = "IQ inverted" if transmit.invert_polarity else "IQ upright"
print(
    f"downlink  at {transmit.timestamp_us} us on {transmit.frequency_hz} Hz, "
    f"{transmit.power_dbm} dBm, {iq}"
)

# The gateway answers each PULL_RESP with a TX_ACK saying what became of it: scheduled, or
# refused with a reason, such as a window that had already passed.
for said in (TxStatus.NONE, TxStatus.TOO_LATE):
    reported = encode(Packet(PacketKind.TX_ACK, 0x00AB, gateway=gateway, tx_status=said))
    status = parse(reported).tx_status
    meaning = "it goes out in the device's window" if status == TxStatus.NONE else "it was not sent"
    print(f"txack     {status}: {meaning}")
```
<!-- end -->

The network side of the same site:

<!-- snippet: bindings/python/guides/gateway.py#network -->
From [`bindings/python/guides/gateway.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gateway.py):

```python
from pamoja.core import PamojaError
from pamoja.gateway import Network
from pamoja.lora import plan_for
from pamoja.lorawan import device

# One site, on the band it operates in, admitting one device it was told about: its EUI from
# its label, the application it joins, and the root key it was provisioned with.
dev_eui = bytes.fromhex("70b3d57ed0001234")
join_eui = bytes.fromhex("70b3d57ed0000000")
app_key = bytes([0x2B]) * 16
site = Network(plan_for("EU868"), 0x000013, first_dev_addr=0x26010001)
site.register(dev_eui, join_eui, app_key)

# The gateway forwards a join request it heard. Nothing about the device is known here beyond
# the key, which is what verifies the request, and the accept is timed for the join window,
# five seconds after the request.
eu868 = plan_for("EU868")
dr5 = eu868.link_settings(5)
joiner = device(dev_eui, join_eui, app_key)
heard_at = 1_000_000
joined = site.uplink(
    Rxpk(868_100_000, joiner.join_request(0x0102), link=dr5, timestamp_us=heard_at)
)
accepted_at = joined.accept.timestamp_us
print(
    f"joined    {joined.dev_addr:#010x}, accepted at {accepted_at} us, "
    f"{(accepted_at - heard_at) // 1_000_000} s after the request"
)

# The device reads the accept and sends a reading. The site decrypts it and says where an
# answer goes: the uplink's own channel, a second after it ended.
granted = joiner.accept_join(joined.accept.payload, 0x0102).session()
carried = Rxpk(
    868_100_000,
    granted.encode_uplink(0, 2, b"21.5"),
    link=dr5,
    timestamp_us=9_000_000,
)
reading = site.uplink(carried)
print(
    f"uplink    frame {reading.fcnt} on port {reading.fport} says {reading.payload.decode()}, "
    f"answer at {reading.slot.timestamp_us} us on {reading.slot.frequency_hz} Hz"
)

# The answer goes out in that window, encrypted with the session the join granted.
reply = site.answer(reading.dev_addr, reading.slot, 2, b"ok")
print(f"answer    {len(reply.payload)} bytes at {reply.timestamp_us} us")

# The same frame again, as a replay would send it, is refused: its counter was seen.
try:
    site.uplink(carried)
    raise AssertionError("a counter is taken once")
except PamojaError as refused:
    print(f"replay    {refused}")

# A gateway hears every network in range, and a frame from one this site never granted is
# reported as another network's rather than refused.
elsewhere = session(0x12345678, bytes([0x09]) * 16, bytes([0x08]) * 16)
stranger = site.uplink(
    Rxpk(868_300_000, elsewhere.encode_uplink(0, 1, b"hello"), link=dr5)
)
print(f"foreign   {stranger.dev_addr:#010x} belongs to another network")
```
<!-- end -->

The Basics Station session:

<!-- snippet: bindings/python/guides/gateway.py#station -->
From [`bindings/python/guides/gateway.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gateway.py):

```python
from pamoja.core import version
from pamoja.gateway import (
    DISCOVERY_PATH,
    STATION_PROTOCOL_VERSION,
    StationKind,
    StationLevels,
    StationMessage,
    StationWindow,
    station_discovery,
    station_discovery_parse,
    station_encode,
    station_heard,
    station_parse,
    station_router_accepted,
    station_router_parse,
    station_xtime,
    station_xtime_parts,
)

# The station asks its configured address where its network server is, naming itself. The
# server reads who asked, and sends it to the websocket its session runs on.
station = "b827ebfffe010203"
asking = station_discovery(station)
print(f"ask       {DISCOVERY_PATH} {asking}")
asked = station_discovery_parse(asking)
answer = station_router_accepted(asked, "0000000000000001", "ws://lns.example.invalid:3001/router")
print(f"open      {station_router_parse(answer).uri}")

# Once the websocket is open the station speaks first, saying what it is.
hello = station_encode(
    StationMessage(
        StationKind.VERSION,
        station="pamoja",
        firmware=version(),
        package="pamoja-gateway",
        model="linux",
        protocol=STATION_PROTOCOL_VERSION,
        features="gps",
    )
)
said = station_parse(hello)
print(f"version   {said.station} {said.firmware} on {said.model}, protocol {said.protocol}")

# The radio hears a node's reading 3512.348611 seconds into the station's first run. A
# station holds no key, so it splits the frame into the fields the protocol names and lets the
# server judge them, with its own clock for the moment it arrived.
heard_at = station_xtime(0, 1, 3_512_348_611)
updf = station_encode(
    station_heard(
        node.encode_uplink(7, 2, b"21.5"),
        5,
        868_100_000,
        StationLevels(rctx=0, xtime=heard_at, rssi=-97.0, snr=-3.2),
    )
)
uplink = station_parse(updf)
print(
    f"updf      {uplink.dev_addr:#010x} counter {uplink.fcnt} on port {uplink.fport}, "
    f"DR{uplink.data_rate}, {len(uplink.payload)} bytes still encrypted"
)

# The server answers in the receive windows the region gives: the first at the uplink's own
# rate and channel, the second where the plan fixes it. It hands the station's clock back
# untouched, so the station can time the answer from the moment it heard the uplink.
rx1_rate = eu868.rx1_data_rate(uplink.data_rate, 0, False)
rx2_hz, rx2_rate = eu868.rx2()
dnmsg = station_encode(
    StationMessage(
        StationKind.DOWNLINK,
        dev_eui="70b3d57ed0001234",
        class_=0,
        diid=1,
        payload=node.encode_downlink(0, 2, b"ok"),
        rx_delay=1,
        rx1=StationWindow(rx1_rate, uplink.frequency_hz),
        rx2=StationWindow(rx2_rate, rx2_hz),
        priority=0,
        xtime=uplink.levels.xtime,
        rctx=uplink.levels.rctx,
    )
)
told = station_parse(dnmsg)
delay = told.rx_delay or 1
print(
    f"dnmsg     RX1 DR{told.rx1.data_rate} on {told.rx1.frequency_hz} Hz or "
    f"RX2 DR{told.rx2.data_rate} on {told.rx2.frequency_hz} Hz, {delay} s after the uplink"
)

# The station opens the first window a second after the uplink on its own clock, puts the
# answer on the air, and reports it by the identifier the server gave it.
uplink_at = station_xtime_parts(told.xtime)
sent_at = station_xtime(uplink_at.unit, uplink_at.session, uplink_at.micros + delay * 1_000_000)
dntxed = station_encode(
    StationMessage(
        StationKind.TRANSMITTED,
        diid=told.diid,
        dev_eui=told.dev_eui,
        rctx=told.rctx or 0,
        xtime=sent_at,
        txtime=station_xtime_parts(sent_at).micros / 1e6,
    )
)
reported = station_parse(dntxed)
went = station_xtime_parts(reported.xtime)
print(f"dntxed    downlink {reported.diid} went out at {went.micros} us of run {went.session}")
```
<!-- end -->

And the network server beside the gateway daemon, on a plain `socket`:

<!-- snippet: bindings/python/guides/gateway.py#hardware -->
From [`bindings/python/guides/gateway.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gateway.py):

```python
import socket as udp

from pamoja.gateway import DEFAULT_PORT, Crc


def serve_beside_the_gateway():
    """Answers what a gateway daemon on this Raspberry Pi forwards, until it goes quiet."""
    # The daemon forwards to 127.0.0.1 when it runs on the same Pi. To serve gateways
    # elsewhere, listen on 0.0.0.0 behind a firewall that admits only them: the protocol has
    # no authentication of its own.
    site = Network(plan_for("EU868"), 0x000013)
    site.register(
        bytes.fromhex("70b3d57ed0001234"),
        bytes.fromhex("70b3d57ed0000000"),
        bytes([0x2B]) * 16,
    )

    listener = udp.socket(udp.AF_INET, udp.SOCK_DGRAM)
    try:
        listener.bind(("127.0.0.1", DEFAULT_PORT))
    except OSError:
        print(f"absent    another program holds port {DEFAULT_PORT}")
        return

    # A pamoja gateway holds its path open every five seconds, so six seconds of silence
    # means none is running. After that the server keeps answering until a minute passes
    # with nothing heard.
    listener.settimeout(6)
    downlinks = None
    token = 0
    with listener:
        while True:
            try:
                datagram, sender = listener.recvfrom(65_535)
            except TimeoutError:
                break
            listener.settimeout(60)

            try:
                packet = parse(datagram)
            except PamojaError as why:
                print(f"ignored   {why}")
                continue
            ack = acknowledgment(packet)
            if ack is not None:
                listener.sendto(encode(ack), sender)

            if packet.kind == PacketKind.PULL_DATA:
                if downlinks is None:
                    print(f"gateway   {packet.gateway} holds its downlink path open")
                downlinks = sender
            elif packet.kind == PacketKind.PUSH_DATA:
                for arrived in packet.packets:
                    if arrived.crc != Crc.OK:
                        continue
                    try:
                        event = site.uplink(arrived)
                    except PamojaError as why:
                        print(f"refused   {why}")
                        continue
                    if event.outcome == "joined":
                        print(f"joined    {event.dev_addr:#010x}")
                        transmit = event.accept
                    elif event.outcome == "data":
                        print(f"reading   {event.dev_addr:#010x} says {event.payload.decode()}")
                        transmit = site.answer(event.dev_addr, event.slot, 2, b"ok")
                    else:
                        continue
                    if downlinks is not None:
                        token = (token + 1) & 0xFFFF
                        resp = Packet(PacketKind.PULL_RESP, token, transmit=transmit)
                        listener.sendto(encode(resp), downlinks)
            elif packet.kind == PacketKind.TX_ACK:
                print(f"txack     {packet.tx_status}")

    if downlinks is None:
        print("absent    no gateway reported in, so nothing was answered")


serve_beside_the_gateway()
```
<!-- end -->

## C#

In C#, `Pamoja.Gateway` holds a datagram as a `GatewayPacket` record, with
`GatewayRxpk`, `GatewayStat`, and `GatewayTxpk` beside it, and `Gateway.Encode`,
`Gateway.Parse`, and `Gateway.Acknowledgment` over them; `Gateway.NameOf` gives a
`GatewayTxStatus` its protocol word. `GatewayNetwork` is the site. A Basics Station
message of any kind is a `GatewayStationMessage` record written with
`GatewayStation.Encode`, and `GatewayStation` has the rest: `Heard`, `Parse`,
`Discovery`, `DiscoveryParse`, `RouterAccepted`, and `Xtime`. A refusal by the core
throws `PamojaException`, and an argument the library cannot take throws
`ArgumentException` or `ArgumentOutOfRangeException` before anything crosses into
native code.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs):

```csharp
// A gateway on a Raspberry Pi, whose identifier is written from its network interface.
const string GatewayEui = "b827ebfffe010203";

// Every few seconds it sends a PULL_DATA, which holds a path open through whatever
// translates its address, so the server has somewhere to send a downlink. The server
// answers each one, and a gateway that stops hearing answers knows the path is gone.
byte[] pull = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PullData, 0x7A01)
{
    GatewayEui = GatewayEui,
});
GatewayPacket held = Gateway.Acknowledgment(Gateway.Parse(pull))!;
Console.WriteLine(
    $"pull      {pull.Length} bytes out and {Gateway.Encode(held).Length} back hold the downlink path open");

// A node sends a reading, and the gateway hears it on 868.1 MHz at SF9, near the edge
// of its range. It forwards the frame as it arrived, with the levels, the
// concentrator's own timestamp, and its counts since the last report. It holds no key
// and reads none of it.
using var node = new LorawanSession(0x26010001, Filled(0x44), Filled(0x55));
byte[] frame = node.EncodeUplink(7, 2, "21.5"u8);
var heard = new GatewayRxpk(868_100_000, frame)
{
    Link = new LoraLink(9, 125_000),
    RssiDbm = -97,
    SnrDb = -3.2,
    TimestampMicros = 3_512_348_611,
};
var counts = new GatewayStat { Received = 2, ReceivedOk = 1, Forwarded = 1, AcknowledgedPercent = 100 };
byte[] datagram = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PushData, 0x1234)
{
    GatewayEui = GatewayEui,
    Packets = [heard],
    Status = counts,
});
Console.WriteLine(
    $"push      a reading and the gateway's counts, {datagram.Length} bytes, token {0x1234:x4}");

// The server reads it. The frequency is in hertz, the datarate identifier is the link
// settings, and the payload is bytes, so nothing is decoded by hand.
GatewayPacket forwarded = Gateway.Parse(datagram);
GatewayRxpk received = forwarded.Packets[0];
Console.WriteLine(Invariant(
    $"heard     {received.FrequencyHz} Hz at SF{received.Link!.SpreadingFactor}, {received.Link.BandwidthHz / 1000} kHz, ") +
    Invariant($"{received.RssiDbm:F0} dBm, SNR {received.SnrDb:F1} dB, ") +
    $"CRC {received.Crc.ToString().ToLowerInvariant()}, {received.Payload.Length} bytes");
GatewayStat report = forwarded.Status!;
Console.WriteLine(Invariant(
    $"counts    {report.Received} received, {report.ReceivedOk} with a good CRC, {report.Forwarded} forwarded, {report.AcknowledgedPercent:F1}% acknowledged"));

// It is acknowledged at once, by token, before anything in it is read.
GatewayPacket ack = Gateway.Acknowledgment(Gateway.Parse(datagram))!;
Console.WriteLine($"ack       token {ack.Token:x4} acknowledged in {Gateway.Encode(ack).Length} bytes");

// An answer goes back in a PULL_RESP, timed in the concentrator's own microseconds for
// the device's first receive window, a second after the uplink ended, with the
// inverted polarity a LoRaWAN device listens for.
byte[] answer = node.EncodeDownlink(0, 2, "ok"u8);
byte[] pullResp = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PullResp, 0x00AB)
{
    Transmit = new GatewayTxpk(868_100_000, answer)
    {
        Link = new LoraLink(9, 125_000),
        TimestampMicros = 3_513_348_611,
        PowerDbm = 14,
        InvertPolarity = true,
    },
});
GatewayTxpk transmit = Gateway.Parse(pullResp).Transmit!;
string iq = transmit.InvertPolarity ? "IQ inverted" : "IQ upright";
Console.WriteLine(
    $"downlink  at {transmit.TimestampMicros} us on {transmit.FrequencyHz} Hz, {transmit.PowerDbm} dBm, {iq}");

// The gateway answers each PULL_RESP with a TX_ACK saying what became of it:
// scheduled, or refused with a reason, such as a window that had already passed.
foreach (GatewayTxStatus said in new[] { GatewayTxStatus.None, GatewayTxStatus.TooLate })
{
    byte[] reported = Gateway.Encode(new GatewayPacket(GatewayPacketKind.TxAck, 0x00AB)
    {
        GatewayEui = GatewayEui,
        TxStatus = said,
    });
    GatewayTxStatus status = Gateway.Parse(reported).TxStatus!.Value;
    string meaning = status == GatewayTxStatus.None
        ? "it goes out in the device's window"
        : "it was not sent";
    Console.WriteLine($"txack     {Gateway.NameOf(status)}: {meaning}");
}
```
<!-- end -->

The network side of the same site:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs#network -->
From [`bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs):

```csharp
// One site, on the band it operates in, admitting one device it was told about: its EUI
// from its label, the application it joins, and the root key it was provisioned with.
byte[] devEui = Convert.FromHexString("70b3d57ed0001234");
byte[] joinEui = Convert.FromHexString("70b3d57ed0000000");
byte[] appKey = Filled(0x2B);
using LoraChannelPlan eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
using var site = new GatewayNetwork(eu868, 0x000013, firstDevAddr: 0x26010001);
site.Register(devEui, joinEui, appKey);

// The gateway forwards a join request it heard. Nothing about the device is known here
// beyond the key, which is what verifies the request, and the accept is timed for the
// join window, five seconds after the request.
LoraLink dr5 = eu868.LinkSettings(5)!;
using var joiner = new LorawanDevice(devEui, joinEui, appKey);
const uint HeardAt = 1_000_000;
GatewayNetworkEvent joined = site.Uplink(
    new GatewayRxpk(868_100_000, joiner.JoinRequest(0x0102)) { Link = dr5, TimestampMicros = HeardAt });
uint acceptedAt = joined.Accept!.TimestampMicros!.Value;
Console.WriteLine(
    $"joined    0x{joined.DevAddr:x8}, accepted at {acceptedAt} us, {(acceptedAt - HeardAt) / 1_000_000} s after the request");

// The device reads the accept and sends a reading. The site decrypts it and says where
// an answer goes: the uplink's own channel, a second after it ended.
using LorawanJoinAccept granted = joiner.AcceptJoin(joined.Accept!.Payload, 0x0102);
using LorawanSession device = granted.Session();
var carried = new GatewayRxpk(868_100_000, device.EncodeUplink(0, 2, "21.5"u8))
{
    Link = dr5,
    TimestampMicros = 9_000_000,
};
GatewayNetworkEvent reading = site.Uplink(carried);
Console.WriteLine(
    $"uplink    frame {reading.Fcnt} on port {reading.Fport} says {Encoding.UTF8.GetString(reading.Payload!)}, " +
    $"answer at {reading.Slot!.TimestampUs} us on {reading.Slot!.FrequencyHz} Hz");

// The answer goes out in that window, encrypted with the session the join granted.
GatewayTxpk reply = site.Answer(reading.DevAddr, reading.Slot!, 2, "ok"u8);
Console.WriteLine($"answer    {reply.Payload.Length} bytes at {reply.TimestampMicros} us");

// The same frame again, as a replay would send it, is refused: its counter was seen.
try
{
    site.Uplink(carried);
    throw new InvalidOperationException("a counter is taken once");
}
catch (PamojaException refused)
{
    Console.WriteLine($"replay    {refused.Message}");
}

// A gateway hears every network in range, and a frame from one this site never
// granted is reported as another network's rather than refused.
using var elsewhere = new LorawanSession(0x12345678, Filled(0x09), Filled(0x08));
GatewayNetworkEvent stranger = site.Uplink(
    new GatewayRxpk(868_300_000, elsewhere.EncodeUplink(0, 1, "hello"u8)) { Link = dr5 });
Console.WriteLine($"foreign   0x{stranger.DevAddr:x8} belongs to another network");
```
<!-- end -->

The Basics Station session:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs#station -->
From [`bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs):

```csharp
// The station asks its configured address where its network server is, naming itself.
// The server reads who asked, and sends it to the websocket its session runs on.
byte[] station = Convert.FromHexString("b827ebfffe010203");
string asking = GatewayStation.Discovery(station);
Console.WriteLine($"ask       {GatewayStation.DiscoveryPath} {asking}");
byte[] asked = GatewayStation.DiscoveryParse(asking);
string answer = GatewayStation.RouterAccepted(
    asked, Convert.FromHexString("0000000000000001"), "ws://lns.example.invalid:3001/router");
Console.WriteLine($"open      {GatewayStation.RouterParse(answer).Uri}");

// Once the websocket is open the station speaks first, saying what it is.
string hello = GatewayStation.Encode(new GatewayStationMessage(GatewayStationKind.Version)
{
    Station = "pamoja",
    Firmware = PamojaCore.Version,
    Package = "pamoja-gateway",
    Model = "linux",
    Protocol = GatewayStation.ProtocolVersion,
    Features = "gps",
});
GatewayStationMessage said = GatewayStation.Parse(hello);
Console.WriteLine($"version   {said.Station} {said.Firmware} on {said.Model}, protocol {said.Protocol}");

// The radio hears a node's reading 3512.348611 seconds into the station's first run. A
// station holds no key, so it splits the frame into the fields the protocol names and
// lets the server judge them, with its own clock for the moment it arrived.
using var node = new LorawanSession(0x26010001, Filled(0x44), Filled(0x55));
long heardAt = GatewayStation.Xtime(0, 1, 3_512_348_611);
string updf = GatewayStation.Encode(GatewayStation.Heard(
    node.EncodeUplink(7, 2, "21.5"u8),
    5,
    868_100_000,
    new GatewayStationLevels(0, heardAt) { Rssi = -97.0, Snr = -3.2 }));
GatewayStationMessage uplink = GatewayStation.Parse(updf);
Console.WriteLine(
    $"updf      0x{uplink.DevAddr:x8} counter {uplink.Fcnt} on port {uplink.Fport}, " +
    $"DR{uplink.DataRate}, {uplink.Payload.Length} bytes still encrypted");

// The server answers in the receive windows the region gives: the first at the
// uplink's own rate and channel, the second where the plan fixes it. It hands the
// station's clock back untouched, so the station can time the answer from the moment
// it heard the uplink.
using LoraChannelPlan eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
byte rx1Rate = eu868.Rx1DataRate(uplink.DataRate, 0)!.Value;
LoraRx2 rx2 = eu868.Rx2();
string dnmsg = GatewayStation.Encode(new GatewayStationMessage(GatewayStationKind.Downlink)
{
    DevEui = Convert.FromHexString("70b3d57ed0001234"),
    Class = 0,
    Diid = 1,
    Payload = node.EncodeDownlink(0, 2, "ok"u8),
    RxDelay = 1,
    Rx1 = new GatewayStationWindow(rx1Rate, uplink.FrequencyHz),
    Rx2 = new GatewayStationWindow(rx2.DataRate, rx2.FrequencyHz),
    Priority = 0,
    Xtime = uplink.Levels!.Xtime,
    Rctx = uplink.Levels!.Rctx,
});
GatewayStationMessage told = GatewayStation.Parse(dnmsg);
byte delay = told.RxDelay ?? 1;
Console.WriteLine(
    $"dnmsg     RX1 DR{told.Rx1!.DataRate} on {told.Rx1!.FrequencyHz} Hz or " +
    $"RX2 DR{told.Rx2!.DataRate} on {told.Rx2!.FrequencyHz} Hz, {delay} s after the uplink");

// The station opens the first window a second after the uplink on its own clock, puts
// the answer on the air, and reports it by the identifier the server gave it.
GatewayStationXtime uplinkAt = GatewayStation.XtimeParts(told.Xtime!.Value);
long sentAt = GatewayStation.Xtime(uplinkAt.Unit, uplinkAt.Session, uplinkAt.Micros + (delay * 1_000_000L));
string dntxed = GatewayStation.Encode(new GatewayStationMessage(GatewayStationKind.Transmitted)
{
    Diid = told.Diid,
    DevEui = told.DevEui,
    Rctx = told.Rctx ?? 0,
    Xtime = sentAt,
    Txtime = GatewayStation.XtimeParts(sentAt).Micros / 1e6,
});
GatewayStationMessage reported = GatewayStation.Parse(dntxed);
GatewayStationXtime went = GatewayStation.XtimeParts(reported.Xtime!.Value);
Console.WriteLine($"dntxed    downlink {reported.Diid} went out at {went.Micros} us of run {went.Session}");
```
<!-- end -->

And the network server beside the gateway daemon, on a `UdpClient`:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs#hardware -->
From [`bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs):

```csharp
// The daemon forwards to 127.0.0.1 when it runs on the same Pi. To serve gateways
// elsewhere, listen on 0.0.0.0 behind a firewall that admits only them: the protocol
// has no authentication of its own.
using LoraChannelPlan eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
using var site = new GatewayNetwork(eu868, 0x000013);
site.Register(
    Convert.FromHexString("70b3d57ed0001234"),
    Convert.FromHexString("70b3d57ed0000000"),
    Filled(0x2B));

using var socket = new UdpClient(AddressFamily.InterNetwork);
try
{
    socket.Client.Bind(new IPEndPoint(IPAddress.Loopback, Gateway.DefaultPort));
}
catch (SocketException)
{
    Console.WriteLine($"absent    another program holds port {Gateway.DefaultPort}");
    return;
}

// A pamoja gateway holds its path open every five seconds, so six seconds of silence
// means none is running. After that the server keeps answering until a minute passes
// with nothing heard.
socket.Client.ReceiveTimeout = 6_000;
IPEndPoint? downlinks = null;
ushort token = 0;
while (true)
{
    var from = new IPEndPoint(IPAddress.Any, 0);
    byte[] datagram;
    try
    {
        datagram = socket.Receive(ref from);
    }
    catch (SocketException quiet) when (quiet.SocketErrorCode == SocketError.TimedOut)
    {
        break;
    }

    socket.Client.ReceiveTimeout = 60_000;

    GatewayPacket packet;
    try
    {
        packet = Gateway.Parse(datagram);
    }
    catch (PamojaException why)
    {
        Console.WriteLine($"ignored   {why.Message}");
        continue;
    }

    if (Gateway.Acknowledgment(packet) is { } ack)
    {
        socket.Send(Gateway.Encode(ack), from);
    }

    switch (packet.Kind)
    {
        case GatewayPacketKind.PullData:
            if (downlinks is null)
            {
                Console.WriteLine($"gateway   {packet.GatewayEui} holds its downlink path open");
            }

            downlinks = from;
            break;
        case GatewayPacketKind.PushData:
            foreach (GatewayRxpk arrived in packet.Packets)
            {
                if (arrived.Crc != GatewayCrc.Ok)
                {
                    continue;
                }

                GatewayTxpk transmit;
                try
                {
                    GatewayNetworkEvent happened = site.Uplink(arrived);
                    if (happened.Outcome == GatewayNetworkOutcome.Joined)
                    {
                        Console.WriteLine($"joined    0x{happened.DevAddr:x8}");
                        transmit = happened.Accept!;
                    }
                    else if (happened.Outcome == GatewayNetworkOutcome.Data)
                    {
                        Console.WriteLine(
                            $"reading   0x{happened.DevAddr:x8} says {Encoding.UTF8.GetString(happened.Payload!)}");
                        transmit = site.Answer(happened.DevAddr, happened.Slot!, 2, "ok"u8);
                    }
                    else
                    {
                        continue;
                    }
                }
                catch (PamojaException why)
                {
                    Console.WriteLine($"refused   {why.Message}");
                    continue;
                }

                if (downlinks is not null)
                {
                    token++;
                    byte[] resp = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PullResp, token)
                    {
                        Transmit = transmit,
                    });
                    socket.Send(resp, downlinks);
                }
            }

            break;
        case GatewayPacketKind.TxAck:
            Console.WriteLine($"txack     {Gateway.NameOf(packet.TxStatus!.Value)}");
            break;
    }
}

if (downlinks is null)
{
    Console.WriteLine("absent    no gateway reported in, so nothing was answered");
}
```
<!-- end -->

## Values at a glance

**The six datagrams** of the packet forwarder protocol, from Semtech's
PROTOCOL.TXT. Each starts with the protocol version, 2, a random token, and the byte
below; the three a gateway sends carry its identifier next.

| Kind | Byte | Sent by | Carries | Answered with |
| --- | --- | --- | --- | --- |
| PUSH_DATA | 0x00 | gateway | the packets it heard, its counts | PUSH_ACK |
| PUSH_ACK | 0x01 | server | the token alone | |
| PULL_DATA | 0x02 | gateway | the token alone, to hold its path open | PULL_ACK |
| PULL_RESP | 0x03 | server | one packet to transmit, and when | TX_ACK |
| PULL_ACK | 0x04 | server | the token alone | |
| TX_ACK | 0x05 | gateway | what became of the PULL_RESP | |

**What a TX_ACK says:**

| Word | Means | Usually |
| --- | --- | --- |
| NONE | the packet is programmed | |
| TOO_LATE | it arrived too late to program | the server answered after the window |
| TOO_EARLY | its timestamp is too far ahead | a time taken from another gateway's counter |
| COLLISION_PACKET | another packet holds that time | two answers in one window, or a duty cycle still owed |
| COLLISION_BEACON | a class B beacon holds that time | |
| TX_FREQ | the radio chain cannot reach the frequency | a frequency outside the board's band |
| TX_POWER | the gateway cannot transmit at that power | |
| GPS_UNLOCKED | a GPS time was asked for without a GPS lock | |

**What a received packet carries,** and how pamoja holds it:

| Field | Carries | Held as |
| --- | --- | --- |
| tmst | the concentrator's counter at the end of the packet, a 32-bit count of microseconds that wraps every 71.6 minutes | microseconds |
| freq | the carrier, in megahertz to the hertz | hertz |
| datr, codr | the spreading factor and bandwidth, `SF9BW125`, and the coding rate | link settings |
| rssi, lsnr | the strength to the decibel, the SNR to a tenth | decibels |
| stat | the CRC: 1 checked, -1 failed, 0 absent | Ok, Failed, Absent |
| data | the packet, as base64 | bytes |

**When a site answers,** from RP002-1.0.5 section 3.3, and where on EU863-870:

| Window | Opens | Where |
| --- | --- | --- |
| Join accept | 5 s after the request | the request's own channel and data rate |
| First receive window | 1 s after the uplink | the uplink's own channel and data rate |
| Second receive window | 1 s after the first | 869.525 MHz at DR0 |

**What the site reports:**

| Event or refusal | Means |
| --- | --- |
| Joined | a registered device joined, and the accept is ready to transmit |
| Data | a frame decrypted, with its counter, port, payload, and where an answer goes |
| Foreign | a frame for an address this site never granted, which is another network's |
| a counter already seen | a replay, or a device that restarted its counter without joining again |
| a counter too far ahead | more than 16384 frames past the last one taken |
| no registered key | a join request no registration verifies |
| no session | an answer for an address the site holds no session for |

**The Basics Station messages:**

| msgtype | Sent by | Carries |
| --- | --- | --- |
| version | station | its software, firmware, model, protocol, features |
| router_config | server | the networks and join ranges to forward, the region, the band, the data rates |
| jreq | station | a join request heard, split into its fields, with the station clock |
| updf | station | a data frame heard, still encrypted, with the station clock |
| propdf | station | a proprietary frame, whole |
| dnmsg | server | a frame to transmit, and the windows or ping slot it goes in |
| dnsched | server | frames for a group, each at a GPS time |
| dntxed | station | that a frame went out, and when |
| timesync | either | the clocks the two keep between them |

**What a configuration's filters mean** to the reference station:

| Field | Written as | Forwards |
| --- | --- | --- |
| NetID | null | every network's data frames |
| NetID | a list | only the networks listed, matched against the top seven bits of the device address; an empty list forwards none |
| JoinEui | a list of ranges | join requests from those ranges; an empty list forwards every join |
| DRs | one entry per data rate, `[-1,0,0]` for a number left undefined | uplinks at the rates the table names |

**A station's clock,** as the reference station writes an `xtime`:

| Bits | Hold |
| --- | --- |
| 0 to 47 | the microseconds the station's run has counted |
| 48 to 55 | the run, a random byte the station never leaves at 0 |
| 56 to 62 | the radio the time was read on |

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| build and read a datagram | `Packet::PullData { token, gateway }`, `Packet::PushData { .. }`, `to_bytes()`, `Packet::parse(&bytes)` |
| describe what was heard | `Rxpk::new(hz, link, payload).with_rssi_dbm(..).with_snr_db(..).with_timestamp_us(..)`, `Stat::new().with_counts(..)` |
| answer a gateway | `packet.acknowledgment()`, `Txpk::at(tmst, hz, link, payload).with_inverted_polarity(true)` |
| run a site | `Network::new(plan, net_id)`, `register(Registration::new(..))`, `uplink(&rxpk)`, `answer(dev_addr, slot, port, payload)` |
| find a server | `Discovery::new(eui).to_json()`, `Discovery::from_json`, `Router::accepted(router, muxs, uri)`, `Router::from_json` |
| speak Basics Station | `Message::heard(frame, dr, hz, levels)`, `Message::Version { .. }`, `to_json()`, `Message::from_json` |
| read a station clock | `Xtime::new(unit, session, micros)`, `value()`, `Xtime::of(xtime)` |

### TypeScript

| To | Call |
| --- | --- |
| build and read a datagram | `encode({ kind: PacketKind.PushData, token, gateway, packets, status })`, `parse(datagram)` |
| describe what was heard | `{ frequencyHz, payload, link, rssiDbm, snrDb, timestampUs }` |
| answer a gateway | `acknowledgment(packet)`, `encode({ kind: PacketKind.PullResp, token, transmit })` |
| run a site | `new Network(plan, netId, windows?, firstDevAddr?)`, `register(devEui, joinEui, appKey)`, `uplink(rxpk)`, `answer(devAddr, slot, port, payload)` |
| find a server | `stationDiscovery(eui)`, `stationDiscoveryParse(text)`, `stationRouterAccepted(router, muxs, uri)`, `stationRouterParse(text)` |
| speak Basics Station | `stationHeard(frame, dr, hz, levels)`, `stationEncode({ kind: StationKind.Version, .. })`, `stationParse(text)` |
| read a station clock | `stationXtime(unit, session, micros)`, `stationXtimeParts(xtime)` |

### Python

| To | Call |
| --- | --- |
| build and read a datagram | `encode(Packet(PacketKind.PUSH_DATA, token, gateway=.., packets=[..], status=..))`, `parse(datagram)` |
| describe what was heard | `Rxpk(hz, payload, link=.., rssi_dbm=.., snr_db=.., timestamp_us=..)`, `Stat(received=.., ..)` |
| answer a gateway | `acknowledgment(packet)`, `Packet(PacketKind.PULL_RESP, token, transmit=Txpk(..))` |
| run a site | `Network(plan, net_id, first_dev_addr=..)`, `register(dev_eui, join_eui, app_key)`, `uplink(rxpk)`, `answer(dev_addr, slot, port, payload)` |
| find a server | `station_discovery(eui)`, `station_discovery_parse(text)`, `station_router_accepted(router, muxs, uri)`, `station_router_parse(text)` |
| speak Basics Station | `station_heard(frame, dr, hz, levels)`, `station_encode(StationMessage(StationKind.VERSION, ..))`, `station_parse(text)` |
| read a station clock | `station_xtime(unit, session, micros)`, `station_xtime_parts(xtime)` |

### C#

| To | Call |
| --- | --- |
| build and read a datagram | `Gateway.Encode(new GatewayPacket(GatewayPacketKind.PushData, token) { .. })`, `Gateway.Parse(datagram)` |
| describe what was heard | `new GatewayRxpk(hz, payload) { Link, RssiDbm, SnrDb, TimestampMicros }`, `new GatewayStat { Received, .. }` |
| answer a gateway | `Gateway.Acknowledgment(packet)`, `new GatewayPacket(GatewayPacketKind.PullResp, token) { Transmit }` |
| run a site | `new GatewayNetwork(plan, netId, firstDevAddr:)`, `Register(devEui, joinEui, appKey)`, `Uplink(rxpk)`, `Answer(devAddr, slot, port, payload)` |
| find a server | `GatewayStation.Discovery(eui)`, `DiscoveryParse(text)`, `RouterAccepted(router, muxs, uri)`, `RouterParse(text)` |
| speak Basics Station | `GatewayStation.Heard(frame, dr, hz, levels)`, `Encode(new GatewayStationMessage(GatewayStationKind.Version) { .. })`, `Parse(text)` |
| read a station clock | `GatewayStation.Xtime(unit, session, micros)`, `XtimeParts(xtime)` |

<!-- languages end -->

## When it goes wrong

A datagram or a message that is not the protocol is refused with what is wrong with
it, and the site refuses a frame with the reason. The mistakes that cost an
afternoon:

- **Downlinks never arrive.** A server can only send to a gateway behind a router
  through the path its PULL_DATA opened. Answer every PULL_DATA, and send each
  PULL_RESP to the address the latest PULL_DATA came from, not to the address the
  PUSH_DATA did.
- **Every answer comes back TOO_LATE.** The first window opens a second after the
  uplink ended, so the server has that second, less the network's delay and the
  gateway's own margin, to decide and answer. Acknowledge first, then work.
- **Downlinks go wrong about an hour in.** The concentrator's counter is 32 bits of
  microseconds and wraps every 71.6 minutes, so a window worked out by adding to it
  has to wrap with it. `Network` adds with wrapping; code that adds in a wider type
  sends a time past anything the counter holds.
- **The gateway says NONE and the device hears nothing.** The downlink must invert IQ,
  and go out on the channel and data rate the window names. The second window's
  frequency is the region's, not the uplink's.
- **Every join is refused.** No registration verifies the request: a device EUI or a
  join EUI typed in the wrong byte order, or a root key that differs from the one the
  device holds.
- **A reading is refused as a replay.** The device counted from zero again without
  joining again, such as after a reboot, or a frame really was played back. A join
  resets the counters on both ends.
- **Frames from strangers fill the log.** A gateway hears every network in range.
  Another network's frame is reported, not refused, and is not an error.
- **The gateway reports in, then nothing.** The protocol names no port. Most network
  servers listen on 1700, while Semtech's sample configurations use 1730; match the
  gateway's configuration, and open the port in any firewall between them. With no
  authentication of its own, it belongs on a private network or in a tunnel.
- **A Semtech gateway is missed at start.** Semtech's forwarder holds its path open
  every ten seconds by default and pamoja's every five. A server that gives up
  sooner can miss one that is running.
- **A station forwards no data frames.** A configuration's network list that is
  present and empty forwards none. Write it as null to forward every network's.
- **A station answer lands microseconds off.** An xtime has to come back exactly as
  it went up. In TypeScript it is a bigint for that reason, and any code that turns
  it into a number rounds it.
- **A join filter lets everything through.** The reference station reads its join
  ranges up to the first one that starts at 0, so a first range from 0 turns the
  filter off, and ranges after one from 0 are never read.

## Where next

<!-- table: next gateway -->
- [LoRaWAN](lorawan.md): LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join.
- [MQTT](mqtt.md): An MQTT client with the topic and wildcard rules, as the core transport.
- [Telemetry](telemetry.md): Observability that ships only what is worth the bytes as link cost rises, while counting everything.
- Beside it: [Gateway](../boards/gateway.md), [Radios and antennas](../radio.md).
- Also in Radio and reach: [LoRa airtime and range](lora.md), [LoRa radios](radios.md), [Mesh frames](mesh.md), [Routing](routing.md).
<!-- end -->

## Reference

<!-- table: reference gateway -->
- Rust: [`pamoja-gateway`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gateway/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gateway)
- TypeScript: [`@pamoja/gateway`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gateway.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gateway)
- Python: [`pamoja.gateway`](https://pamoja.molex.cloud/docs/reference/python/pamoja/gateway.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gateway)
- C#: [`Pamoja.Gateway`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gateway.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gateway)
- Hardware: [SX1302](https://pamoja.molex.cloud/docs/hardware.html#sx1302), [SX1303](https://pamoja.molex.cloud/docs/hardware.html#sx1303), [SX1250](https://pamoja.molex.cloud/docs/hardware.html#sx1250), [RAK2287 WisLink concentrator](https://pamoja.molex.cloud/docs/hardware.html#rak2287), [RAK5146 WisLink concentrator](https://pamoja.molex.cloud/docs/hardware.html#rak5146), [WM1302 LoRaWAN gateway module](https://pamoja.molex.cloud/docs/hardware.html#wm1302)
<!-- end -->
