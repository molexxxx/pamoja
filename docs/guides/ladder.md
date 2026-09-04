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

It builds a ladder over two in-process links, one that drops everything and one
that carries a single message and then goes down, and follows two readings and
two flushes through it.

It proves:

- Rungs are tried in the order they were added, and a refusing rung falls through
  to the next.
- The reading arrives on the broker belonging to the rung that carried it, so
  which link was used is observable rather than assumed.
- With every rung down, a send is buffered rather than lost, and the ladder
  reports how much it is holding.
- A flush while the links are down forwards nothing and leaves the backlog intact.
- Once a rung is reachable the backlog goes out exactly once.

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
let mut gateway = LoopbackTransport::new(backhaul.clone());
gateway.connect().await.unwrap();
gateway.subscribe("sensors/1/temperature").await.unwrap();

// Rungs are tried in the order they are added, cheapest first. The mesh hop loses
// every packet here; the backhaul carries one send, then drops the next two.
let mut ladder = TransportLadder::new(MemoryStore::new())
    .rung(DegradedLink::new(LoopbackTransport::new(mesh)).drop_every(1))
    .rung(DegradedLink::new(LoopbackTransport::new(backhaul)).intermittent(1, 2));
ladder.connect().await.unwrap();

// The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
// broker only that rung publishes to.
let topic = "sensors/1/temperature";
assert_eq!(ladder.send(topic, b"21.5").await.unwrap(), Delivery::Sent);
assert_eq!(gateway.recv().await.unwrap().unwrap().payload, b"21.5");

// Now nothing will take a send, so the next reading is buffered rather than lost.
let delivery = ladder.send(topic, b"21.6").await.unwrap();
assert_eq!(delivery, Delivery::Buffered);
assert_eq!(ladder.buffered().await.unwrap(), 1);

// A flush while the links are still down forwards nothing and leaves the backlog
// intact, because a record is removed only once a rung has accepted it.
assert_eq!(ladder.flush().await.unwrap(), 0);
assert_eq!(ladder.buffered().await.unwrap(), 1);

// The backhaul is reachable again, so the buffered reading goes out exactly once.
assert_eq!(ladder.flush().await.unwrap(), 1);
assert_eq!(ladder.buffered().await.unwrap(), 0);
assert_eq!(gateway.recv().await.unwrap().unwrap().payload, b"21.6");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/ladder.ts#example -->
From [`bindings/node/guides/ladder.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ladder.ts):

```typescript
import assert from 'node:assert/strict'

import { Transport } from '@pamoja/core'
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

async function main() {
  // Two links off the same node: a near mesh hop and a metered backhaul. Each is a
  // separate broker, so which one carried a reading is visible from its subscriber.
  const mesh = new LoopbackBroker()
  const backhaul = new LoopbackBroker()
  const gateway = backhaul.link()
  await gateway.connect()
  await gateway.subscribe('sensors/1/temperature')

  // Rungs are tried in the order they are added, cheapest first. The mesh hop loses every
  // packet here; the backhaul carries one send, then drops the next two.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.degraded(mesh.rung(), 1, 0, 0))
  await ladder.rung(Transport.degraded(backhaul.rung(), 0, 1, 2))
  await ladder.connect()

  // The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
  // broker only that rung publishes to.
  const topic = 'sensors/1/temperature'
  assert.equal(await ladder.send(topic, Buffer.from('21.5')), Delivery.Sent)
  assert.equal((await gateway.recv())?.payload.toString(), '21.5')

  // Now nothing will take a send, so the next reading is buffered rather than lost.
  assert.equal(await ladder.send(topic, Buffer.from('21.6')), Delivery.Buffered)
  assert.equal(await ladder.buffered(), 1)

  // A flush while the links are still down forwards nothing and leaves the backlog
  // intact, because a record is removed only once a rung has accepted it.
  assert.equal(await ladder.flush(), 0)
  assert.equal(await ladder.buffered(), 1)

  // The backhaul is reachable again, so the buffered reading goes out exactly once.
  assert.equal(await ladder.flush(), 1)
  assert.equal(await ladder.buffered(), 0)
  assert.equal((await gateway.recv())?.payload.toString(), '21.6')
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


async def main() -> None:
    # Two links off the same node: a near mesh hop and a metered backhaul. Each is a
    # separate broker, so which one carried a reading is visible from its subscriber.
    mesh = LoopbackBroker()
    backhaul = LoopbackBroker()
    gateway = backhaul.link()
    await gateway.connect()
    await gateway.subscribe("sensors/1/temperature")

    # Rungs are tried in the order they are added, cheapest first. The mesh hop loses
    # every packet here; the backhaul carries one send, then drops the next two.
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.degraded(mesh.rung(), drop_every=1))
    await ladder.rung(Transport.degraded(backhaul.rung(), up=1, down=2))
    await ladder.connect()

    # The mesh hop refuses, so the reading goes out over the backhaul and arrives on
    # the broker only that rung publishes to.
    assert await ladder.send("sensors/1/temperature", b"21.5") == Delivery.SENT
    assert (await gateway.recv()).payload == b"21.5"

    # Now nothing will take a send, so the next reading is buffered rather than lost.
    assert await ladder.send("sensors/1/temperature", b"21.6") == Delivery.BUFFERED
    assert await ladder.buffered() == 1

    # A flush while the links are still down forwards nothing and leaves the backlog
    # intact, because a record is removed only once a rung has accepted it.
    assert await ladder.flush() == 0
    assert await ladder.buffered() == 1

    # The backhaul is reachable again, so the buffered reading goes out exactly once.
    assert await ladder.flush() == 1
    assert await ladder.buffered() == 0
    assert (await gateway.recv()).payload == b"21.6"


asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LadderGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/LadderGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LadderGuide.cs):

