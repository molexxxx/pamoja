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

It queues three readings in memory, reads the oldest without taking it, drains
the queue, and then fills a two-record store and appends one more.

It proves:

- Appending queues a record and the count reflects it.
- Peeking returns the oldest record and leaves it in the queue.
- Draining returns records oldest first, in the order they were appended.
- A bounded store raises on the append that would overflow it and keeps what it
  already holds.

## Rust

<!-- snippet: examples/tests/guides/sync.rs#example -->
From [`examples/tests/guides/sync.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/sync.rs):

```rust
use pamoja_core::Store;
use pamoja_sync::MemoryStore;

// A node with nowhere to send buffers its readings. This queue is held in memory, so
// it lasts as long as the process; FileStore::open(dir) is the same queue on disk.
let mut outbox = MemoryStore::new();
for reading in [b"20.1", b"20.4", b"20.2"] {
    outbox.append(reading).await.unwrap();
}
assert_eq!(outbox.len().await.unwrap(), 3);

// Peek reads the oldest record without taking it, so a send that fails part-way leaves
// the queue exactly as it was.
assert_eq!(outbox.peek().await.unwrap(), Some(b"20.1".to_vec()));
assert_eq!(outbox.len().await.unwrap(), 3);

// The link returns and the queue drains oldest first, in the order the readings were
// taken rather than the order they happen to come back off a buffer.
let mut drained = Vec::new();
while let Some(record) = outbox.pop().await.unwrap() {
    drained.push(record);
}
assert_eq!(
    drained,
    [b"20.1".to_vec(), b"20.4".to_vec(), b"20.2".to_vec()]
);
assert_eq!(outbox.len().await.unwrap(), 0);

// A bounded queue refuses the append that would overflow it. A full store is
// backpressure the caller is told about, not a reading dropped behind its back.
let mut bounded = MemoryStore::with_capacity(2);
bounded.append(b"20.1").await.unwrap();
bounded.append(b"20.4").await.unwrap();
assert!(bounded.append(b"20.2").await.is_err());
assert_eq!(bounded.len().await.unwrap(), 2);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/sync.ts#example -->
From [`bindings/node/guides/sync.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sync.ts):

```typescript
import assert from 'node:assert/strict'

import { Store } from '@pamoja/sync'

async function main() {
  // A node with nowhere to send buffers its readings. This queue is held in memory, so
  // it lasts as long as the process; Store.file(dir) is the same queue on disk.
  const outbox = Store.memory()
  for (const reading of ['20.1', '20.4', '20.2']) {
    await outbox.append(Buffer.from(reading))
  }
  assert.equal(await outbox.len(), 3)

  // Peek reads the oldest record without taking it, so a send that fails part-way leaves
  // the queue exactly as it was.
  assert.equal((await outbox.peek())?.toString(), '20.1')
  assert.equal(await outbox.len(), 3)

  // The link returns and the queue drains oldest first, in the order the readings were
  // taken rather than the order they happen to come back off a buffer.
  const drained: string[] = []
  let record = await outbox.pop()
  while (record !== null) {
    drained.push(record.toString())
    record = await outbox.pop()
  }
  assert.deepEqual(drained, ['20.1', '20.4', '20.2'])
  assert.equal(await outbox.len(), 0)

  // A bounded queue refuses the append that would overflow it. A full store is
  // backpressure the caller is told about, not a reading dropped behind its back.
  const bounded = Store.memory(2)
  await bounded.append(Buffer.from('20.1'))
  await bounded.append(Buffer.from('20.4'))
  await assert.rejects(() => bounded.append(Buffer.from('20.2')))
  assert.equal(await bounded.len(), 2)
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
    # A node with nowhere to send buffers its readings. This queue is held in memory, so
    # it lasts as long as the process; Store.file(dir) is the same queue on disk.
    outbox = Store.memory()
    for reading in (b"20.1", b"20.4", b"20.2"):
        await outbox.append(reading)
    assert await outbox.len() == 3

    # Peek reads the oldest record without taking it, so a send that fails part-way
    # leaves the queue exactly as it was.
    assert await outbox.peek() == b"20.1"
    assert await outbox.len() == 3

    # The link returns and the queue drains oldest first, in the order the readings were
    # taken rather than the order they happen to come off a heap.
    drained = []
    while (record := await outbox.pop()) is not None:
        drained.append(record)
    assert drained == [b"20.1", b"20.4", b"20.2"]
    assert await outbox.len() == 0

    # A bounded store refuses the append that would overflow it. A full queue is
    # backpressure the caller is told about, not a reading dropped behind its back.
    bounded = Store.memory(2)
    await bounded.append(b"20.1")
    await bounded.append(b"20.4")
    try:
        await bounded.append(b"20.2")
    except PamojaError:
        pass
    else:
        raise AssertionError("a full store should refuse rather than drop")
    assert await bounded.len() == 2


asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SyncGuide.cs#example -->
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
<!-- end -->

## Reference

<!-- table: reference sync -->
- Rust: [`pamoja-sync`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html)
- TypeScript: [`@pamoja/sync`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html)
- Python: [`pamoja.sync`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html)
- C#: [`Pamoja.Sync`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html)
<!-- end -->
