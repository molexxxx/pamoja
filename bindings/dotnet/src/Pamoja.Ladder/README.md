# Pamoja.Ladder

Cheapest reachable link first, buffering to a store when every link is down. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/ladder.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html)

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
const string Report = "vessel/7/report";
const string Orders = "vessel/7/orders";

// Three networks a vessel can reach: the harbor's wifi, the coast's cellular
// network, and a satellite. Each is a broker with an office ashore listening
// on it, so which one carried a report is read off that office rather than
// assumed.
using var harbor = new LoopbackBroker();
using var coast = new LoopbackBroker();
using var sky = new LoopbackBroker();
using LoopbackTransport harborOffice = harbor.Link();
using LoopbackTransport coastOffice = coast.Link();
using LoopbackTransport skyOffice = sky.Link();
foreach (LoopbackTransport office in new[] { harborOffice, coastOffice, skyOffice })
{
    await office.ConnectAsync();
    await office.SubscribeAsync(Report);
}

// Rungs go on cheapest first. The satellite only sends, so it goes on as an
// uplink, which the ladder never subscribes or listens on.
using var ladder = new Ladder(Store.Memory());
ladder.Rung(harbor.Rung());
ladder.Rung(coast.Rung());
ladder.Uplink(sky.Rung());
await ladder.ConnectAsync();
await ladder.SubscribeAsync(Orders);

// In the harbor, the cheapest link takes the report.
Delivery first = await ladder.SendAsync(Report, "report 1");
Console.WriteLine($"harbor    carried {(await harborOffice.ReceiveAsync())!.Text}");

// Past the breakwater the wifi is out of reach and the report falls through
// to the coast, and further out to the satellite.
harbor.Reachable = false;
await ladder.SendAsync(Report, "report 2");
Console.WriteLine(
    $"coast     carried {(await coastOffice.ReceiveAsync())!.Text}, with the harbor out of reach");
coast.Reachable = false;
await ladder.SendAsync(Report, "report 3");
Console.WriteLine(
    $"sky       carried {(await skyOffice.ReceiveAsync())!.Text}, with the coast out of reach too");

// In a storm nothing is in reach, and the report waits in the store rather
// than being lost.
sky.Reachable = false;
Delivery stormy = await ladder.SendAsync(Report, "report 4");
int waiting = await ladder.BufferedAsync();
Console.WriteLine($"vessel    buffered report 4 with every link out of reach, {waiting} waiting");

// A flush with every link still out of reach forwards nothing, because a
// record leaves the store only once a link has taken it.
int idle = await ladder.FlushAsync();
int still = await ladder.BufferedAsync();
Console.WriteLine(
    $"vessel    flushed {idle} while every link was out of reach, {still} still waiting");

// Back in reach of the coast, a flush sends the backlog, oldest first.
coast.Reachable = true;
int forwarded = await ladder.FlushAsync();
TransportMessage late = (await coastOffice.ReceiveAsync())!;
int left = await ladder.BufferedAsync();
Console.WriteLine($"coast     carried {late.Text} on a flush of {forwarded}, {left} waiting");

// Orders from shore come back over whichever listening link is in reach.
await coastOffice.SendAsync(Orders, "return to port");
TransportMessage order = await ladder.ReceiveAsync();
Console.WriteLine($"vessel    took {order.Text} over the coast network");

// A ladder does one thing at a time, so a vessel that listens and reports
// waits for orders with a limit and reports between waits.
TimeSpan quiet = TimeSpan.FromMilliseconds(50);
try
{
    await ladder.ReceiveAsync(quiet);
}
catch (TimeoutException)
{
    Console.WriteLine($"vessel    heard nothing more from shore within {quiet.TotalMilliseconds} ms");
}

await ladder.SendAsync(Report, "report 5");
TransportMessage last = (await coastOffice.ReceiveAsync())!;
Console.WriteLine($"coast     carried {last.Text} between waits");
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