```csharp
// Two links off the same node: a near mesh hop and a metered backhaul. Each is a
// separate broker, so which one carried a reading is visible from its subscriber.
using var mesh = new LoopbackBroker();
using var backhaul = new LoopbackBroker();
using var gateway = backhaul.Link();
await gateway.ConnectAsync();
await gateway.SubscribeAsync("sensors/1/temperature");

// Rungs are tried in the order they are added, cheapest first. The mesh hop loses
// every packet here; the backhaul carries one send, then drops the next two.
using var ladder = new Ladder(Store.Memory());
ladder.Rung(Transport.Degraded(mesh.Rung(), dropEvery: 1));
ladder.Rung(Transport.Degraded(backhaul.Rung(), up: 1, down: 2));
await ladder.ConnectAsync();

// The mesh hop refuses, so the reading goes out over the backhaul and arrives on
// the broker only that rung publishes to.
const string topic = "sensors/1/temperature";
Expect(
    await ladder.SendAsync(topic, "21.5"u8.ToArray()) == Delivery.Sent,
    "a dead rung falls through to the next one");
Expect(
    (await gateway.ReceiveAsync())?.Payload.AsSpan().SequenceEqual("21.5"u8) == true,
    "and the reading arrives over the rung that took it");

// Now nothing will take a send, so the next reading is buffered rather than lost.
Expect(
    await ladder.SendAsync(topic, "21.6"u8.ToArray()) == Delivery.Buffered,
    "with every rung down the reading is buffered");
Expect(await ladder.BufferedAsync() == 1, "and the backlog holds it");

// A flush while the links are still down forwards nothing and leaves the backlog
// intact, because a record is removed only once a rung has accepted it.
Expect(await ladder.FlushAsync() == 0, "a flush with no link forwards nothing");
Expect(await ladder.BufferedAsync() == 1, "and loses nothing");

// The backhaul is reachable again, so the buffered reading goes out exactly once.
Expect(await ladder.FlushAsync() == 1, "the reading goes out once a link returns");
Expect(await ladder.BufferedAsync() == 0, "leaving nothing queued");
Expect(
    (await gateway.ReceiveAsync())?.Payload.AsSpan().SequenceEqual("21.6"u8) == true,
    "and it arrives exactly once");
```
<!-- end -->

## Reference

<!-- table: reference ladder -->
- Rust: [`pamoja-ladder`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html)
- TypeScript: [`@pamoja/ladder`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html)
- Python: [`pamoja.ladder`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html)
- C#: [`Pamoja.Ladder`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html)
<!-- end -->
