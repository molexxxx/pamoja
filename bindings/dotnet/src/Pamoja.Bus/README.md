# Pamoja.Bus

An in-memory typed publish and subscribe event bus. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/bus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Bus
```

```csharp
using Pamoja.Bus;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Codec`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs):

```csharp
// A sampler announces a reading and whatever cares about readings picks it up,
// with neither side holding a reference to the other.
using EventBus hub = new EventBus(8);
using EventBus sampler = hub.Subscribe();
using EventBus logger = hub.Subscribe();

await hub.PublishAsync("battery.low"u8.ToArray());
Expect(
    (await sampler.NextAsync())!.AsSpan().SequenceEqual("battery.low"u8),
    "the sampler's endpoint received the event");
Expect(
    (await logger.NextAsync())!.AsSpan().SequenceEqual("battery.low"u8),
    "and so did the logger's");

// An endpoint taken later starts from the next event, so it never sees what went
// out before it existed.
using EventBus late = hub.Subscribe();
await hub.PublishAsync("link.up"u8.ToArray());
Expect(
    (await late.NextAsync())!.AsSpan().SequenceEqual("link.up"u8),
    "the endpoint taken last begins at the event after it");
Expect(
    (await sampler.NextAsync())!.AsSpan().SequenceEqual("link.up"u8),
    "an endpoint that was already there follows on in order");

// The buffer is per endpoint and bounded, so an endpoint further behind than the
// capacity drops what it missed and resumes with the most recent events.
using EventBus slow = new EventBus(2);
using EventBus reader = slow.Subscribe();
for (byte count = 0; count < 5; count++)
{
    await slow.PublishAsync(new byte[] { count });
}

Expect((await reader.NextAsync())![0] == 3, "the events it fell behind on were dropped");
Expect((await reader.NextAsync())![0] == 4, "and it resumes with the most recent");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-bus`](https://crates.io/crates/pamoja-bus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html), [docs.rs](https://docs.rs/pamoja-bus) |
| TypeScript | [`@pamoja/bus`](https://www.npmjs.com/package/@pamoja/bus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html) |
| Python | [`pamoja-bus`](https://pypi.org/project/pamoja-bus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html) |
| C# | [`Pamoja.Bus`](https://www.nuget.org/packages/Pamoja.Bus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html) |

## Documentation

- [`Pamoja.Bus` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html), every type in this namespace.
- [The Event bus guide](https://pamoja.molex.cloud/docs/guides/bus.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
