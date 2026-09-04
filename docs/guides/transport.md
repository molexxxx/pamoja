# Engine surface

A node should not care which link it got. The reading it took is worth the same
whether it leaves over MQTT, over CoAP, over a mesh hop, or into a queue on disk
until morning. So every link in pamoja implements one contract: connect, send,
subscribe, receive, disconnect. Everything that carries traffic takes that
contract rather than a particular link, which is what lets a store, a ladder, or
a fault injector work with all of them and with each other.

The fault injector is the clearest case. It is a transport that wraps a transport
and fails a set number of the sends passing through it, so the offline path can be
exercised without unplugging anything. It composes exactly where a real link does,
because as far as the ladder above it is concerned it is a real link.

In Rust the contract is a trait and composition is a move, so the compiler already
stops a link being used after it has been handed on. The three bindings have no way
to say "any transport", so each holds one handle that dispatches to whichever kind
was built, and composing empties the handle rather than leaving it aliasing what now
belongs to something else. The examples below check that, since it is the one place
the bindings have to do by hand what Rust gets from the language.

## What the example does

It runs a link that fails its first send, underneath a ladder with a queue, and
follows two readings through: the one the failure catches, and the one taken
after it.

It proves:

- A fault injector is a transport wrapping a transport, so it goes wherever a
  link goes.
- A refused send is buffered rather than lost, and the queue says how much it is
  holding.
- The reading taken next joins the back of that queue instead of overtaking it,
  even though the link would carry it now, so what reaches the subscriber is in
  the order the readings were taken.
- A flush forwards the whole backlog and empties the queue.
- In the bindings, composing a transport spends the handle.

## Rust

