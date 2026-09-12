# LoRaWAN gateways

A gateway is a radio with an uplink. It hears packets from every node in range,
whatever network they belong to, and hands them to a server that knows what to do
with them; the server hands back the packets to transmit, and the gateway puts
them on the air at the microsecond the device is listening.

The protocol between the two is not LoRaWAN. LoRaWAN is what the packets
themselves carry, encrypted end to end; the gateway never reads them. What the
gateway speaks is Semtech's packet forwarder protocol: six kinds of UDP datagram,
deliberately plain, with no authentication and no retries, which is why it belongs
on a private network or inside a tunnel. `pamoja-gateway` speaks it from both
sides, so the same types build a gateway and the server it talks to.

## What the example does

It carries one packet up and one packet down.

A gateway hears a packet on 868.1 MHz at SF7, and forwards it in a PUSH_DATA with
the levels it was heard at and the concentrator's own timestamp. The server reads
the datagram, acknowledges it by token, and later sends a PULL_RESP with a packet
to transmit at the timestamp that hits the device's receive window, with the
inverted polarity a LoRaWAN device listens for. The gateway answers with a TX_ACK
that says the slot was already taken.

It proves:

- A frequency crosses in hertz rather than as a float of megahertz, a payload
  crosses as bytes rather than base64, and a datarate identifier such as
  `SF7BW125` is the same link settings the airtime and range math takes.
- Every uplink is acknowledged by token, and the acknowledgment is four bytes:
  the version, the token, and the identifier of the kind.
- A downlink says when to transmit in the concentrator's own microseconds, which
  is what a receive window is counted in.
- A refused downlink says why, in the protocol's own words, rather than
  disappearing.

## Run it

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example gateway" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example gateway</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- gateway" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- gateway</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/gateway.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/gateway.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- gateway" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- gateway</code></div>
</div>
<!-- end -->

## Rust

<!-- snippet: examples/guides/gateway.rs#example -->
From [`examples/guides/gateway.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/gateway.rs):

```rust
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
let Packet::PushData { uplink, .. } = Packet::parse(&datagram)?
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
    ?
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
    Packet::parse(&downlink.to_bytes())?
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
    Packet::parse(&refused.to_bytes())?
else {
    panic!("a TX_ACK parses as one");
};
println!("txack     {status}, scheduled {}", status.scheduled());
```
<!-- end -->


And the network side of the same site, which admits the device and answers it:

<!-- snippet: examples/guides/gateway.rs#network -->
From [`examples/guides/gateway.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/gateway.rs):

```rust
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
```
<!-- end -->

And the same site reached over the Basics Station protocol instead, where the station
finds its network server, says what it is, and reports a frame it heard as the fields the
protocol names:

