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
// One broker and two links off it, all in this process. Nothing binds a port
// and nothing has to be running for the traffic below to flow.
using var broker = new LoopbackBroker();
using LoopbackTransport publisher = broker.Link();
using LoopbackTransport subscriber = broker.Link();
await publisher.ConnectAsync();
await subscriber.ConnectAsync();

// A `+` stands for exactly one level, so the deeper topic is not delivered
// here and the first message this subscriber sees is the second publish.
await subscriber.SubscribeAsync("line/+/temp");
await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());
await publisher.SendAsync("line/mixer/temp", "21.5"u8.ToArray());

TransportMessage? message = await subscriber.ReceiveAsync();
Expect(message?.Topic == "line/mixer/temp", "the topic survives the trip");
Expect(
    message!.Payload.AsSpan().SequenceEqual("21.5"u8),
    "and so does the reading");

// A `#` covers the levels that remain, so a second link takes the whole
// subtree, including the reading the single-level filter passed over.
using LoopbackTransport watcher = broker.Link();
await watcher.ConnectAsync();
await watcher.SubscribeAsync("line/#");
await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());

TransportMessage? deep = await watcher.ReceiveAsync();
Expect(deep?.Topic == "line/mixer/temp/raw", "the deeper topic arrives here");
Expect(deep!.Payload.AsSpan().SequenceEqual("2150"u8), "with its own payload");

// A link that has been disconnected reports the failure instead of dropping
// the reading, which is the case a test wants to reach without unplugging.
await publisher.DisconnectAsync();
bool refused = false;
try
{
    await publisher.SendAsync("line/mixer/temp", "21.6"u8.ToArray());
}
catch (PamojaException)
{
    refused = true;
}
Expect(refused, "a disconnected link refuses to publish");
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