<!-- snippet: examples/tests/guides/transport.rs#example -->
From [`examples/tests/guides/transport.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/transport.rs):

```rust
use pamoja_core::Transport;
use pamoja_ladder::{Delivery, TransportLadder};
use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
use pamoja_sync::MemoryStore;

// Whatever a link is underneath, MQTT, CoAP, or the in-process broker here, it reaches
// the rest of the framework through one trait. Anything that takes a link is generic
// over it, so a node is written once and pointed at whichever link it has.
let broker = LoopbackBroker::new();
let mut gateway = LoopbackTransport::new(broker.clone());
gateway.connect().await.unwrap();
gateway.subscribe("sensors/1/temperature").await.unwrap();

// The fault injector is itself a transport wrapping a transport, so it composes
// anywhere a link does. This one fails its next send and passes the rest through.
let mut ladder = TransportLadder::new(MemoryStore::new())
    .rung(Faulty::new(LoopbackTransport::new(broker), 1));
ladder.connect().await.unwrap();

// The injected failure lands, so the reading is buffered rather than lost.
let topic = "sensors/1/temperature";
assert_eq!(
    ladder.send(topic, b"20.1").await.unwrap(),
    Delivery::Buffered
);
assert_eq!(ladder.buffered().await.unwrap(), 1);

// The next reading joins the back of the queue instead of overtaking it, even though
// the link would take it now. Order on the wire is the order the readings were taken.
assert_eq!(
    ladder.send(topic, b"20.4").await.unwrap(),
    Delivery::Buffered
);
assert_eq!(ladder.buffered().await.unwrap(), 2);

// Flushing forwards the backlog oldest first, and the subscriber sees it in order.
assert_eq!(ladder.flush().await.unwrap(), 2);
assert_eq!(ladder.buffered().await.unwrap(), 0);
assert_eq!(gateway.recv().await.unwrap().unwrap().payload, b"20.1");
assert_eq!(gateway.recv().await.unwrap().unwrap().payload, b"20.4");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/transport.ts#example -->
From [`bindings/node/guides/transport.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/transport.ts):

```typescript
import assert from 'node:assert/strict'

import { Transport } from '@pamoja/core'
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

async function main() {
  // Whatever a link is underneath, MQTT, CoAP, or the in-process broker below, it reaches
  // the rest of the framework as one Transport. Anything that takes a link takes that, so
  // a node is written once and pointed at whichever link it has.
  const broker = new LoopbackBroker()
  const gateway = broker.link()
  await gateway.connect()
  await gateway.subscribe('sensors/1/temperature')

  // The fault injector is a Transport wrapping a Transport, so it composes anywhere a link
  // does. This one fails its next send and passes everything after through.
  const flaky = Transport.faulty(broker.rung(), 1)
  assert.equal(flaky.isAvailable, true)

  // Composing consumes the transport, because whatever it was composed into owns it from
  // here. The handle is emptied rather than left aliasing what now belongs to something
  // else, so it cannot be sent on twice.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(flaky)
  assert.equal(flaky.isAvailable, false)
  await ladder.connect()

  // The injected failure lands, so the reading is buffered rather than lost.
  assert.equal(await ladder.send('sensors/1/temperature', Buffer.from('20.1')), Delivery.Buffered)
  assert.equal(await ladder.buffered(), 1)

  // The next reading joins the back of the queue instead of overtaking it, even though the
  // link would take it now. Order on the wire is the order the readings were taken.
  assert.equal(await ladder.send('sensors/1/temperature', Buffer.from('20.4')), Delivery.Buffered)
  assert.equal(await ladder.buffered(), 2)

  // Flushing forwards the backlog oldest first, and the subscriber sees it in order.
  assert.equal(await ladder.flush(), 2)
  assert.equal(await ladder.buffered(), 0)
  assert.equal((await gateway.recv())?.payload.toString(), '20.1')
  assert.equal((await gateway.recv())?.payload.toString(), '20.4')
}

main()
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/transport.py#example -->
From [`bindings/python/guides/transport.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/transport.py):

```python
import asyncio

from pamoja.core import Transport
from pamoja.ladder import Delivery, Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store


async def main() -> None:
    # Whatever a link is underneath, MQTT, CoAP, or the in-process broker below, it
    # reaches the rest of the framework as one Transport. Anything that takes a link
    # takes that, so a node is written once and pointed at whichever link it has.
    broker = LoopbackBroker()
    gateway = broker.link()
    await gateway.connect()
    await gateway.subscribe("sensors/1/temperature")

    # The fault injector is a Transport wrapping a Transport, so it composes anywhere a
    # link does. This one fails its next send and passes everything after through.
    flaky = Transport.faulty(broker.rung(), 1)
    assert flaky.is_available

    # Composing consumes the transport, because whatever it was composed into owns it
    # from here. The handle is emptied rather than left aliasing what now belongs to
    # something else, so it cannot be sent on twice.
    ladder = Ladder(Store.memory())
    await ladder.rung(flaky)
    assert not flaky.is_available
    await ladder.connect()

    # The injected failure lands, so the reading is buffered rather than lost.
    assert await ladder.send("sensors/1/temperature", b"20.1") == Delivery.BUFFERED
    assert await ladder.buffered() == 1

    # The next reading joins the back of the queue instead of overtaking it, even though
    # the link would take it now. Order on the wire is the order the readings were taken.
    assert await ladder.send("sensors/1/temperature", b"20.4") == Delivery.BUFFERED
    assert await ladder.buffered() == 2

    # Flushing forwards the backlog oldest first, and the subscriber sees it in order.
    assert await ladder.flush() == 2
    assert await ladder.buffered() == 0
    assert (await gateway.recv()).payload == b"20.1"
    assert (await gateway.recv()).payload == b"20.4"


asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/TransportGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/TransportGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/TransportGuide.cs):

```csharp
// Whatever a link is underneath, MQTT, CoAP, or the in-process broker below, it
// reaches the rest of the framework as one Transport. Anything that takes a link
// takes that, so a node is written once and pointed at whichever link it has.
using var broker = new LoopbackBroker();
using var gateway = broker.Link();
await gateway.ConnectAsync();
await gateway.SubscribeAsync("sensors/1/temperature");

// The fault injector is a Transport wrapping a Transport, so it composes anywhere
// a link does. This one fails its next send and passes everything after through.
var flaky = Transport.Faulty(broker.Rung(), 1);
Expect(flaky.IsAvailable, "a transport not yet composed is holdable");

// Composing consumes the transport, because whatever it was composed into owns it
// from here. The handle is emptied rather than left aliasing what now belongs to
// something else, so it cannot be sent on twice.
using var ladder = new Ladder(Store.Memory());
ladder.Rung(flaky);
Expect(!flaky.IsAvailable, "and it is spent once something else owns it");
await ladder.ConnectAsync();

// The injected failure lands, so the reading is buffered rather than lost.
Expect(
    await ladder.SendAsync("sensors/1/temperature", "20.1"u8.ToArray()) == Delivery.Buffered,
    "a refused send is buffered");
Expect(await ladder.BufferedAsync() == 1, "and the backlog holds it");

// The next reading joins the back of the queue instead of overtaking it, even
// though the link would take it now. Order on the wire is the order the readings
// were taken.
Expect(
    await ladder.SendAsync("sensors/1/temperature", "20.4"u8.ToArray()) == Delivery.Buffered,
    "the next reading joins the backlog rather than passing it");
Expect(await ladder.BufferedAsync() == 2, "so both are queued");

// Flushing forwards the backlog oldest first, and the subscriber sees it in order.
Expect(await ladder.FlushAsync() == 2, "a flush forwards the whole backlog");
Expect(await ladder.BufferedAsync() == 0, "leaving nothing queued");
Expect(
    (await gateway.ReceiveAsync())?.Payload.AsSpan().SequenceEqual("20.1"u8) == true,
    "the oldest reading arrives first");
Expect(
    (await gateway.ReceiveAsync())?.Payload.AsSpan().SequenceEqual("20.4"u8) == true,
    "then the one taken after it");
```
<!-- end -->

## Reference

<!-- table: reference transport -->
- Rust: the `Transport` trait in [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html)
- TypeScript: [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html)
- Python: [`pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html)
- C#: [`Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html)
<!-- end -->
