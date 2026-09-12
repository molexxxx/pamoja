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
reaches the node over whatever link happens to be up. In Rust, a send-only link such
as a LoRa uplink is added with `uplink` rather than `rung`, and is never listened on.

## What the example does

It builds a ladder over two in-process links, a near mesh hop and a metered
backhaul, and follows two readings and two flushes through it. Each link has its
own loopback broker, and the gateway subscribes to the backhaul's, so which rung
carried a reading is read off a subscriber rather than taken on trust.

Neither failure is hand-written into a transport. Both rungs are ordinary
loopback links wrapped in the simulator's degraded link, so what the ladder sees
is the transport error an out-of-range radio raises. The mesh hop drops every
send. The backhaul carries one send, refuses the next two, then is reachable
again, which lines up with the four attempts the ladder makes on it: the two
readings and the two flushes.

Last, a command goes the other way. The ladder is subscribed to the valve topic, the
gateway publishes `open` on the backhaul, and the ladder hands it up, so the object
that carried the readings out is the one that carries the command in.

It proves:

- Rungs are tried in the order they were added, and a refusing rung falls through
  to the next.
- The first reading arrives on the backhaul's subscriber carrying `21.5`, so
  which link was used is observable rather than assumed.
- With every rung down, a send is buffered rather than lost, and the ladder
  reports the one record it is holding.
- A flush while both links are down forwards nothing and leaves that record
  waiting in the queue.
- The next flush forwards one, the gateway receives `21.6`, and the queue drops
  to zero, so the backlog went out exactly once.
- A subscription placed on the ladder reaches its rungs, and the `open` published
  on the backhaul arrives through the ladder's own receive.

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

