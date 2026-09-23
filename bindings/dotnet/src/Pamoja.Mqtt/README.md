# Pamoja.Mqtt

An MQTT client with the topic and wildcard rules, as the core transport. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mqtt.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html)

## Install

```sh
dotnet add package Pamoja.Mqtt
```

```csharp
using Pamoja.Mqtt;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Core`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs):

```csharp
// The broker on the site. The guide's CI runs one on localhost; point these at
// yours and nothing else changes.
const string Broker = "127.0.0.1";
const ushort Port = 1883;

static string Connection(bool connected) =>
    connected ? "still connected" : "not connected";

// The gateway takes every temperature on the site. A `+` stands for exactly one
// level, so this matches every node's temperature and nothing deeper.
await using var gateway = new MqttClient(new MqttClientOptions
{
    ClientId = "site-gateway",
    Host = Broker,
    Port = Port,
    Qos = Qos.AtLeastOnce,
});
await gateway.ConnectAsync();
await gateway.SubscribeAsync("sensors/+/temperature");
Console.WriteLine("gateway   subscribed to sensors/+/temperature");

// A node publishes under that pattern. At least once has the broker acknowledge
// each message, where at most once would send it and forget it.
await using var node = new MqttClient(new MqttClientOptions
{
    ClientId = "node-1",
    Host = Broker,
    Port = Port,
    Qos = Qos.AtLeastOnce,
});
await node.ConnectAsync();
await node.PublishAsync("sensors/1/temperature", "21.5");
Console.WriteLine("node      published 21.5 to sensors/1/temperature");

// The gateway receives it with the topic attached, which is how it knows which
// node sent the reading without the payload having to repeat it.
MqttMessage received = (await gateway.RecvAsync())!;
Console.WriteLine(
    $"gateway   got {received.Text}"
    + $" on {received.Topic}");

// Two days of readings saved at one a minute, sent as one message, make a packet
// over the connection's 10 KiB limit. The send is refused before anything leaves
// and the connection stays up; a node that must send it raises the limit on every
// client that shares the topic, or splits it.
string backlog = string.Join(",", Enumerable.Repeat("21.5", 2 * 24 * 60));
try
{
    await node.PublishAsync("sensors/1/backlog", backlog);
    Console.WriteLine("node      sent an oversized backlog, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"node      backlog refused: {error.Message}");
}

bool afterRefusal = await node.IsConnectedAsync();
Console.WriteLine($"node      {Connection(afterRefusal)}");

// Disconnecting leaves the client reusable, so a node that loses its link can
// reconnect the same object when the broker comes back.
await node.DisconnectAsync();
bool afterDisconnect = await node.IsConnectedAsync();
Console.WriteLine($"node      {Connection(afterDisconnect)} after disconnecting");

// A broker that is not there is reported rather than leaving a client that looks
// connected, so a retry loop has something to test.
await using var nowhere = new MqttClient(new MqttClientOptions
{
    ClientId = "node-2",
    Host = Broker,
    Port = 1,
    KeepAliveSecs = 1,
});
try
{
    await nowhere.ConnectAsync();
    Console.WriteLine("an unreachable broker accepted a connection, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"unreachable broker refused: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mqtt`](https://crates.io/crates/pamoja-mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html), [docs.rs](https://docs.rs/pamoja-mqtt), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-mqtt) |
| TypeScript | [`@pamoja/mqtt`](https://www.npmjs.com/package/@pamoja/mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-mqtt) |
| Python | [`pamoja-mqtt`](https://pypi.org/project/pamoja-mqtt/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-mqtt) |
| C# | [`Pamoja.Mqtt`](https://www.nuget.org/packages/Pamoja.Mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-mqtt) |

## Documentation

- [`Pamoja.Mqtt` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html), every type in this namespace.
- [The MQTT guide](https://pamoja.molex.cloud/docs/guides/mqtt.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
