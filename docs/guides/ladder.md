# Transport ladder

Most nodes that matter have more than one way to reach home, and the ways are not
equally priced. A mesh hop to the neighbor costs almost nothing. A cellular
backhaul costs money per byte and a chunk of the battery. Satellite costs more
again. What a node wants is the cheapest link that is actually working right now,
decided per message, without the application knowing which one it got.

A ladder is that decision. Rungs are added cheapest first and tried in order, and
the first one that accepts a message wins. If none will take it, the message goes
into a store rather than being lost, and a later flush drains the backlog when a
rung comes back.

The ordering rule is what keeps the data honest. A record leaves the store only
once a rung has accepted it, so a flush against a link that is still down forwards
nothing and loses nothing. And a reading taken while a backlog exists joins the
back of it rather than overtaking, so what arrives upstream is in the order it was
measured.

A ladder is a link in its own right, both ways. Anything written against one
transport, a profile node, a rule, a hand-written loop, runs over a ladder unchanged
and gains the buffering. A subscription placed on the ladder goes onto every rung
that listens, and a receive hands up whichever rung delivers first, so a command
reaches the node over whatever link happens to be up. A link that only sends, such
as a satellite messenger or a LoRa uplink, goes on with `uplink` rather than `rung`,
and the ladder never subscribes it or listens on it.

## What the example does

It follows a fishing vessel out of harbor. Three networks are in play, cheapest
first: the harbor's wifi, the coast's cellular network, and a satellite that only
sends. Each is a loopback broker with an office ashore listening on it, so which
network carried a report is read off that office. The example takes each network out
of reach with the broker's reachable switch, the way the vessel's distance from shore
would, so no link is written to fail.

It proves:

- Rungs are tried in the order they were added: in the harbor, the wifi carries
  report 1.
- A link out of reach passes a report down to the next: the coast carries report 2
  and the satellite report 3, with nothing in the node's code about which network is
  gone.
- With every link out of reach, report 4 is buffered rather than lost, and the ladder
  counts the 1 record it holds.
- A flush with every link still out of reach forwards 0 and leaves that record where
  it was.
- Back in reach of the coast, a flush forwards 1, the coast's office receives report
  4, and 0 are left, so the backlog went out exactly once.
- An order published ashore on the coast network arrives through the ladder's own
  receive.
- A wait for orders with a 50 ms limit gives up without taking anything, and report 5
  goes out between waits, over the coast.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example ladder" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example ladder</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- ladder" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- ladder</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/ladder.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/ladder.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- ladder" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- ladder</code></div>
</div>
<!-- end -->

## Rust

In Rust, `TransportLadder::new(store)` takes the store it buffers into, a `MemoryStore` or a
`FileStore` from `pamoja-sync`, and `rung(link)` and `uplink(link)` add links as a builder,
cheapest first: `rung` for a link that implements `Transport` and `Receive`, `uplink` for one
that needs only `Transport`. `connect` connects every rung that is down and places the ladder's
filters on each listening one. `send` and `send_text` return `Delivery::Sent` or
`Delivery::Buffered`, `flush` returns how many buffered records went out, and `buffered` how many
wait. The ladder implements `Transport` and `Receive` itself, so `subscribe` and `recv` work as on
any link. Every call takes `&mut self`, so a ladder does one thing at a time. `recv` is
cancel-safe when every rung's receive is, as the shipped links' are, so `tokio::time::timeout`
around it, or a `select!` against a timer, lets a node wait for commands and report between
waits.

