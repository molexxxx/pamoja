# Pamoja.Sync

Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sync.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
// A node with nowhere to send buffers its readings. This queue is held in memory,
// so it lasts as long as the process; Store.File(dir) is the same queue on disk.
using var outbox = Store.Memory();
foreach (var reading in new[] { "20.1", "20.4", "20.2" })
{
    await outbox.AppendAsync(System.Text.Encoding.UTF8.GetBytes(reading));
}

Expect(await outbox.CountAsync() == 3, "three readings are queued");

// Peek reads the oldest record without taking it, so a send that fails part-way
// leaves the queue exactly as it was.
Expect(
    (await outbox.PeekAsync())?.AsSpan().SequenceEqual("20.1"u8) == true,
    "peek returns the oldest record");
Expect(await outbox.CountAsync() == 3, "and leaves it in the queue");

// The link returns and the queue drains oldest first, in the order the readings
// were taken rather than the order they happen to come off a heap.
var drained = new List<string>();
while (await outbox.PopAsync() is { } record)
{
    drained.Add(System.Text.Encoding.UTF8.GetString(record));
}

Expect(drained.SequenceEqual(["20.1", "20.4", "20.2"]), "drained oldest first");
Expect(await outbox.CountAsync() == 0, "leaving the queue empty");

// A bounded store refuses the append that would overflow it. A full queue is
// backpressure the caller is told about, not a reading dropped behind its back.
using var bounded = Store.Memory(2);
await bounded.AppendAsync("20.1"u8.ToArray());
await bounded.AppendAsync("20.4"u8.ToArray());

bool refused = false;
try
{
    await bounded.AppendAsync("20.2"u8.ToArray());
}
catch (PamojaException)
{
    refused = true;
}

Expect(refused, "a full store refuses rather than drops");
Expect(await bounded.CountAsync() == 2, "and keeps what it already holds");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sync`](https://crates.io/crates/pamoja-sync) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html), [docs.rs](https://docs.rs/pamoja-sync) |
| TypeScript | [`@pamoja/sync`](https://www.npmjs.com/package/@pamoja/sync) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html) |
| Python | [`pamoja-sync`](https://pypi.org/project/pamoja-sync/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html) |
| C# | [`Pamoja.Sync`](https://www.nuget.org/packages/Pamoja.Sync) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html) |

## Documentation

- [`Pamoja.Sync` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html), every type in this namespace.
- [The Store and forward guide](https://pamoja.molex.cloud/docs/guides/sync.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
