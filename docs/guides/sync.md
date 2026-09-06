# Store and forward

A node on a weak link spends part of its life with nowhere to send. The reading
it took is still worth what it was worth, so the choice is between dropping it
and holding it. pamoja holds it: a store is a queue of records that a sender
drains when a link comes back, and the rest of the framework is built to assume
one is there.

Two stores implement the same queue. The in-memory one lasts as long as the
process, which is what a test or a short-lived task wants. The file-backed one
survives a power cut: each record is written and fsynced before the append is
acknowledged, so a node that loses power mid-write comes back with a queue that is
either missing the last record or has it whole, never half of it.

The interesting parts are the ones that are easy to get wrong. A record leaves the
queue only once something has accepted it, so a send that fails part-way loses
nothing. Order is the order the readings were taken. And a bounded store refuses
the append that would overflow it rather than quietly discarding the oldest,
because a caller that is told it is full can decide what to do, and a caller that
is not told cannot.

## What the example does

It queues three readings on a node with no link, peeks at the oldest without
taking it, drains the queue when the link comes back, then fills a two-record
store and offers it a third reading, printing what the queue does at each
stage.

The three readings do not rise in value order, so the drain catches a queue
that sorted them or handed the newest back first. The store here is the
in-memory one; the file-backed store takes the same calls against a directory,
which is what a node uses to hold a backlog across a reboot.

It proves:

- Peek returns the oldest record, `20.1`, and leaves all three readings queued,
  so a send that fails part-way loses nothing.
- The queue drains oldest first, `20.1` then `20.4` then `20.2`, the order the
  readings were taken.
- Popping until it returns nothing leaves the queue empty.
- A full store refuses the third append and still holds two records, so the
  caller is told to back off rather than have the oldest reading dropped to
  make room.

## Rust

<!-- snippet: examples/tests/guides/sync.rs#example -->
From [`examples/tests/guides/sync.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/sync.rs):

```rust
use pamoja_core::Store;
use pamoja_sync::MemoryStore;

// A node with nowhere to send buffers its readings. This queue is held in memory, so
// it lasts as long as the process; FileStore::open(dir) is the same queue on disk,
// which is what a node uses to survive a reboot with its backlog intact.
let mut outbox = MemoryStore::new();
for reading in [b"20.1", b"20.4", b"20.2"] {
    outbox.append(reading).await.expect("the queue takes it");
}
let held = outbox.len().await.expect("a count");
println!("queued    {held} readings with no link");

// Peek reads the oldest record without taking it, so a send that fails part-way leaves
// the queue exactly as it was.
let oldest = outbox.peek().await.expect("a peek").expect("a record");
let still_held = outbox.len().await.expect("a count");
let oldest_reading = String::from_utf8_lossy(&oldest);
println!("oldest    {oldest_reading} and still {still_held} held");

// The link returns and the queue drains oldest first, in the order the readings were
// taken rather than the order they happen to come back off a buffer.
let mut drained = Vec::new();
while let Some(record) = outbox.pop().await.expect("a pop") {
    drained.push(String::from_utf8_lossy(&record).into_owned());
}
println!("drained   {}", drained.join(", "));

// A bounded queue refuses the append that would overflow it. A full store is
// backpressure the caller is told about, not a reading dropped behind its back.
let mut bounded = MemoryStore::with_capacity(2);
bounded.append(b"20.1").await.expect("room");
bounded.append(b"20.4").await.expect("room");
match bounded.append(b"20.2").await {
    Ok(()) => println!("a full queue took a third reading, which should never happen"),
    Err(error) => println!("full      refused the third reading: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/sync.ts#example -->
From [`bindings/node/guides/sync.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sync.ts):

```typescript
import { Store } from '@pamoja/sync'

async function main() {
  // A node with nowhere to send buffers its readings. This queue is held in memory, so it
  // lasts as long as the process; Store.file(dir) is the same queue on disk, which is what
  // a node uses to survive a reboot with its backlog intact.
  const outbox = Store.memory()
  for (const reading of ['20.1', '20.4', '20.2']) {
    await outbox.append(Buffer.from(reading))
  }
  console.log(`queued    ${await outbox.len()} readings with no link`)

  // Peek reads the oldest record without taking it, so a send that fails part-way leaves
  // the queue exactly as it was.
  const oldest = (await outbox.peek())!
  console.log(`oldest    ${oldest.toString()} and still ${await outbox.len()} held`)

  // The link returns and the queue drains oldest first, in the order the readings were
  // taken rather than the order they happen to come back off a buffer.
  const drained: string[] = []
  for (let record = await outbox.pop(); record !== null; record = await outbox.pop()) {
    drained.push(record.toString())
  }
  console.log(`drained   ${drained.join(', ')}`)

  // A bounded queue refuses the append that would overflow it. A full store is
  // backpressure the caller is told about, not a reading dropped behind its back.
  const bounded = Store.memory(2)
  await bounded.append(Buffer.from('20.1'))
  await bounded.append(Buffer.from('20.4'))
  try {
    await bounded.append(Buffer.from('20.2'))
    console.log('a full queue took a third reading, which should never happen')
  } catch (error) {
    console.log(`full      refused the third reading: ${(error as Error).message}`)
  }

  return { oldest, drained, left: await outbox.len(), held: await bounded.len() }
}

main()
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/sync.py#example -->
From [`bindings/python/guides/sync.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sync.py):

```python
import asyncio

from pamoja.core import PamojaError
from pamoja.sync import Store


async def main() -> None:
    # A node with nowhere to send buffers its readings. This queue is held in memory, so it
    # lasts as long as the process; Store.file(dir) is the same queue on disk, which is what
    # a node uses to survive a reboot with its backlog intact.
    outbox = Store.memory()
    for reading in (b"20.1", b"20.4", b"20.2"):
        await outbox.append(reading)
    print(f"queued    {await outbox.len()} readings with no link")

    # Peek reads the oldest record without taking it, so a send that fails part-way leaves
    # the queue exactly as it was.
    oldest = await outbox.peek()
    print(f"oldest    {oldest.decode()} and still {await outbox.len()} held")

    # The link returns and the queue drains oldest first, in the order the readings were
    # taken rather than the order they happen to come back off a buffer.
    drained = []
    while (record := await outbox.pop()) is not None:
        drained.append(record.decode())
    print(f"drained   {', '.join(drained)}")

    # A bounded queue refuses the append that would overflow it. A full store is
    # backpressure the caller is told about, not a reading dropped behind its back.
    bounded = Store.memory(capacity=2)
    await bounded.append(b"20.1")
    await bounded.append(b"20.4")
    try:
        await bounded.append(b"20.2")
        print("a full queue took a third reading, which should never happen")
    except PamojaError as error:
        print(f"full      refused the third reading: {error}")

    return oldest, drained, await outbox.len(), await bounded.len()


oldest, drained, left, held = asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SyncGuide.cs#example -->
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
<!-- end -->

## Reference

<!-- table: reference sync -->
- Rust: [`pamoja-sync`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sync)
- TypeScript: [`@pamoja/sync`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sync)
- Python: [`pamoja.sync`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sync)
- C#: [`Pamoja.Sync`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sync)
<!-- end -->
