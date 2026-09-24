# Pamoja.Gateway

What a LoRaWAN gateway speaks: the Semtech packet forwarder protocol and the Basics Station protocol on both sides, the network side of a single site, and a bridge from the radio to the link that leaves it. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/gateway.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gateway.html)

## Install

```sh
dotnet add package Pamoja.Gateway
```

```csharp
using Pamoja.Gateway;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Lora` and `Pamoja.Lorawan`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-gateway`](https://crates.io/crates/pamoja-gateway) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gateway/index.html), [docs.rs](https://docs.rs/pamoja-gateway), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gateway) |
| TypeScript | [`@pamoja/gateway`](https://www.npmjs.com/package/@pamoja/gateway) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gateway.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gateway) |
| Python | [`pamoja-gateway`](https://pypi.org/project/pamoja-gateway/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gateway.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gateway) |
| C# | [`Pamoja.Gateway`](https://www.nuget.org/packages/Pamoja.Gateway) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gateway.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gateway) |

## Documentation

- [`Pamoja.Gateway` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gateway.html), every type in this namespace.
- [The LoRaWAN gateways guide](https://pamoja.molex.cloud/docs/guides/gateway.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
