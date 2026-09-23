# Store and forward

A node on a weak link spends part of its life with nowhere to send. The reading
it took is still worth what it was worth, so the choice is between dropping it
and holding it. pamoja holds it: a store is a queue of records that a sender
drains when a link comes back, and the rest of the framework is built to assume
one is there.

Two stores implement the same queue. The in-memory one lasts as long as the
process, which is what a test or a short-lived task wants. The file-backed one
keeps each record in a file of its own, written and flushed before the append
returns and only then renamed into place, so a node that loses power comes back with
every record it had, and never with one half written.

The interesting parts are the ones that are easy to get wrong. A record leaves the
queue only once something has accepted it, so a send that fails part-way loses
nothing. Order is the order the readings were taken. And a bounded store refuses
the append that would overflow it rather than quietly discarding the oldest,
because a caller that is told it is full can decide what to do, and a caller that
is not told cannot.

## What the example does

It is a scale under a beehive in a remote apiary, logging the hive's weight to a
queue on its SD card that holds three weights at most. It logs three with no link,
is refused a fourth, reboots, and starts to drain over a cellular uplink that carries
one weight and drops. Later the beekeeper's gateway comes within reach, and the scale
drains the rest onto it. The store is the file-backed one, in a temporary directory,
and the uplink is a loopback link inside the simulator's degraded link, so the drop is
the same transport error a real one raises.

It proves:

- A bounded store holds three weights and refuses the fourth with `io error: store is
  at capacity`, so no weight it already had is dropped to make room.
- The store is its directory: reopened after the reboot, it holds all 3, oldest first,
  `41.2`.
- A drain sends a record before removing it. The uplink takes 1, fails on the next
  with `transport error: link unreachable`, and the 2 it never took stay, oldest
  first, `41.5`.
- A later drain onto another link delivers the rest in the order they were weighed,
  `41.5` then `40.9`, and leaves the store empty.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example sync" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example sync</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- sync" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- sync</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/sync.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/sync.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- sync" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- sync</code></div>
</div>
<!-- end -->

## Rust

In Rust, `MemoryStore::new()` and `MemoryStore::with_capacity(n)` are the in-memory queue, and
`FileStore::open(dir)` and `FileStore::open_with_capacity(dir, n)` the one on disk; each
implements the core `Store` trait. `append(&bytes)` and `append_text(text)` add a record,
`peek()` and `peek_text()` read the oldest without taking it, `pop()` and `pop_text()` take it,
and `len()` and `is_empty()` count what is held. `drain_to(&mut store, &mut link, topic)` sends
every record to one topic, oldest first, removing each only once the link has taken it, and
returns how many went; at the first failure it stops and returns the link's error. A ladder
drains its own store with `flush`. On Linux and other Unix systems, a `FileStore` append that
returned has reached the disk, directory entry and all; on Windows the record's contents have,
and the directory entry is left to the file system.

