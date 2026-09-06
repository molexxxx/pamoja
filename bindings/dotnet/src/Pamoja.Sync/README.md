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
// so it lasts as long as the process; Store.File(dir) is the same queue on disk,
// which is what a node uses to survive a reboot with its backlog intact.
using var outbox = Store.Memory();
foreach (string reading in new[] { "20.1", "20.4", "20.2" })
{
    await outbox.AppendAsync(Encoding.UTF8.GetBytes(reading));
}

Console.WriteLine($"queued    {await outbox.CountAsync()} readings with no link");

// Peek reads the oldest record without taking it, so a send that fails part-way
// leaves the queue exactly as it was.
byte[] oldest = (await outbox.PeekAsync())!;
Console.WriteLine(
    $"oldest    {Encoding.UTF8.GetString(oldest)}"
    + $" and still {await outbox.CountAsync()} held");

// The link returns and the queue drains oldest first, in the order the readings
// were taken rather than the order they happen to come back off a buffer.
List<string> drained = [];
while (await outbox.PopAsync() is { } record)
{
    drained.Add(Encoding.UTF8.GetString(record));
}

Console.WriteLine($"drained   {string.Join(", ", drained)}");

// A bounded queue refuses the append that would overflow it. A full store is
// backpressure the caller is told about, not a reading dropped behind its back.
using var bounded = Store.Memory(2);
await bounded.AppendAsync("20.1"u8.ToArray());
await bounded.AppendAsync("20.4"u8.ToArray());
try
{
    await bounded.AppendAsync("20.2"u8.ToArray());
    Console.WriteLine("a full queue took a third reading, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"full      refused the third reading: {error.Message}");
}
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