<!-- snippet: examples/guides/gateway.rs#station -->
From [`examples/guides/gateway.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/gateway.rs):

```rust
use pamoja_gateway::station::{Discovery, Levels, Message, Router, DISCOVERY_PATH};
use pamoja_gateway::udp::Eui;
use pamoja_lorawan::{Session, Uplink};

// The same gateway, now speaking the other protocol. It is configured with an address,
// and asks on that path for the websocket its session runs on.
let station = Eui::from_hex("b827ebfffe010203").expect("sixteen hexadecimal digits");
let asking = Discovery::new(station);
println!("ask       {DISCOVERY_PATH} {}", asking.to_json());

// The server answers with where to connect, or with why it will not have this station.
let answer = Router::from_json(
    br#"{"router":"b827:ebff:fe01:203","muxs":"::0","uri":"ws://lns.example.invalid:3001/router"}"#,
)
.expect("the answer is well formed");
let uri = answer
    .uri
    .clone()
    .expect("an accepted station is sent somewhere");
println!("open      {uri}");

// A station opens with what it is, which is how the server knows what it can do.
let hello = Message::Version {
    station: "pamoja".to_owned(),
    firmware: "0.1.18".to_owned(),
    package: "pamoja-gateway".to_owned(),
    model: "linux".to_owned(),
    protocol: pamoja_gateway::station::PROTOCOL_VERSION,
    features: "gps".to_owned(),
};
println!("version   {}", hello.to_json());

// Now a device sends a reading, and the radio hears the frame. A station holds no key,
// so it does not read the payload: it splits the frame into the fields the protocol
// names and lets the server judge them.
let session = Session::new(0x2601_0001, [0x44; 16], [0x55; 16]);
let frame = session
    .encode_uplink(&Uplink::new(7, 2, b"21.5"))
    .expect("it fits one frame");
let heard = Message::heard(
    frame.as_bytes(),
    5,
    868_100_000,
    Levels {
        rctx: 0,
        xtime: 1_000_000,
        gpstime: None,
        rssi: -35.0,
        snr: 5.1,
    },
)
.expect("a station sends a data frame up");
println!("updf      {}", heard.to_json());

let Message::Uplink {
    dev_addr,
    fcnt,
    fport,
    ref payload,
    ..
} = heard
else {
    panic!("a data frame going up is read as one");
};
println!(
    "heard     {dev_addr:#010x} counter {fcnt} on port {}",
    fport.unwrap_or(0)
);
println!("payload   {} bytes, still encrypted", payload.len());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/gateway.ts#example -->
From [`bindings/node/guides/gateway.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gateway.ts):

```typescript
import { PacketKind, TxStatus, acknowledgment, encode, parse } from '@pamoja/gateway'
import { link } from '@pamoja/lora'

// A gateway on a Raspberry Pi, whose identifier is written from its network interface.
const gateway = 'b827ebfffe010203'
const dr5 = link(7, 125_000)

// It heard a packet on 868.1 MHz, and forwards it with the levels it was heard at and the
// concentrator's own timestamp of the reception.
const heard = {
  frequencyHz: 868_100_000,
  payload: Buffer.from('TEST_PACKET_1234'),
  link: dr5,
  rssiDbm: -35,
  snrDb: 5.1,
  timestampUs: 3_512_348_611,
}
const datagram = encode({ kind: PacketKind.PushData, token: 0x1234, gateway, packets: [heard] })
console.log(`push      ${datagram.length} bytes, token ${(0x1234).toString(16)}`)

// The server reads it. Nothing about the packet has to be decoded by hand: the frequency is
// in hertz, the datarate identifier is the link settings, and the payload is bytes.
const received = parse(datagram).packets![0]
console.log(
  `heard     ${received.frequencyHz} Hz at SF${received.link!.spreadingFactor}, ` +
    `${received.link!.bandwidthHz / 1000} kHz, ${received.rssiDbm} dBm, ` +
    `SNR ${received.snrDb} dB, ${received.payload.length} bytes`,
)

// Every uplink is acknowledged at once, by token, before anything is processed.
const ack = acknowledgment(parse(datagram))!
console.log(`ack       ${encode(ack).length} bytes`)

// Later the server sends one back, at the concentrator timestamp that hits the device's
// receive window, with the inverted polarity a LoRaWAN device listens for.
const downlink = encode({
  kind: PacketKind.PullResp,
  token: 0x00ab,
  transmit: {
    frequencyHz: 869_525_000,
    payload: Buffer.from('downlink'),
    link: dr5,
    timestampUs: 3_513_348_611,
    powerDbm: 27,
    invertPolarity: true,
    withoutCrc: true,
  },
})
const transmit = parse(downlink).transmit!
console.log(
  `downlink  ${transmit.frequencyHz} Hz at ${transmit.powerDbm} dBm, ` +
    `inverted IQ ${transmit.invertPolarity}`,
)

// The gateway answers with what became of it. A packet already scheduled in that window is
// refused rather than dropped silently.
const refused = encode({
  kind: PacketKind.TxAck,
  token: 0x00ab,
  gateway,
  txStatus: TxStatus.CollisionPacket,
})
const status = parse(refused).txStatus!
console.log(`txack     ${status}, scheduled ${status === TxStatus.None}`)
```
<!-- end -->


And the network side of the same site, which admits the device and answers it:

<!-- snippet: bindings/node/guides/gateway.ts#network -->
From [`bindings/node/guides/gateway.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gateway.ts):

```typescript
import { Network } from '@pamoja/gateway'
import { planFor, LoraRegion } from '@pamoja/lora'
import { device, session } from '@pamoja/lorawan'

// One site, on the band it operates in, admitting one device it was told about.
const devEui = Buffer.alloc(8, 0x11)
const appEui = Buffer.alloc(8, 0x22)
const appKey = Buffer.alloc(16, 0x33)
const site = new Network(planFor(LoraRegion.Eu868), 0x00002a, null, 0x26010001)
site.register(devEui, appEui, appKey)

// The gateway forwards a join request it heard. Nothing about the device is known here beyond
// the key it was provisioned with, which is what verifies the request.
const joiner = device(devEui, appEui, appKey)
const joined = site.uplink({
  frequencyHz: 868_100_000,
  payload: joiner.joinRequest(0x0102),
  link: dr5,
  timestampUs: 1_000_000,
})
console.log(
  `joined    0x${joined.devAddr.toString(16).padStart(8, '0')} at ${joined.accept!.timestampUs} us, ` +
    `inverted IQ ${joined.accept!.invertPolarity}`,
)

// The device reads the accept and sends a reading. The site decrypts it and says where an
// answer goes, which is the uplink window plus the delay the region recommends.
const granted = joiner.acceptJoin(joined.accept!.payload, 0x0102)
const carried = site.uplink({
  frequencyHz: 868_100_000,
  payload: granted.session().encodeUplink(0, 2, Buffer.from('21.5')),
  link: dr5,
  timestampUs: 9_000_000,
})
console.log(
  `uplink    frame ${carried.fcnt}, ${carried.payload!.length} bytes, ` +
    `answer at ${carried.slot!.timestampUs} us on ${carried.slot!.frequencyHz} Hz`,
)

// The answer goes out in that window, encrypted with the session the join granted.
const answer = site.answer(carried.devAddr, carried.slot!, 2, Buffer.from('ok'))
console.log(`downlink  ${answer.payload.length} bytes at ${answer.timestampUs} us`)

// A gateway hears every network in range, and a frame from one this site never granted is
// reported rather than refused.
const stranger = site.uplink({
  frequencyHz: 868_100_000,
  payload: session(0x12345678, Buffer.alloc(16, 0x09), Buffer.alloc(16, 0x08)).encodeUplink(
    0,
    1,
    Buffer.from('hello'),
  ),
  link: dr5,
})
console.log(
  `foreign   0x${stranger.devAddr.toString(16).padStart(8, '0')} belongs to another network`,
)
```
<!-- end -->

And the same site reached over the Basics Station protocol instead, where the station
finds its network server, says what it is, and reports a frame it heard as the fields the
protocol names:

<!-- snippet: bindings/node/guides/gateway.ts#station -->
From [`bindings/node/guides/gateway.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gateway.ts):

```typescript
import {
  DISCOVERY_PATH,
  STATION_PROTOCOL_VERSION,
  StationKind,
  stationDiscovery,
  stationHeard,
  stationEncode,
  stationRouterParse,
} from '@pamoja/gateway'

// The same gateway, now speaking the other protocol. It is configured with an address, and
// asks on that path for the websocket its session runs on.
const stationEui = 'b827ebfffe010203'
const ask = stationDiscovery(stationEui)
console.log(`ask       ${DISCOVERY_PATH} ${ask}`)

// The server answers with where to connect, or with why it will not have this station.
const routed = stationRouterParse(
  '{"router":"b827:ebff:fe01:203","muxs":"::0","uri":"ws://lns.example.invalid:3001/router"}',
)
console.log(`open      ${routed.uri}`)

// A station opens with what it is, which is how the server knows what it can do.
const hello = stationEncode({
  kind: StationKind.Version,
  station: 'pamoja',
  firmware: '0.1.18',
  package: 'pamoja-gateway',
  model: 'linux',
  protocol: STATION_PROTOCOL_VERSION,
  features: 'gps',
})
console.log(`version   ${hello}`)

// Now a device sends a reading, and the radio hears the frame. A station holds no key, so it
// does not read the payload: it splits the frame into the fields the protocol names and lets
// the server judge them.
const active = session(0x26010001, Buffer.alloc(16, 0x44), Buffer.alloc(16, 0x55))
const frame = active.encodeUplink(7, 2, Buffer.from('21.5'))
const reported = stationHeard(frame, 5, 868_100_000, {
  rctx: 0,
  xtime: 1_000_000,
  rssi: -35,
  snr: 5.1,
})
console.log(`updf      ${stationEncode(reported)}`)
console.log(
  `heard     0x${(reported.devAddr ?? 0).toString(16).padStart(8, '0')} counter ${reported.fcnt} on port ${reported.fport}`,
)
console.log(`payload   ${reported.payload?.length} bytes, still encrypted`)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/gateway.py#example -->
From [`bindings/python/guides/gateway.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gateway.py):

```python
from pamoja.gateway import Packet, PacketKind, Rxpk, Txpk, TxStatus, acknowledgment, encode, parse
from pamoja.lora import link

# A gateway on a Raspberry Pi, whose identifier is written from its network interface.
gateway = "b827ebfffe010203"
dr5 = link(7, 125_000)

# It heard a packet on 868.1 MHz, and forwards it with the levels it was heard at and the
# concentrator's own timestamp of the reception.
heard = Rxpk(
    868_100_000,
    b"TEST_PACKET_1234",
    link=dr5,
    rssi_dbm=-35,
    snr_db=5.1,
    timestamp_us=3_512_348_611,
)
datagram = encode(Packet(PacketKind.PUSH_DATA, 0x1234, gateway=gateway, packets=[heard]))
print(f"push      {len(datagram)} bytes, token {0x1234:04x}")

# The server reads it. Nothing about the packet has to be decoded by hand: the frequency is in
# hertz, the datarate identifier is the link settings, and the payload is bytes.
received = parse(datagram).packets[0]
print(
    f"heard     {received.frequency_hz} Hz at SF{received.link.spreading_factor}, "
    f"{received.link.bandwidth_hz // 1000} kHz, {received.rssi_dbm} dBm, "
    f"SNR {received.snr_db} dB, {len(received.payload)} bytes"
)

# Every uplink is acknowledged at once, by token, before anything is processed.
ack = acknowledgment(parse(datagram))
print(f"ack       {len(encode(ack))} bytes")

# Later the server sends one back, at the concentrator timestamp that hits the device's receive
# window, with the inverted polarity a LoRaWAN device listens for.
downlink = encode(
    Packet(
        PacketKind.PULL_RESP,
        0x00AB,
        transmit=Txpk(
            869_525_000,
            b"downlink",
            link=dr5,
            timestamp_us=3_513_348_611,
            power_dbm=27,
            invert_polarity=True,
            without_crc=True,
        ),
    )
)
transmit = parse(downlink).transmit
print(
    f"downlink  {transmit.frequency_hz} Hz at {transmit.power_dbm} dBm, "
    f"inverted IQ {transmit.invert_polarity}"
)

# The gateway answers with what became of it. A packet already scheduled in that window is
# refused rather than dropped silently.
refused = encode(
    Packet(PacketKind.TX_ACK, 0x00AB, gateway=gateway, tx_status=TxStatus.COLLISION_PACKET)
)
status = parse(refused).tx_status
print(f"txack     {status}, scheduled {status == TxStatus.NONE}")
```
<!-- end -->


And the network side of the same site, which admits the device and answers it:

<!-- snippet: bindings/python/guides/gateway.py#network -->
From [`bindings/python/guides/gateway.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gateway.py):

```python
from pamoja.gateway import Network
from pamoja.lora import plan_for
from pamoja.lorawan import device, session

# One site, on the band it operates in, admitting one device it was told about.
dev_eui = bytes([0x11]) * 8
app_eui = bytes([0x22]) * 8
app_key = bytes([0x33]) * 16
site = Network(plan_for("EU868"), 0x00002A, first_dev_addr=0x26010001)
site.register(dev_eui, app_eui, app_key)

# The gateway forwards a join request it heard. Nothing about the device is known here beyond
# the key it was provisioned with, which is what verifies the request.
joiner = device(dev_eui, app_eui, app_key)
joined = site.uplink(
    Rxpk(868_100_000, joiner.join_request(0x0102), link=dr5, timestamp_us=1_000_000)
)
print(
    f"joined    {joined.dev_addr:#010x} at {joined.accept.timestamp_us} us, "
    f"inverted IQ {joined.accept.invert_polarity}"
)

# The device reads the accept and sends a reading. The site decrypts it and says where an
# answer goes, which is the uplink window plus the delay the region recommends.
granted = joiner.accept_join(joined.accept.payload, 0x0102)
carried = site.uplink(
    Rxpk(
        868_100_000,
        granted.session().encode_uplink(0, 2, b"21.5"),
        link=dr5,
        timestamp_us=9_000_000,
    )
)
print(
    f"uplink    frame {carried.fcnt}, {len(carried.payload)} bytes, "
    f"answer at {carried.slot.timestamp_us} us on {carried.slot.frequency_hz} Hz"
)

# The answer goes out in that window, encrypted with the session the join granted.
answer = site.answer(carried.dev_addr, carried.slot, 2, b"ok")
print(f"downlink  {len(answer.payload)} bytes at {answer.timestamp_us} us")

# A gateway hears every network in range, and a frame from one this site never granted is
# reported rather than refused.
stranger = site.uplink(
    Rxpk(
        868_100_000,
        session(0x12345678, bytes([0x09]) * 16, bytes([0x08]) * 16).encode_uplink(0, 1, b"hello"),
        link=dr5,
    )
)
print(f"foreign   {stranger.dev_addr:#010x} belongs to another network")
```
<!-- end -->

And the same site reached over the Basics Station protocol instead, where the station
finds its network server, says what it is, and reports a frame it heard as the fields the
protocol names:

<!-- snippet: bindings/python/guides/gateway.py#station -->
From [`bindings/python/guides/gateway.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gateway.py):

```python
from pamoja.gateway import (
    DISCOVERY_PATH,
    STATION_PROTOCOL_VERSION,
    StationKind,
    StationLevels,
    station_discovery,
    station_heard,
    station_parse,
    station_router_parse,
)
from pamoja.lorawan import session

# The same gateway, now speaking the other protocol. It is configured with an address, and
# asks on that path for the websocket its session runs on.
station_eui = "b827ebfffe010203"
ask = station_discovery(station_eui)
print(f"ask       {DISCOVERY_PATH} {ask}")

# The server answers with where to connect, or with why it will not have this station.
answer = station_router_parse(
    '{"router":"b827:ebff:fe01:203","muxs":"::0","uri":"ws://lns.example.invalid:3001/router"}'
)
print(f"open      {answer.uri}")

# A station opens with what it is, which is how the server knows what it can do.
hello = station_parse(
    '{"msgtype":"version","station":"pamoja","firmware":"0.1.18",'
    '"package":"pamoja-gateway","model":"linux",'
    f'"protocol":{STATION_PROTOCOL_VERSION},"features":"gps"}}'
)
print(f"version   {hello.msgtype} {hello.station} {hello.firmware}")

# Now a device sends a reading, and the radio hears the frame. A station holds no key, so it
# does not read the payload: it splits the frame into the fields the protocol names and lets
# the server judge them.
active = session(0x26010001, bytes([0x44] * 16), bytes([0x55] * 16))
frame = active.encode_uplink(7, 2, b"21.5")
heard = station_heard(
    frame,
    5,
    868_100_000,
    StationLevels(rctx=0, xtime=1_000_000, rssi=-35.0, snr=5.1),
)
print(f"updf      {heard.msgtype} on {heard.frequency_hz} Hz at DR{heard.data_rate}")
print(f"heard     {heard.dev_addr:#010x} counter {heard.fcnt} on port {heard.fport}")
print(f"payload   {len(heard.payload)} bytes, still encrypted")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs):

```csharp
// A gateway on a Raspberry Pi, whose identifier is written from its network interface.
const string GatewayEui = "b827ebfffe010203";
var dr5 = new LoraLink(7, 125_000);

// It heard a packet on 868.1 MHz, and forwards it with the levels it was heard at and
// the concentrator's own timestamp of the reception.
var heard = new GatewayRxpk(868_100_000, Encoding.UTF8.GetBytes("TEST_PACKET_1234"))
{
    Link = dr5,
    RssiDbm = -35,
    SnrDb = 5.1,
    TimestampMicros = 3_512_348_611,
};
byte[] datagram = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PushData, 0x1234)
{
    GatewayEui = GatewayEui,
    Packets = [heard],
});
Console.WriteLine($"push      {datagram.Length} bytes, token {0x1234:x4}");

// The server reads it. Nothing about the packet has to be decoded by hand: the
// frequency is in hertz, the datarate identifier is the link settings, and the payload
// is bytes.
GatewayRxpk received = Gateway.Parse(datagram).Packets[0];
Console.WriteLine(
    $"heard     {received.FrequencyHz} Hz at SF{received.Link!.SpreadingFactor}, " +
    $"{received.Link.BandwidthHz / 1000} kHz, {received.RssiDbm} dBm, " +
    $"SNR {received.SnrDb} dB, {received.Payload.Length} bytes");

// Every uplink is acknowledged at once, by token, before anything is processed.
GatewayPacket ack = Gateway.Acknowledgment(Gateway.Parse(datagram))!;
Console.WriteLine($"ack       {Gateway.Encode(ack).Length} bytes");

// Later the server sends one back, at the concentrator timestamp that hits the device's
// receive window, with the inverted polarity a LoRaWAN device listens for.
byte[] downlink = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PullResp, 0x00AB)
{
    Transmit = new GatewayTxpk(869_525_000, Encoding.UTF8.GetBytes("downlink"))
    {
        Link = dr5,
        TimestampMicros = 3_513_348_611,
        PowerDbm = 27,
        InvertPolarity = true,
        WithoutCrc = true,
    },
});
GatewayTxpk transmit = Gateway.Parse(downlink).Transmit!;
Console.WriteLine(
    $"downlink  {transmit.FrequencyHz} Hz at {transmit.PowerDbm} dBm, " +
    $"inverted IQ {transmit.InvertPolarity}");

// The gateway answers with what became of it. A packet already scheduled in that window
// is refused rather than dropped silently.
byte[] refused = Gateway.Encode(new GatewayPacket(GatewayPacketKind.TxAck, 0x00AB)
{
    GatewayEui = GatewayEui,
    TxStatus = GatewayTxStatus.CollisionPacket,
});
GatewayTxStatus status = Gateway.Parse(refused).TxStatus!.Value;
Console.WriteLine(
    $"txack     {Gateway.NameOf(status)}, scheduled {status == GatewayTxStatus.None}");
```
<!-- end -->


And the network side of the same site, which admits the device and answers it:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs#network -->
From [`bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs):

```csharp
// One site, on the band it operates in, admitting one device it was told about.
byte[] devEui = new byte[8];
Array.Fill(devEui, (byte)0x11);
byte[] appEui = new byte[8];
Array.Fill(appEui, (byte)0x22);
byte[] appKey = new byte[16];
Array.Fill(appKey, (byte)0x33);

var dr5 = new LoraLink(7, 125_000);
using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
using var site = new GatewayNetwork(plan, 0x00002A, firstDevAddr: 0x26010001);
site.Register(devEui, appEui, appKey);

// The gateway forwards a join request it heard. Nothing about the device is known here
// beyond the key it was provisioned with, which is what verifies the request.
using var joiner = new LorawanDevice(devEui, appEui, appKey);
GatewayNetworkEvent joined = site.Uplink(
    new GatewayRxpk(868_100_000, joiner.JoinRequest(0x0102))
    {
        Link = dr5,
        TimestampMicros = 1_000_000,
    });
Console.WriteLine(
    $"joined    0x{joined.DevAddr:x8} at {joined.Accept!.TimestampMicros} us, " +
    $"inverted IQ {joined.Accept!.InvertPolarity.ToString().ToLowerInvariant()}");

// The device reads the accept and sends a reading. The site decrypts it and says where
// an answer goes, which is the uplink window plus the delay the region recommends.
using LorawanJoinAccept granted = joiner.AcceptJoin(joined.Accept!.Payload, 0x0102);
using LorawanSession activated = granted.Session();
GatewayNetworkEvent carried = site.Uplink(
    new GatewayRxpk(868_100_000, activated.EncodeUplink(0, 2, "21.5"u8))
    {
        Link = dr5,
        TimestampMicros = 9_000_000,
    });
Console.WriteLine(
    $"uplink    frame {carried.Fcnt}, {carried.Payload!.Length} bytes, " +
    $"answer at {carried.Slot!.TimestampUs} us on {carried.Slot!.FrequencyHz} Hz");

// The answer goes out in that window, encrypted with the session the join granted.
GatewayTxpk answer = site.Answer(carried.DevAddr, carried.Slot!, 2, "ok"u8);
Console.WriteLine(
    $"downlink  {answer.Payload.Length} bytes at {answer.TimestampMicros} us");

// A gateway hears every network in range, and a frame from one this site never granted
// is reported rather than refused.
using LorawanSession elsewhere = new LorawanSession(0x12345678, NetworkKey(0x09), NetworkKey(0x08));
GatewayNetworkEvent stranger = site.Uplink(
    new GatewayRxpk(868_100_000, elsewhere.EncodeUplink(0, 1, "hello"u8)) { Link = dr5 });
Console.WriteLine($"foreign   0x{stranger.DevAddr:x8} belongs to another network");
```
<!-- end -->

And the same site reached over the Basics Station protocol instead, where the station
finds its network server, says what it is, and reports a frame it heard as the fields the
protocol names:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs#station -->
From [`bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GatewayGuide.cs):

```csharp
// The same gateway, now speaking the other protocol. It is configured with an
// address, and asks on that path for the websocket its session runs on.
byte[] stationEui = Convert.FromHexString("b827ebfffe010203");
string ask = GatewayStation.Discovery(stationEui);
Console.WriteLine($"ask       {GatewayStation.DiscoveryPath} {ask}");

// The server answers with where to connect, or with why it will not have this station.
GatewayStationRouter routed = GatewayStation.RouterParse(
    """{"router":"b827:ebff:fe01:203","muxs":"::0","uri":"ws://lns.example.invalid:3001/router"}""");
Console.WriteLine($"open      {routed.Uri}");

// An identifier is written the way the protocol prefers, which folds zero groups.
Console.WriteLine($"id6       {GatewayStation.Id6(stationEui)}");

// Now a device sends a reading, and the radio hears the frame. A station holds no
// key, so it does not read the payload: it splits the frame into the fields the
// protocol names and lets the server judge them.
using var active = new LorawanSession(0x26010001, NetworkKey(0x44), NetworkKey(0x55));
byte[] frame = active.EncodeUplink(7, 2, "21.5"u8);
GatewayStationMessage heard = GatewayStation.Heard(
    frame,
    5,
    868_100_000,
    new GatewayStationLevels(0, 1_000_000) { Rssi = -35.0, Snr = 5.1 });
Console.WriteLine($"updf      {heard.Json}");
Console.WriteLine(
    $"heard     0x{heard.DevAddr:x8} counter {heard.Fcnt} on port {heard.Fport}");
Console.WriteLine($"payload   {heard.Payload.Length} bytes, still encrypted");
```
<!-- end -->

## Reference

<!-- table: reference gateway -->
- Rust: [`pamoja-gateway`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gateway/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gateway)
- TypeScript: [`@pamoja/gateway`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gateway.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gateway)
- Python: [`pamoja.gateway`](https://pamoja.molex.cloud/docs/reference/python/pamoja/gateway.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gateway)
- C#: [`Pamoja.Gateway`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gateway.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gateway)
<!-- end -->
