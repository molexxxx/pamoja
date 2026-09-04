# Pamoja.Mqtt

An MQTT client with the topic and wildcard rules, as the core transport. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mqtt.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
// MQTT numbers its three delivery guarantees 0, 1 and 2 on the wire.
Expect((int)Qos.AtMostOnce == 0, "at most once is level 0");
Expect((int)Qos.AtLeastOnce == 1, "at least once is level 1");
Expect((int)Qos.ExactlyOnce == 2, "exactly once is level 2");

// Nothing listens on this port, so the broker is unreachable. Constructing the
// client touches nothing; only connecting does.
await using var client = new MqttClient(new MqttClientOptions
{
    ClientId = "guide-node",
    Host = "127.0.0.1",
    Port = 47811,
    KeepAliveSecs = 1,
    Qos = Qos.ExactlyOnce,
});
Expect(!await client.IsConnectedAsync(), "a fresh client holds no connection");

// A refused connection surfaces as a transport error and leaves the client as it
// was, so the same object can be retried once the broker is back.
bool refused = false;
try
{
    await client.ConnectAsync();
}
catch (PamojaException error)
{
    refused = error.Message.StartsWith("transport error", StringComparison.Ordinal);
}

Expect(refused, "an unreachable broker is reported, not swallowed");
Expect(
    !await client.IsConnectedAsync(),
    "a failed connect leaves the client disconnected");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mqtt`](https://crates.io/crates/pamoja-mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html), [docs.rs](https://docs.rs/pamoja-mqtt) |
| TypeScript | [`@pamoja/mqtt`](https://www.npmjs.com/package/@pamoja/mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html) |
| Python | [`pamoja-mqtt`](https://pypi.org/project/pamoja-mqtt/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html) |
| C# | [`Pamoja.Mqtt`](https://www.nuget.org/packages/Pamoja.Mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html) |

## Documentation

- [`Pamoja.Mqtt` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html), every type in this namespace.
- [The MQTT guide](https://pamoja.molex.cloud/docs/guides/mqtt.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
