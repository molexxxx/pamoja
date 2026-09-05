# Transport ladder

Most nodes that matter have more than one way to reach home, and the ways are not
equally priced. A mesh hop to the neighbour costs almost nothing. A cellular
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

## Rust

<!-- snippet: examples/tests/guides/ladder.rs#example -->
From [`examples/tests/guides/ladder.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/ladder.rs):

```rust
use pamoja_core::Transport;
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
gateway.connect().await.expect("the gateway connects");
gateway.subscribe(topic).await.expect("subscribe");

// Rungs are tried in the order they are added, cheapest first. The mesh hop loses
// every packet here; the backhaul carries one send, then drops the next two.
let mut ladder = TransportLadder::new(MemoryStore::new())
    .rung(DegradedLink::new(LoopbackTransport::new(mesh)).drop_every(1))
    .rung(DegradedLink::new(LoopbackTransport::new(backhaul)).intermittent(1, 2));
ladder.connect().await.expect("the ladder connects");

// The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
// broker only that rung publishes to.
let first = ladder.send(topic, b"21.5").await.expect("a delivery");
let arrived = gateway.recv().await.expect("recv").expect("a message");
let reading = String::from_utf8_lossy(&arrived.payload);
println!("first reading: {first:?}, gateway got {reading}");

// Now nothing will take a send, so the next reading is buffered rather than lost.
let second = ladder.send(topic, b"21.6").await.expect("a delivery");
let waiting = ladder.buffered().await.expect("a count");
println!("second reading: {second:?}, {waiting} waiting in the queue");

// A flush while the links are still down forwards nothing and leaves the backlog
// intact, because a record is removed only once a rung has accepted it.
let while_down = ladder.flush().await.expect("a flush");
let still_queued = ladder.buffered().await.expect("a count");
println!("flush while down forwarded {while_down}, queue still {still_queued}");

// The backhaul is reachable again, so the buffered reading goes out exactly once.
let when_up = ladder.flush().await.expect("a flush");
let late = gateway.recv().await.expect("recv").expect("a message");
let buffered_reading = String::from_utf8_lossy(&late.payload);
println!("flush when up forwarded {when_up}, gateway got {buffered_reading}");
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
  const first = await ladder.send(TOPIC, Buffer.from('21.5'))
  const arrived = (await gateway.recv())!
  console.log(`first reading: ${first}, gateway got ${arrived.payload.toString()}`)

  // Now nothing will take a send, so the next reading is buffered rather than lost.
  const second = await ladder.send(TOPIC, Buffer.from('21.6'))
  const waiting = await ladder.buffered()
  console.log(`second reading: ${second}, ${waiting} waiting in the queue`)

  // A flush while the links are still down forwards nothing and leaves the backlog
  // intact, because a record is removed only once a rung has accepted it.
  const whileDown = await ladder.flush()
  console.log(`flush while down forwarded ${whileDown}, queue still ${await ladder.buffered()}`)

  // The backhaul is reachable again, so the buffered reading goes out exactly once.
  const whenUp = await ladder.flush()
  const late = (await gateway.recv())!
  console.log(`flush when up forwarded ${whenUp}, gateway got ${late.payload.toString()}`)

  return { first, second, waiting, whileDown, whenUp, left: await ladder.buffered(), late }
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
    first = await ladder.send(TOPIC, b"21.5")
    arrived = await gateway.recv()
    print(f"first reading: {first}, gateway got {arrived.payload.decode()}")

    # Now nothing will take a send, so the next reading is buffered rather than lost.
    second = await ladder.send(TOPIC, b"21.6")
    waiting = await ladder.buffered()
    print(f"second reading: {second}, {waiting} waiting in the queue")

    # A flush while the links are still down forwards nothing and leaves the backlog
    # intact, because a record is removed only once a rung has accepted it.
    while_down = await ladder.flush()
    print(f"flush while down forwarded {while_down}, queue still {await ladder.buffered()}")

    # The backhaul is reachable again, so the buffered reading goes out exactly once.
    when_up = await ladder.flush()
    late = await gateway.recv()
    print(f"flush when up forwarded {when_up}, gateway got {late.payload.decode()}")

    return first, second, waiting, while_down, when_up, await ladder.buffered(), late


first, second, waiting, while_down, when_up, left, late = asyncio.run(main())
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
<!-- end -->

## Reference

<!-- table: reference ladder -->
- Rust: [`pamoja-ladder`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html)
- TypeScript: [`@pamoja/ladder`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html)
- Python: [`pamoja.ladder`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html)
- C#: [`Pamoja.Ladder`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html)
<!-- end -->
