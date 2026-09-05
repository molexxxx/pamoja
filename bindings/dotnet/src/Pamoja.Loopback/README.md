# Pamoja.Loopback

An in-process transport with topic matching and a fault injector, for testing with no broker. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/loopback.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Loopback
```

```csharp
using Pamoja.Loopback;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Core`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/LoopbackGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LoopbackGuide.cs):

```csharp
// One broker and two links off it, all in this process. Nothing binds a port and
// nothing has to be running for the traffic below to flow, which is what makes
// this the link to develop a node against before it has a real one.
using var broker = new LoopbackBroker();
using LoopbackTransport publisher = broker.Link();
using LoopbackTransport subscriber = broker.Link();
await publisher.ConnectAsync();
await subscriber.ConnectAsync();

// A `+` stands for exactly one level, so this takes the mixer's temperature but
// not the raw reading a level below it.
await subscriber.SubscribeAsync("line/+/temp");
await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());
await publisher.SendAsync("line/mixer/temp", "21.5"u8.ToArray());

TransportMessage message = (await subscriber.ReceiveAsync())!;
Console.WriteLine(
    $"line/+/temp took {System.Text.Encoding.UTF8.GetString(message.Payload)}"
    + $" from {message.Topic}");

// A `#` covers every level that remains, so a second link takes the whole subtree,
// including the reading the single-level filter passed over.
using LoopbackTransport watcher = broker.Link();
await watcher.ConnectAsync();
await watcher.SubscribeAsync("line/#");
await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());

TransportMessage deep = (await watcher.ReceiveAsync())!;
Console.WriteLine(
    $"line/#     took {System.Text.Encoding.UTF8.GetString(deep.Payload)}"
    + $" from {deep.Topic}");

// A link that has been disconnected reports the failure instead of dropping the
// reading, which is the case a test wants to reach without unplugging anything.
await publisher.DisconnectAsync();
try
{
    await publisher.SendAsync("line/mixer/temp", "21.6"u8.ToArray());
    Console.WriteLine("a disconnected link took a reading, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"disconnected refused the reading: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-loopback`](https://crates.io/crates/pamoja-loopback) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html), [docs.rs](https://docs.rs/pamoja-loopback) |
| TypeScript | [`@pamoja/loopback`](https://www.npmjs.com/package/@pamoja/loopback) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html) |
| Python | [`pamoja-loopback`](https://pypi.org/project/pamoja-loopback/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/loopback.html) |
| C# | [`Pamoja.Loopback`](https://www.nuget.org/packages/Pamoja.Loopback) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html) |

## Documentation

- [`Pamoja.Loopback` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html), every type in this namespace.
- [The Loopback guide](https://pamoja.molex.cloud/docs/guides/loopback.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