<!-- snippet: examples/guides/ladder.rs#example -->
From [`examples/guides/ladder.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/ladder.rs):

```rust
use std::time::Duration;

use pamoja_core::{Receive, Transport};
use pamoja_ladder::{Delivery, TransportLadder};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_sync::MemoryStore;

// Three networks a vessel can reach: the harbor's wifi, the coast's cellular
// network, and a satellite. Each is a broker with an office ashore listening on
// it, so which one carried a report is read off that office rather than assumed.
let harbor = LoopbackBroker::new();
let coast = LoopbackBroker::new();
let sky = LoopbackBroker::new();
let mut harbor_office = LoopbackTransport::new(harbor.clone());
let mut coast_office = LoopbackTransport::new(coast.clone());
let mut sky_office = LoopbackTransport::new(sky.clone());
for office in [&mut harbor_office, &mut coast_office, &mut sky_office] {
    office.connect().await?;
    office.subscribe("vessel/7/report").await?;
}

// Rungs go on cheapest first. The satellite only sends, so it goes on as an
// uplink, which the ladder never subscribes or listens on.
let mut ladder = TransportLadder::new(MemoryStore::new())
    .rung(LoopbackTransport::new(harbor.clone()))
    .rung(LoopbackTransport::new(coast.clone()))
    .uplink(LoopbackTransport::new(sky.clone()));
ladder.connect().await?;
ladder.subscribe("vessel/7/orders").await?;

// In the harbor, the cheapest link takes the report.
let first = ladder.send_text("vessel/7/report", "report 1").await?;
let taken = harbor_office.recv().await?.expect("a report");
println!("harbor    carried {}", taken.text()?);

// Past the breakwater the wifi is out of reach and the report falls through to
// the coast, and further out to the satellite.
harbor.set_reachable(false);
ladder.send_text("vessel/7/report", "report 2").await?;
let taken = coast_office.recv().await?.expect("a report");
println!(
    "coast     carried {}, with the harbor out of reach",
    taken.text()?
);
coast.set_reachable(false);
ladder.send_text("vessel/7/report", "report 3").await?;
let taken = sky_office.recv().await?.expect("a report");
println!(
    "sky       carried {}, with the coast out of reach too",
    taken.text()?
);

// In a storm nothing is in reach, and the report waits in the store rather than
// being lost.
sky.set_reachable(false);
let stormy = ladder.send_text("vessel/7/report", "report 4").await?;
let waiting = ladder.buffered().await?;
println!("vessel    buffered report 4 with every link out of reach, {waiting} waiting");

// A flush with every link still out of reach forwards nothing, because a record
// leaves the store only once a link has taken it.
let forwarded = ladder.flush().await?;
let still = ladder.buffered().await?;
println!(
    "vessel    flushed {forwarded} while every link was out of reach, {still} still waiting"
);

// Back in reach of the coast, a flush sends the backlog, oldest first.
coast.set_reachable(true);
let forwarded = ladder.flush().await?;
let taken = coast_office.recv().await?.expect("a report");
let left = ladder.buffered().await?;
println!(
    "coast     carried {} on a flush of {forwarded}, {left} waiting",
    taken.text()?
);

// Orders from shore come back over whichever listening link is in reach.
coast_office
    .send_text("vessel/7/orders", "return to port")
    .await?;
let order = ladder.recv().await?.expect("an order");
println!("vessel    took {} over the coast network", order.text()?);

// A ladder does one thing at a time, so a vessel that listens and reports waits
// for orders with a limit and reports between waits.
let quiet = Duration::from_millis(50);
if tokio::time::timeout(quiet, ladder.recv()).await.is_err() {
    println!(
        "vessel    heard nothing more from shore within {} ms",
        quiet.as_millis()
    );
}
ladder.send_text("vessel/7/report", "report 5").await?;
let taken = coast_office.recv().await?.expect("a report");
println!("coast     carried {} between waits", taken.text()?);
```
<!-- end -->

## TypeScript

In TypeScript, `new Ladder(store)` from `@pamoja/ladder` takes a store from `@pamoja/sync`,
`Store.memory(capacity?)` or `Store.file(dir)`, and consumes it. `await ladder.rung(transport)`
and `await ladder.uplink(transport)` add links, consuming each, so a link on a ladder cannot be
driven directly afterwards. `send(topic, bufferOrText)` resolves with `Delivery.Sent` or
`Delivery.Buffered`, `flush()` with how many records went out, and `buffered()` with how many
wait. `subscribe(filter)` places a filter on every listening rung, and `recv(timeoutMs?)`
resolves with the next message from any of them, or rejects with `no message arrived within N
ms` when its limit runs out. Calls on one ladder run one at a time, so a send waits behind a
receive in progress.

<!-- snippet: bindings/node/guides/ladder.ts#example -->
From [`bindings/node/guides/ladder.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ladder.ts):

