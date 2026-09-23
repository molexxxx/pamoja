# Pamoja.Sync

Offline-first queues in memory or on disk, bounded or not, the on-disk one surviving power loss, and the drain that forwards them in order when a link returns. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sync.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html)

## Install

```sh
dotnet add package Pamoja.Sync
```

```csharp
using Pamoja.Sync;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Codec` and `Pamoja.Core`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/SyncGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SyncGuide.cs):

```csharp
const string Topic = "apiary/hive-3/weight";

// The scale logs its weight to a queue on its SD card, bounded so a long
// outage cannot fill the card. The directory is the queue, so the scale can
// lose power at any moment and lose nothing it logged.
DirectoryInfo dir = Directory.CreateTempSubdirectory("pamoja-hive-");
Store outbox = Store.File(dir.FullName, 3);
foreach (string weight in new[] { "41.2", "41.5", "40.9" })
{
    await outbox.AppendAsync(weight);
}

int logged = await outbox.CountAsync();
Console.WriteLine($"hive      logged {logged} weights with no link, the most its store holds");

// A full store refuses the next weight rather than dropping one it already
// holds.
try
{
    await outbox.AppendAsync("41.1");
}
catch (PamojaException error)
{
    Console.WriteLine($"hive      was refused a 4th: {error.Message}");
}

// The scale reboots. Its queue is the directory, so it comes back whole and
// in order.
outbox.Dispose();
outbox = Store.File(dir.FullName, 3);
int held = await outbox.CountAsync();
string oldest = (await outbox.PeekTextAsync())!;
Console.WriteLine($"hive      restarted and still holds {held}, oldest first: {oldest}");

// The cellular uplink carries one weight, then drops. A weight leaves the
// queue only once a link has taken it, so what the uplink never took stays,
// in order.
using var cellular = new LoopbackBroker();
using Transport uplink = Transport.Degraded(cellular.Rung(), up: 1, down: 10);
await uplink.ConnectAsync();
int forwarded = 0;
try
{
    await outbox.DrainToAsync(uplink, Topic);
}
catch (PamojaException error)
{
    forwarded = held - await outbox.CountAsync();
    Console.WriteLine($"uplink    forwarded {forwarded}, then failed: {error.Message}");
}

int left = await outbox.CountAsync();
string next = (await outbox.PeekTextAsync())!;
Console.WriteLine($"hive      still holds {left}, oldest first: {next}");

// The beekeeper's gateway comes within reach, and the scale drains the rest
// onto it.
using var visit = new LoopbackBroker();
using LoopbackTransport gateway = visit.Link();
await gateway.ConnectAsync();
await gateway.SubscribeAsync(Topic);
using Transport toGateway = visit.Rung();
await toGateway.ConnectAsync();
await outbox.DrainToAsync(toGateway, Topic);
var took = new List<string>();
for (int weight = 0; weight < left; weight++)
{
    took.Add((await gateway.ReceiveAsync())!.Text);
}

Console.WriteLine($"gateway   took {string.Join(", ", took)} when the beekeeper came by");
int empty = await outbox.CountAsync();
Console.WriteLine($"hive      holds {empty} once the backlog is through");

outbox.Dispose();
dir.Delete(recursive: true);
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sync`](https://crates.io/crates/pamoja-sync) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html), [docs.rs](https://docs.rs/pamoja-sync), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sync) |
| TypeScript | [`@pamoja/sync`](https://www.npmjs.com/package/@pamoja/sync) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sync) |
| Python | [`pamoja-sync`](https://pypi.org/project/pamoja-sync/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sync) |
| C# | [`Pamoja.Sync`](https://www.nuget.org/packages/Pamoja.Sync) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sync) |

## Documentation

- [`Pamoja.Sync` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html), every type in this namespace.
- [The Store and forward guide](https://pamoja.molex.cloud/docs/guides/sync.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