<!-- snippet: examples/guides/sync.rs#example -->
From [`examples/guides/sync.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/sync.rs):

```rust
use pamoja_core::{Receive, Store, Transport};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_sim::DegradedLink;
use pamoja_sync::{drain_to, FileStore};

// The scale logs its weight to a queue on its SD card, bounded so a long outage
// cannot fill the card. The directory is the queue, so the scale can lose power at
// any moment and lose nothing it logged.
let dir = std::env::temp_dir().join(format!("pamoja-hive-{}", std::process::id()));
let topic = "apiary/hive-3/weight";
let mut outbox = FileStore::open_with_capacity(&dir, 3)?;
for weight in ["41.2", "41.5", "40.9"] {
    outbox.append_text(weight).await?;
}
let logged = outbox.len().await?;
println!("hive      logged {logged} weights with no link, the most its store holds");

// A full store refuses the next weight rather than dropping one it already holds.
let refused = outbox.append_text("41.1").await.expect_err("a full store");
println!("hive      was refused a 4th: {refused}");

// The scale reboots. Its queue is the directory, so it comes back whole and in
// order.
drop(outbox);
let mut outbox = FileStore::open_with_capacity(&dir, 3)?;
let held = outbox.len().await?;
let oldest = outbox.peek_text().await?.expect("a weight");
println!("hive      restarted and still holds {held}, oldest first: {oldest}");

// The cellular uplink carries one weight, then drops. A weight leaves the queue
// only once a link has taken it, so what the uplink never took stays, in order.
let cellular = LoopbackBroker::new();
let mut uplink = DegradedLink::new(LoopbackTransport::new(cellular)).intermittent(1, 10);
uplink.connect().await?;
let dropped = drain_to(&mut outbox, &mut uplink, topic)
    .await
    .expect_err("the uplink drops");
let forwarded = held - outbox.len().await?;
println!("uplink    forwarded {forwarded}, then failed: {dropped}");
let left = outbox.len().await?;
let next = outbox.peek_text().await?.expect("a weight");
println!("hive      still holds {left}, oldest first: {next}");

// The beekeeper's gateway comes within reach, and the scale drains the rest onto it.
let visit = LoopbackBroker::new();
let mut gateway = LoopbackTransport::new(visit.clone());
gateway.connect().await?;
gateway.subscribe(topic).await?;
let mut to_gateway = LoopbackTransport::new(visit);
to_gateway.connect().await?;
drain_to(&mut outbox, &mut to_gateway, topic).await?;
let mut took = Vec::new();
for _ in 0..left {
    let weight = gateway.recv().await?.expect("a weight");
    took.push(weight.text()?.to_owned());
}
println!(
    "gateway   took {} when the beekeeper came by",
    took.join(", ")
);
let empty = outbox.len().await?;
println!("hive      holds {empty} once the backlog is through");
```
<!-- end -->

## TypeScript

In TypeScript, `Store.memory(capacity?)` and `Store.file(dir, capacity?)` from `@pamoja/sync`
make the two stores, with no bound unless a capacity is given. Every call returns a promise:
`append(bufferOrText)`, `peek()` and `pop()`, which resolve with a `Buffer` or with `null` once
the store is empty, `peekText()` and `popText()`, which resolve with a string or `null`, and
`len()`. `drainTo(transport, topic)` sends every record to one topic over a `Transport`, oldest
first, and resolves with how many went, or rejects with the transport's error and leaves the
records it did not send. A store handed to a ladder belongs to the ladder from then on.

<!-- snippet: bindings/node/guides/sync.ts#example -->
From [`bindings/node/guides/sync.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sync.ts):

```typescript
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { Transport } from '@pamoja/core'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

const TOPIC = 'apiary/hive-3/weight'

async function main() {
  // The scale logs its weight to a queue on its SD card, bounded so a long outage cannot
  // fill the card. The directory is the queue, so the scale can lose power at any moment
  // and lose nothing it logged.
  const dir = mkdtempSync(join(tmpdir(), 'pamoja-hive-'))
  let outbox = Store.file(dir, 3)
  for (const weight of ['41.2', '41.5', '40.9']) {
    await outbox.append(weight)
  }
  const logged = await outbox.len()
  console.log(`hive      logged ${logged} weights with no link, the most its store holds`)

  // A full store refuses the next weight rather than dropping one it already holds.
  try {
    await outbox.append('41.1')
  } catch (error) {
    console.log(`hive      was refused a 4th: ${(error as Error).message}`)
  }

  // The scale reboots. Its queue is the directory, so it comes back whole and in order.
  outbox = Store.file(dir, 3)
  const held = await outbox.len()
  const oldest = (await outbox.peekText())!
  console.log(`hive      restarted and still holds ${held}, oldest first: ${oldest}`)

  // The cellular uplink carries one weight, then drops. A weight leaves the queue only
  // once a link has taken it, so what the uplink never took stays, in order.
  const cellular = new LoopbackBroker()
  const uplink = Transport.degraded(cellular.rung(), { up: 1, down: 10 })
  await uplink.connect()
  let forwarded = 0
  try {
    await outbox.drainTo(uplink, TOPIC)
  } catch (error) {
    forwarded = held - (await outbox.len())
    console.log(`uplink    forwarded ${forwarded}, then failed: ${(error as Error).message}`)
  }
  const left = await outbox.len()
  const next = (await outbox.peekText())!
  console.log(`hive      still holds ${left}, oldest first: ${next}`)

  // The beekeeper's gateway comes within reach, and the scale drains the rest onto it.
  const visit = new LoopbackBroker()
  const gateway = visit.link()
  await gateway.connect()
  await gateway.subscribe(TOPIC)
  const toGateway = visit.rung()
  await toGateway.connect()
  await outbox.drainTo(toGateway, TOPIC)
  const took: string[] = []
  for (let weight = 0; weight < left; weight += 1) {
    took.push((await gateway.recv())!.text!)
  }
  console.log(`gateway   took ${took.join(', ')} when the beekeeper came by`)
  const empty = await outbox.len()
  console.log(`hive      holds ${empty} once the backlog is through`)

  rmSync(dir, { recursive: true })
  return { counts: [logged, held, forwarded, left, empty], oldest, next, took }
}

main()
```
<!-- end -->