```typescript
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

const REPORT = 'vessel/7/report'
const ORDERS = 'vessel/7/orders'
const QUIET_MS = 50

async function main() {
  // Three networks a vessel can reach: the harbor's wifi, the coast's cellular network,
  // and a satellite. Each is a broker with an office ashore listening on it, so which one
  // carried a report is read off that office rather than assumed.
  const harbor = new LoopbackBroker()
  const coast = new LoopbackBroker()
  const sky = new LoopbackBroker()
  const harborOffice = harbor.link()
  const coastOffice = coast.link()
  const skyOffice = sky.link()
  for (const office of [harborOffice, coastOffice, skyOffice]) {
    await office.connect()
    await office.subscribe(REPORT)
  }

  // Rungs go on cheapest first. The satellite only sends, so it goes on as an uplink,
  // which the ladder never subscribes or listens on.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(harbor.rung())
  await ladder.rung(coast.rung())
  await ladder.uplink(sky.rung())
  await ladder.connect()
  await ladder.subscribe(ORDERS)

  // In the harbor, the cheapest link takes the report.
  const first = await ladder.send(REPORT, 'report 1')
  console.log(`harbor    carried ${(await harborOffice.recv())!.text!}`)

  // Past the breakwater the wifi is out of reach and the report falls through to the
  // coast, and further out to the satellite.
  harbor.reachable = false
  await ladder.send(REPORT, 'report 2')
  console.log(`coast     carried ${(await coastOffice.recv())!.text!}, with the harbor out of reach`)
  coast.reachable = false
  await ladder.send(REPORT, 'report 3')
  console.log(`sky       carried ${(await skyOffice.recv())!.text!}, with the coast out of reach too`)

  // In a storm nothing is in reach, and the report waits in the store rather than being
  // lost.
  sky.reachable = false
  const stormy = await ladder.send(REPORT, 'report 4')
  const waiting = await ladder.buffered()
  console.log(`vessel    buffered report 4 with every link out of reach, ${waiting} waiting`)

  // A flush with every link still out of reach forwards nothing, because a record leaves
  // the store only once a link has taken it.
  const idle = await ladder.flush()
  const still = await ladder.buffered()
  console.log(`vessel    flushed ${idle} while every link was out of reach, ${still} still waiting`)

  // Back in reach of the coast, a flush sends the backlog, oldest first.
  coast.reachable = true
  const forwarded = await ladder.flush()
  const late = (await coastOffice.recv())!
  const left = await ladder.buffered()
  console.log(`coast     carried ${late.text!} on a flush of ${forwarded}, ${left} waiting`)

  // Orders from shore come back over whichever listening link is in reach.
  await coastOffice.send(ORDERS, 'return to port')
  const order = await ladder.recv()
  console.log(`vessel    took ${order.text!} over the coast network`)

  // A ladder does one thing at a time, so a vessel that listens and reports waits for
  // orders with a limit and reports between waits.
  try {
    await ladder.recv(QUIET_MS)
  } catch {
    console.log(`vessel    heard nothing more from shore within ${QUIET_MS} ms`)
  }
  await ladder.send(REPORT, 'report 5')
  const last = (await coastOffice.recv())!
  console.log(`coast     carried ${last.text!} between waits`)

  return { first, stormy, counts: [waiting, still, left], order: order.text, last: last.text }
}

main()
```
<!-- end -->

## Python

In Python, `Ladder(store)` from `pamoja.ladder` takes a store from `pamoja.sync`,
`Store.memory(capacity)` or `Store.file(dir)`, and consumes it. `await ladder.rung(transport)`
and `await ladder.uplink(transport)` add links. `send(topic, bytes_or_text)` returns
`Delivery.SENT` or `Delivery.BUFFERED`, `flush()` how many records went out, and `buffered()` how
many wait. `subscribe(filter)` places a filter on every listening rung, and `recv()` returns the
next message from any of them. `asyncio.wait_for(ladder.recv(), seconds)` stops waiting without
losing the next message. Calls on one ladder run one at a time, and a failure raises
`PamojaError`.

