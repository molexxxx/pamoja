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
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo test -p pamoja-examples --test guides gateway -- --nocapture" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo test -p pamoja-examples --test guides gateway -- --nocapture</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run test:guides -- gateway" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run test:guides -- gateway</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/gateway.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/gateway.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- gateway" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- gateway</code></div>
</div>
<!-- end -->

## Rust

<!-- snippet: examples/tests/guides/gateway.rs#example -->
From [`examples/tests/guides/gateway.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/gateway.rs):

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

## Reference

<!-- table: reference gateway -->
- Rust: [`pamoja-gateway`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gateway/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gateway)
- TypeScript: [`@pamoja/gateway`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gateway.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gateway)
- Python: [`pamoja.gateway`](https://pamoja.molex.cloud/docs/reference/python/pamoja/gateway.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gateway)
- C#: [`Pamoja.Gateway`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gateway.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gateway)
<!-- end -->