## Python

In Python, `Store.memory(capacity=0)` and `Store.file(dir, capacity=0)` from `pamoja.sync` make
the two stores, 0 meaning no bound. The calls are coroutines: `append(bytes_or_text)`, `peek()`
and `pop()`, which return `bytes` or `None` once the store is empty, `peek_text()` and
`pop_text()`, which return `str` or `None`, and `len()`. `drain_to(transport, topic)` sends
every record to one topic over a `Transport`, oldest first, and returns how many went, or raises
the transport's error and leaves the records it did not send. A failure raises `PamojaError`.

<!-- snippet: bindings/python/guides/sync.py#example -->
From [`bindings/python/guides/sync.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sync.py):

```python
import asyncio
import tempfile

from pamoja.core import PamojaError, Transport
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store

TOPIC = "apiary/hive-3/weight"


async def main(folder: str) -> None:
    # The scale logs its weight to a queue on its SD card, bounded so a long outage
    # cannot fill the card. The directory is the queue, so the scale can lose power at
    # any moment and lose nothing it logged.
    outbox = Store.file(folder, 3)
    for weight in ("41.2", "41.5", "40.9"):
        await outbox.append(weight)
    logged = await outbox.len()
    print(f"hive      logged {logged} weights with no link, the most its store holds")

    # A full store refuses the next weight rather than dropping one it already holds.
    try:
        await outbox.append("41.1")
    except PamojaError as error:
        print(f"hive      was refused a 4th: {error}")

    # The scale reboots. Its queue is the directory, so it comes back whole and in
    # order.
    outbox = Store.file(folder, 3)
    held = await outbox.len()
    oldest = await outbox.peek_text()
    print(f"hive      restarted and still holds {held}, oldest first: {oldest}")

    # The cellular uplink carries one weight, then drops. A weight leaves the queue only
    # once a link has taken it, so what the uplink never took stays, in order.
    cellular = LoopbackBroker()
    uplink = Transport.degraded(cellular.rung(), up=1, down=10)
    await uplink.connect()
    forwarded = 0
    try:
        await outbox.drain_to(uplink, TOPIC)
    except PamojaError as error:
        forwarded = held - await outbox.len()
        print(f"uplink    forwarded {forwarded}, then failed: {error}")
    left = await outbox.len()
    next_weight = await outbox.peek_text()
    print(f"hive      still holds {left}, oldest first: {next_weight}")

    # The beekeeper's gateway comes within reach, and the scale drains the rest onto
    # it.
    visit = LoopbackBroker()
    gateway = visit.link()
    await gateway.connect()
    await gateway.subscribe(TOPIC)
    to_gateway = visit.rung()
    await to_gateway.connect()
    await outbox.drain_to(to_gateway, TOPIC)
    took = [(await gateway.recv()).text for _ in range(left)]
    print(f"gateway   took {', '.join(took)} when the beekeeper came by")
    empty = await outbox.len()
    print(f"hive      holds {empty} once the backlog is through")

    return (logged, held, forwarded, left, empty), oldest, next_weight, took


with tempfile.TemporaryDirectory(prefix="pamoja-hive-") as folder:
    counts, oldest, next_weight, took = asyncio.run(main(folder))
```
<!-- end -->

## C#

In C#, `Store.Memory(capacity)` and `Store.File(dir, capacity)` in `Pamoja.Sync` make the two
stores, 0 meaning no bound, and each is disposable. `AppendAsync(textOrBytes)` adds a record,
`PeekAsync()` and `PopAsync()` give its bytes or `null` once the store is empty,
`PeekTextAsync()` and `PopTextAsync()` its text, and `CountAsync()` counts what is held.
`DrainToAsync(transport, topic)` sends every record to one topic over a `Transport`, oldest
first, and returns how many went, or throws the transport's error and leaves the records it did
not send. Calls on one store run one at a time, and a failure throws `PamojaException`.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SyncGuide.cs#example -->
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
<!-- end -->

## Values at a glance

**The two stores:**

| Store | Rust | TypeScript | Python | C# | Lasts |
| --- | --- | --- | --- | --- | --- |
| memory | `MemoryStore::new()` | `Store.memory()` | `Store.memory()` | `Store.Memory()` | as long as the process |
| memory, bounded | `MemoryStore::with_capacity(n)` | `Store.memory(n)` | `Store.memory(n)` | `Store.Memory(n)` | as long as the process |
| file | `FileStore::open(dir)` | `Store.file(dir)` | `Store.file(dir)` | `Store.File(dir)` | across a restart or a power cut |
| file, bounded | `FileStore::open_with_capacity(dir, n)` | `Store.file(dir, n)` | `Store.file(dir, n)` | `Store.File(dir, n)` | across a restart or a power cut |

**The calls:**

| What | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| add a record | `append(&bytes)`, `append_text(text)` | `append(bufferOrText)` | `append(bytes_or_text)` | `AppendAsync(textOrBytes)` |
| read the oldest | `peek()`, `peek_text()` | `peek()`, `peekText()` | `peek()`, `peek_text()` | `PeekAsync()`, `PeekTextAsync()` |
| take the oldest | `pop()`, `pop_text()` | `pop()`, `popText()` | `pop()`, `pop_text()` | `PopAsync()`, `PopTextAsync()` |
| count | `len()`, `is_empty()` | `len()` | `len()` | `CountAsync()` |
| drain onto a link | `drain_to(&mut store, &mut link, topic)` | `drainTo(transport, topic)` | `drain_to(transport, topic)` | `DrainToAsync(transport, topic)` |

**How the file store keeps a record.** Each record is a file named for its place in the queue,
such as `00000000000000000007.rec`:

| Step | What happens | If the power goes at this point |
| --- | --- | --- |
| an append begins | the record is written to a `.rec.tmp` file and flushed to disk | the append never returned, and the partial file is deleted when the store is next opened |
| the append ends | the file is renamed to `.rec`, and on Unix the directory is flushed | the record is there when the store is next opened |
| a pop | the oldest file is read, then deleted | if the delete had not reached the disk, the record comes back once more |

A drain is a peek, a send, and a pop for each record, so delivery is at least once: a record the
link took just before a cut can be sent again after it, but none is lost.

## When it goes wrong

What a store says:

| What happened | The message | What to check |
| --- | --- | --- |
| an append to a full store | `io error: store is at capacity` | drain, raise the capacity, or decide what to drop |
| a text read of a record that is not UTF-8 | `codec error: the record is not UTF-8 text` | read it as bytes with `peek` or `pop` |
| a file store on a directory it cannot write | `io error:` and the operating system's reason | the path, its permissions, and the free space |
| a drain whose link failed | the link's own error, such as `transport error: link unreachable` | nothing: the records it did not send are still there to drain later |
| a store used after it went to a ladder | `this store was already given to a ladder` | the ladder owns it; drain through the ladder's `flush` |

The mistakes that cost an afternoon:

- **Some records arrive twice after a crash.** Delivery is at least once. Give each reading an
  identity, a timestamp or a sequence number, and have the far side skip one it has already
  taken.
- **Two stores on one directory lose track of each other's records.** Each store keeps its own
  count of what the directory holds. Open one store per directory, and reopen it only once the
  last one is done with.
- **The card fills during a long outage.** An unbounded file store grows until the disk is full,
  and a full disk breaks more than the store. Bound it for the longest outage the node has to
  ride out.
- **The newest reading is refused, not the oldest.** A full store refuses the append. If the
  newest matters more than the oldest, pop the oldest first, deliberately, and say so in the
  code.
- **Everything drains to one topic.** A drain publishes every record to the topic it is given.
  Keep one store per topic, or hand the node a ladder, which keeps each record's topic with it.
- **The backlog survives a reboot on the bench but not in the field.** A memory store lasts as
  long as the process. A node that has to keep its backlog through a power cut uses a file
  store.

## Where next

<!-- table: next sync -->
- [Transport ladder](ladder.md): Cheapest reachable link first, buffering to a store when every link is down.
- [Codecs](codec.md): CBOR, JSON, and raw codecs behind one trait, and batch packing for metered links.
- [Power](power.md): Duty cycling and an energy-aware governor that stretches work as the battery drains.
- Also in Transports and testing: [MQTT](mqtt.md), [CoAP](coap.md), [Loopback](loopback.md), [Event bus](bus.md), [Engine surface](transport.md), [Your own link](link.md), [Simulators](sim.md).
<!-- end -->

## Reference

<!-- table: reference sync -->
- Rust: [`pamoja-sync`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sync)
- TypeScript: [`@pamoja/sync`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sync)
- Python: [`pamoja.sync`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sync)
- C#: [`Pamoja.Sync`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sync)
<!-- end -->