<!-- snippet: bindings/python/guides/ladder.py#example -->
From [`bindings/python/guides/ladder.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ladder.py):

```python
import asyncio

from pamoja.ladder import Delivery, Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store

REPORT = "vessel/7/report"
ORDERS = "vessel/7/orders"
QUIET_MS = 50


async def main() -> None:
    # Three networks a vessel can reach: the harbor's wifi, the coast's cellular
    # network, and a satellite. Each is a broker with an office ashore listening on it,
    # so which one carried a report is read off that office rather than assumed.
    harbor = LoopbackBroker()
    coast = LoopbackBroker()
    sky = LoopbackBroker()
    harbor_office = harbor.link()
    coast_office = coast.link()
    sky_office = sky.link()
    for office in (harbor_office, coast_office, sky_office):
        await office.connect()
        await office.subscribe(REPORT)

    # Rungs go on cheapest first. The satellite only sends, so it goes on as an uplink,
    # which the ladder never subscribes or listens on.
    ladder = Ladder(Store.memory())
    await ladder.rung(harbor.rung())
    await ladder.rung(coast.rung())
    await ladder.uplink(sky.rung())
    await ladder.connect()
    await ladder.subscribe(ORDERS)

    # In the harbor, the cheapest link takes the report.
    first = await ladder.send(REPORT, "report 1")
    print(f"harbor    carried {(await harbor_office.recv()).text}")

    # Past the breakwater the wifi is out of reach and the report falls through to the
    # coast, and further out to the satellite.
    harbor.reachable = False
    await ladder.send(REPORT, "report 2")
    print(f"coast     carried {(await coast_office.recv()).text}, with the harbor out of reach")
    coast.reachable = False
    await ladder.send(REPORT, "report 3")
    print(f"sky       carried {(await sky_office.recv()).text}, with the coast out of reach too")

    # In a storm nothing is in reach, and the report waits in the store rather than
    # being lost.
    sky.reachable = False
    stormy = await ladder.send(REPORT, "report 4")
    waiting = await ladder.buffered()
    print(f"vessel    buffered report 4 with every link out of reach, {waiting} waiting")

    # A flush with every link still out of reach forwards nothing, because a record
    # leaves the store only once a link has taken it.
    idle = await ladder.flush()
    still = await ladder.buffered()
    print(f"vessel    flushed {idle} while every link was out of reach, {still} still waiting")

    # Back in reach of the coast, a flush sends the backlog, oldest first.
    coast.reachable = True
    forwarded = await ladder.flush()
    late = await coast_office.recv()
    left = await ladder.buffered()
    print(f"coast     carried {late.text} on a flush of {forwarded}, {left} waiting")

    # Orders from shore come back over whichever listening link is in reach.
    await coast_office.send(ORDERS, "return to port")
    order = await ladder.recv()
    print(f"vessel    took {order.text} over the coast network")

    # A ladder does one thing at a time, so a vessel that listens and reports waits
    # for orders with a limit and reports between waits.
    try:
        await asyncio.wait_for(ladder.recv(), QUIET_MS / 1000)
    except asyncio.TimeoutError:
        print(f"vessel    heard nothing more from shore within {QUIET_MS} ms")
    await ladder.send(REPORT, "report 5")
    last = await coast_office.recv()
    print(f"coast     carried {last.text} between waits")

    return first, stormy, (waiting, still, left), order.text, last.text


first, stormy, counts, order, last = asyncio.run(main())
```
<!-- end -->

## C#

In C#, `new Ladder(store)` in `Pamoja.Ladder` takes a `Store` from `Pamoja.Sync`,
`Store.Memory(capacity)` or `Store.File(dir)`, and is disposable. `Rung(transport)` and
`Uplink(transport)` add links and consume them. `SendAsync(topic, textOrBytes)` returns
`Delivery.Sent` or `Delivery.Buffered`, `FlushAsync()` how many records went out, and
`BufferedAsync()` how many wait. `SubscribeAsync(filter)` places a filter on every listening
rung, and `ReceiveAsync()` and `ReceiveAsync(limit)` wait for the next message, the second
throwing `TimeoutException` when the time runs out. Calls on one ladder run one at a time, and a
call waiting for its turn holds no thread. A failure throws `PamojaException`.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LadderGuide.cs#example -->
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
<!-- end -->

## Values at a glance

**The calls in each language:**

| What | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| a ladder | `TransportLadder::new(store)` | `new Ladder(store)` | `Ladder(store)` | `new Ladder(store)` |
| a rung that listens | `.rung(link)` | `await ladder.rung(link)` | `await ladder.rung(link)` | `ladder.Rung(link)` |
| a rung that only sends | `.uplink(link)` | `await ladder.uplink(link)` | `await ladder.uplink(link)` | `ladder.Uplink(link)` |
| connect | `connect().await?` | `await ladder.connect()` | `await ladder.connect()` | `await ladder.ConnectAsync()` |
| send | `send(topic, &bytes)`, `send_text(topic, text)` | `await ladder.send(topic, bufferOrText)` | `await ladder.send(topic, bytes_or_text)` | `await ladder.SendAsync(topic, textOrBytes)` |
| drain the backlog | `flush().await?` | `await ladder.flush()` | `await ladder.flush()` | `await ladder.FlushAsync()` |
| records waiting | `buffered().await?` | `await ladder.buffered()` | `await ladder.buffered()` | `await ladder.BufferedAsync()` |
| subscribe | `subscribe(filter).await?` | `await ladder.subscribe(filter)` | `await ladder.subscribe(filter)` | `await ladder.SubscribeAsync(filter)` |
| receive | `recv().await?` | `await ladder.recv(ms?)` | `await ladder.recv()` | `await ladder.ReceiveAsync(limit?)` |

**What a send does:**

| When | The ladder | Returns |
| --- | --- | --- |
| the store is empty and a rung takes the message | delivers it over the first rung that does | `Sent` |
| the store is empty and every rung refuses | appends it to the store | `Buffered` |
| the store holds a backlog | appends it behind the backlog, so it never overtakes, without trying a rung | `Buffered` |
| the store is full | refuses the send with the store's error | an error |

A flush forwards the store's records oldest first. Each leaves the store only once a rung has
taken it, and the flush stops at the first record no rung takes, so what arrives upstream is in
the order it was measured and nothing is sent twice.

**Where a ladder's messages wait:**

| Store | Rust | TypeScript | Python | C# | Lasts |
| --- | --- | --- | --- | --- | --- |
| memory | `MemoryStore::new()` | `Store.memory()` | `Store.memory()` | `Store.Memory()` | as long as the process |
| memory, bounded | `MemoryStore::with_capacity(n)` | `Store.memory(n)` | `Store.memory(n)` | `Store.Memory(n)` | as long as the process |
| file | `FileStore::open(dir)` | `Store.file(dir)` | `Store.file(dir)` | `Store.File(dir)` | across a restart or a power cut |

**Taking a loopback network out of reach,** for testing what a node does when its links go:

| Rust | TypeScript | Python | C# |
| --- | --- | --- | --- |
| `broker.set_reachable(false)` | `broker.reachable = false` | `broker.reachable = False` | `broker.Reachable = false` |

Every link on the broker then fails to connect, send, or subscribe with `transport error: the
broker is out of reach`, including links a ladder owns, and keeps its connection and filters for
when the broker is back. [Your own link](link.md) sets out what a ladder does with each thing a
link can do.

## When it goes wrong

What a ladder says:

| What happened | The message | What to check |
| --- | --- | --- |
| a send with the store full | `io error: store is at capacity` | flush, raise the capacity, or decide what to drop |
| a receive with no rung listening | `resource is closed` | a rung was added with `rung`, the ladder is connected, and a link has not ended |
| no message within the limit | `no message arrived within 50 ms` in TypeScript and C#, `asyncio.TimeoutError` in Python, `Elapsed` in Rust | nothing was sent to the node, which may be what the wait was checking |
| a link that was already on a ladder | `this transport was already added to a ladder or a wrapper` | build another |
| a loopback broker taken out of reach | `transport error: the broker is out of reach` | the test that took it out |

The mistakes that cost an afternoon:

- **Readings pile up in the store and never leave.** Once a backlog exists, every send joins
  it, and only a flush drains it. Flush on a timer, and after every connect.
- **A link that dropped never comes back.** A rung whose link ended, or answered `Closed`, is
  set aside until the next `connect`. A node calls `connect` again from time to time, cheaply,
  since it touches only the rungs that are down.
- **Reports stop while the node waits for commands.** A ladder does one thing at a time, so a
  send waits behind a receive in progress. Wait with a limit and report between waits, as the
  example does.
- **A command never arrives over the satellite.** An uplink is never subscribed or listened on;
  commands arrive only over rungs added with `rung`.
- **The backlog is gone after a reboot.** A memory store lasts as long as the process. A node
  that must keep its backlog through a power cut buffers into a file store.
- **Expensive links carry everything.** Rungs are tried in the order they were added. Add the
  cheapest first, and the costliest last.
- **The store fills during a long outage.** A bounded store refuses the send that would
  overflow it, and the send fails. Size it for the longest outage the node must ride out, or
  catch the failure and decide what to keep.

## Where next

<!-- table: next ladder -->
- [MQTT](mqtt.md): An MQTT client with the topic and wildcard rules, acknowledged delivery, retained messages, a last will, and TLS, as the core transport.
- [LoRa airtime and range](lora.md): Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to, and the link budget that sets its range.
- [Store and forward](sync.md): Offline-first queues in memory or on disk, bounded or not, the on-disk one surviving power loss, and the drain that forwards them in order when a link returns.
- Also in Transports and testing: [CoAP](coap.md), [Loopback](loopback.md), [Event bus](bus.md), [Engine surface](transport.md), [Your own link](link.md), [Simulators](sim.md).
<!-- end -->

## Reference

<!-- table: reference ladder -->
- Rust: [`pamoja-ladder`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-ladder)
- TypeScript: [`@pamoja/ladder`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-ladder)
- Python: [`pamoja.ladder`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-ladder)
- C#: [`Pamoja.Ladder`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-ladder)
<!-- end -->