<!-- snippet: examples/guides/ladder.rs#example -->
From [`examples/guides/ladder.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/ladder.rs):

```rust
use pamoja_core::{Receive, Transport};
use pamoja_ladder::{Delivery, TransportLadder};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_sim::DegradedLink;
use pamoja_sync::MemoryStore;

// Two links off the same node: a near mesh hop and a metered backhaul. Each has its
// own broker, so which rung carried a reading is visible from its subscriber.
let mesh = LoopbackBroker::new();
let backhaul = LoopbackBroker::new();
let topic = "sensors/1/temperature";
let mut gateway = LoopbackTransport::new(backhaul.clone());
gateway.connect().await?;
gateway.subscribe(topic).await?;

// Rungs are tried in the order they are added, cheapest first. The mesh hop loses
// every packet here; the backhaul carries one send, then drops the next two.
let mut ladder = TransportLadder::new(MemoryStore::new())
    .rung(DegradedLink::new(LoopbackTransport::new(mesh)).drop_every(1))
    .rung(DegradedLink::new(LoopbackTransport::new(backhaul)).intermittent(1, 2));
ladder.connect().await?;

// The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
// broker only that rung publishes to.
let first = ladder.send_text(topic, "21.5").await?;
let arrived = gateway.recv().await?.expect("a message");
let reading = arrived.text().expect("text");
println!("first reading: {first:?}, gateway got {reading}");

// Now nothing will take a send, so the next reading is buffered rather than lost.
let second = ladder.send_text(topic, "21.6").await?;
let waiting = ladder.buffered().await.expect("a count");
println!("second reading: {second:?}, {waiting} waiting in the queue");

// A flush while the links are still down forwards nothing and leaves the backlog
// intact, because a record is removed only once a rung has accepted it.
let while_down = ladder.flush().await?;
let still_queued = ladder.buffered().await.expect("a count");
println!("flush while down forwarded {while_down}, queue still {still_queued}");

// The backhaul is reachable again, so the buffered reading goes out exactly once.
let when_up = ladder.flush().await?;
let late = gateway.recv().await?.expect("a message");
let buffered_reading = late.text().expect("text");
println!("flush when up forwarded {when_up}, gateway got {buffered_reading}");

// The ladder is a link both ways. A subscription placed on it goes onto every rung
// that listens, and a receive takes whichever rung delivers, so a command reaches
// the node over whatever link is up. This one comes back over the backhaul.
ladder.subscribe("actuators/1/valve").await?;
gateway.send_text("actuators/1/valve", "open").await?;
let command = ladder.recv().await?.expect("a command");
let order = command.text().expect("text");
println!("command back over the ladder: {order}");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/ladder.ts#example -->
From [`bindings/node/guides/ladder.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ladder.ts):

```typescript
import { Transport } from '@pamoja/core'
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

const TOPIC = 'sensors/1/temperature'

async function main() {
  // Two links off the same node: a near mesh hop and a metered backhaul. Each is a
  // separate broker, so which one carried a reading is visible from its subscriber.
  const mesh = new LoopbackBroker()
  const backhaul = new LoopbackBroker()
  const gateway = backhaul.link()
  await gateway.connect()
  await gateway.subscribe(TOPIC)

  // Rungs are tried in the order they are added, cheapest first. The mesh hop loses every
  // packet here; the backhaul carries one send, then drops the next two.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.degraded(mesh.rung(), { dropEvery: 1 }))
  await ladder.rung(Transport.degraded(backhaul.rung(), { up: 1, down: 2 }))
  await ladder.connect()

  // The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
  // broker only that rung publishes to.
  const first = await ladder.send(TOPIC, '21.5')
  const arrived = (await gateway.recv())!
  console.log(`first reading: ${first}, gateway got ${arrived.text!}`)

  // Now nothing will take a send, so the next reading is buffered rather than lost.
  const second = await ladder.send(TOPIC, '21.6')
  const waiting = await ladder.buffered()
  console.log(`second reading: ${second}, ${waiting} waiting in the queue`)

  // A flush while the links are still down forwards nothing and leaves the backlog
  // intact, because a record is removed only once a rung has accepted it.
  const whileDown = await ladder.flush()
  console.log(`flush while down forwarded ${whileDown}, queue still ${await ladder.buffered()}`)

  // The backhaul is reachable again, so the buffered reading goes out exactly once.
  const whenUp = await ladder.flush()
  const late = (await gateway.recv())!
  console.log(`flush when up forwarded ${whenUp}, gateway got ${late.text!}`)

  // The ladder is a link both ways. A subscription placed on it goes onto every rung that
  // listens, and a receive takes whichever rung delivers, so a command reaches the node
  // over whatever link is up. This one comes back over the backhaul.
  await ladder.subscribe('actuators/1/valve')
  await gateway.send('actuators/1/valve', 'open')
  const command = (await ladder.recv())!
  console.log(`command back over the ladder: ${command.text!}`)

  const left = await ladder.buffered()
  return { first, second, waiting, whileDown, whenUp, left, late, command }
}

main()
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/ladder.py#example -->
From [`bindings/python/guides/ladder.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ladder.py):

```python
import asyncio

from pamoja.core import Transport
from pamoja.ladder import Delivery, Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store

TOPIC = "sensors/1/temperature"


async def main() -> None:
    # Two links off the same node: a near mesh hop and a metered backhaul. Each has its
    # own broker, so which rung carried a reading is visible from its subscriber.
    mesh = LoopbackBroker()
    backhaul = LoopbackBroker()
    gateway = backhaul.link()
    await gateway.connect()
    await gateway.subscribe(TOPIC)

    # Rungs are tried in the order they are added, cheapest first. The mesh hop loses
    # every packet here; the backhaul carries one send, then drops the next two.
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.degraded(mesh.rung(), drop_every=1))
    await ladder.rung(Transport.degraded(backhaul.rung(), up=1, down=2))
    await ladder.connect()

    # The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
    # broker only that rung publishes to.
    first = await ladder.send(TOPIC, "21.5")
    arrived = await gateway.recv()
    print(f"first reading: {first}, gateway got {arrived.text}")

    # Now nothing will take a send, so the next reading is buffered rather than lost.
    second = await ladder.send(TOPIC, "21.6")
    waiting = await ladder.buffered()
    print(f"second reading: {second}, {waiting} waiting in the queue")

    # A flush while the links are still down forwards nothing and leaves the backlog
    # intact, because a record is removed only once a rung has accepted it.
    while_down = await ladder.flush()
    print(f"flush while down forwarded {while_down}, queue still {await ladder.buffered()}")

    # The backhaul is reachable again, so the buffered reading goes out exactly once.
    when_up = await ladder.flush()
    late = await gateway.recv()
    print(f"flush when up forwarded {when_up}, gateway got {late.text}")

    # The ladder is a link both ways. A subscription placed on it goes onto every rung
    # that listens, and a receive takes whichever rung delivers, so a command reaches
    # the node over whatever link is up. This one comes back over the backhaul.
    await ladder.subscribe("actuators/1/valve")
    await gateway.send("actuators/1/valve", "open")
    command = await ladder.recv()
    print(f"command back over the ladder: {command.text}")

    left = await ladder.buffered()
    return first, second, waiting, while_down, when_up, left, late, command


first, second, waiting, while_down, when_up, left, late, command = asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LadderGuide.cs#example -->
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
Delivery first = await ladder.SendAsync(Topic, "21.5");
TransportMessage arrived = (await gateway.ReceiveAsync())!;
Console.WriteLine(
    $"first reading: {first}, gateway got"
    + $" {arrived.Text}");

// Now nothing will take a send, so the next reading is buffered rather than lost.
Delivery second = await ladder.SendAsync(Topic, "21.6");
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
    + $" {late.Text}");

// The ladder is a link both ways. A subscription placed on it goes onto every
// rung that listens, and a receive takes whichever rung delivers, so a command
// reaches the node over whatever link is up. This one comes back over the
// backhaul.
await ladder.SubscribeAsync("actuators/1/valve");
await gateway.SendAsync("actuators/1/valve", "open");
TransportMessage command = (await ladder.ReceiveAsync())!;
Console.WriteLine(
    $"command back over the ladder: {command.Text}");
```
<!-- end -->

## Reference

<!-- table: reference ladder -->
- Rust: [`pamoja-ladder`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-ladder)
- TypeScript: [`@pamoja/ladder`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-ladder)
- Python: [`pamoja.ladder`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-ladder)
- C#: [`Pamoja.Ladder`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-ladder)
<!-- end -->
