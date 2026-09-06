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
const string Topic = "sensors/1/temperature";

// Two links off the same node: a near mesh hop and a metered backhaul. Each is a
// separate broker, so which one carried a reading is visible from its subscriber.
using var mesh = new LoopbackBroker();
using var backhaul = new LoopbackBroker();
using var gateway = backhaul.Link();
await gateway.ConnectAsync();
await gateway.SubscribeAsync(Topic);

// Rungs are tried in the order they are added, cheapest first. The mesh hop loses
// every packet here; the backhaul carries one send, then drops the next two.
using var ladder = new Ladder(Store.Memory());
ladder.Rung(Transport.Degraded(mesh.Rung(), dropEvery: 1));
ladder.Rung(Transport.Degraded(backhaul.Rung(), up: 1, down: 2));
await ladder.ConnectAsync();

// The mesh hop refuses, so the reading goes out over the backhaul and arrives on
// the broker only that rung publishes to.
Delivery first = await ladder.SendAsync(Topic, "21.5"u8.ToArray());
TransportMessage arrived = (await gateway.ReceiveAsync())!;
Console.WriteLine(
    $"first reading: {first}, gateway got"
    + $" {System.Text.Encoding.UTF8.GetString(arrived.Payload)}");

// Now nothing will take a send, so the next reading is buffered rather than lost.
Delivery second = await ladder.SendAsync(Topic, "21.6"u8.ToArray());
int waiting = await ladder.BufferedAsync();
Console.WriteLine($"second reading: {second}, {waiting} waiting in the queue");

// A flush while the links are still down forwards nothing and leaves the backlog
// intact, because a record is removed only once a rung has accepted it.
int whileDown = await ladder.FlushAsync();
Console.WriteLine(
    $"flush while down forwarded {whileDown}, queue still {await ladder.BufferedAsync()}");

// The backhaul is reachable again, so the buffered reading goes out exactly once.
int whenUp = await ladder.FlushAsync();
TransportMessage late = (await gateway.ReceiveAsync())!;
Console.WriteLine(
    $"flush when up forwarded {whenUp}, gateway got"
    + $" {System.Text.Encoding.UTF8.GetString(late.Payload)}");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-ladder`](https://crates.io/crates/pamoja-ladder) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html), [docs.rs](https://docs.rs/pamoja-ladder), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-ladder) |
| TypeScript | [`@pamoja/ladder`](https://www.npmjs.com/package/@pamoja/ladder) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-ladder) |
| Python | [`pamoja-ladder`](https://pypi.org/project/pamoja-ladder/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-ladder) |
| C# | [`Pamoja.Ladder`](https://www.nuget.org/packages/Pamoja.Ladder) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-ladder) |

## Documentation

- [`Pamoja.Ladder` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html), every type in this namespace.
- [The Transport ladder guide](https://pamoja.molex.cloud/docs/guides/ladder.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
