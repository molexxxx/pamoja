# Pamoja.Gateway

The Semtech UDP packet forwarder protocol on both sides: the datagrams a gateway and a network server exchange, and the packets, reports and downlinks they carry. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Lora`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

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
