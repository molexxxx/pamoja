# Pamoja.Ladder

Cheapest reachable link first, buffering to a store when every link is down. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/ladder.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Ladder
```

```csharp
using Pamoja.Ladder;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Core` and `Pamoja.Sync`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/LadderGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LadderGuide.cs):

```csharp
// Two links off the same node: a near mesh hop and a metered backhaul. Each is a
// separate broker, so which one carried a reading is visible from its subscriber.
using var mesh = new LoopbackBroker();
using var backhaul = new LoopbackBroker();
using var gateway = backhaul.Link();
await gateway.ConnectAsync();
await gateway.SubscribeAsync("sensors/1/temperature");

// Rungs are tried in the order they are added, cheapest first. The mesh hop loses
// every packet here; the backhaul carries one send, then drops the next two.
using var ladder = new Ladder(Store.Memory());
ladder.Rung(Transport.Degraded(mesh.Rung(), dropEvery: 1));
ladder.Rung(Transport.Degraded(backhaul.Rung(), up: 1, down: 2));
await ladder.ConnectAsync();

// The mesh hop refuses, so the reading goes out over the backhaul and arrives on
// the broker only that rung publishes to.
const string topic = "sensors/1/temperature";
Expect(
    await ladder.SendAsync(topic, "21.5"u8.ToArray()) == Delivery.Sent,
    "a dead rung falls through to the next one");
Expect(
    (await gateway.ReceiveAsync())?.Payload.AsSpan().SequenceEqual("21.5"u8) == true,
    "and the reading arrives over the rung that took it");

// Now nothing will take a send, so the next reading is buffered rather than lost.
Expect(
    await ladder.SendAsync(topic, "21.6"u8.ToArray()) == Delivery.Buffered,
    "with every rung down the reading is buffered");
Expect(await ladder.BufferedAsync() == 1, "and the backlog holds it");

// A flush while the links are still down forwards nothing and leaves the backlog
// intact, because a record is removed only once a rung has accepted it.
Expect(await ladder.FlushAsync() == 0, "a flush with no link forwards nothing");
Expect(await ladder.BufferedAsync() == 1, "and loses nothing");

// The backhaul is reachable again, so the buffered reading goes out exactly once.
Expect(await ladder.FlushAsync() == 1, "the reading goes out once a link returns");
Expect(await ladder.BufferedAsync() == 0, "leaving nothing queued");
Expect(
    (await gateway.ReceiveAsync())?.Payload.AsSpan().SequenceEqual("21.6"u8) == true,
    "and it arrives exactly once");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-ladder`](https://crates.io/crates/pamoja-ladder) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html), [docs.rs](https://docs.rs/pamoja-ladder) |
| TypeScript | [`@pamoja/ladder`](https://www.npmjs.com/package/@pamoja/ladder) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html) |
| Python | [`pamoja-ladder`](https://pypi.org/project/pamoja-ladder/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html) |
| C# | [`Pamoja.Ladder`](https://www.nuget.org/packages/Pamoja.Ladder) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html) |

## Documentation

- [`Pamoja.Ladder` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html), every type in this namespace.
- [The Transport ladder guide](https://pamoja.molex.cloud/docs/guides/ladder.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
