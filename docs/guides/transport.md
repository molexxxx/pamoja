# Engine surface

A node should not care which link it got. The reading it took is worth the same
whether it leaves over MQTT, over CoAP, over a mesh hop, or into a queue on disk
until morning. So every link in pamoja implements one contract: connect,
subscribe, send, and, for a link that can deliver, receive. Everything that
carries traffic takes that contract rather than a particular link, which is what
lets a store, a ladder, or a fault injector work with all of them and with each
other. A ladder is a link under that contract too, so a node written against one
transport runs over a ladder unchanged.

The fault injector is the clearest case. It is a transport that wraps a transport
and fails a set number of the sends passing through it, so the offline path can be
exercised without unplugging anything. It composes exactly where a real link does,
because as far as the ladder above it is concerned it is a real link.

In Rust the contract is a trait and composition is a move, so the compiler already
stops a link being used after it has been handed on. The three bindings have no way
to say "any transport", so each holds one handle that dispatches to whichever kind
was built, and composing empties the handle rather than leaving it aliasing what now
belongs to something else. A link handed to a fault injector or to a ladder is
spent, and calling anything on it afterwards throws.

## What the example does

It runs a link that fails its first send, underneath a ladder with a queue, and
follows two readings through: the one the failure catches, and the one taken
after it.

The gateway and the ladder are two links onto the same in-process broker, so what
the subscriber reads is what actually crossed the link rather than something
handed to it. The failure is arranged by telling the injector how many sends to
refuse, and the depth of the backlog is the ladder's own `buffered` count, not a
tally the example keeps.

It proves:

- A fault injector sits in the ladder where a plain link would, and the send it
  refuses comes back as `Buffered` rather than an error, so the reading is held
  instead of lost.
- The reading taken next is buffered too, even though the link would carry it
  now, and the ladder counts both as queued.
- A flush forwards the whole backlog and leaves nothing queued behind it.
- The subscriber reads `20.1` and then `20.4`, so the far end sees the readings
  in the order they were taken, not the order the link became willing to carry
  them.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example transport" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example transport</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- transport" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- transport</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/transport.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/transport.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- transport" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- transport</code></div>
</div>
<!-- end -->

## Rust

<!-- snippet: examples/guides/transport.rs#example -->
From [`examples/guides/transport.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/transport.rs):

```rust
use pamoja_core::{Receive, Transport};
use pamoja_ladder::{Delivery, TransportLadder};
use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
use pamoja_sync::MemoryStore;

// Whatever a link is underneath, MQTT, CoAP, or the in-process broker here, it reaches
// the rest of the framework through one trait. Anything that takes a link is generic
// over it, so a node is written once and pointed at whichever link it has.
let broker = LoopbackBroker::new();
let topic = "sensors/1/temperature";
let mut gateway = LoopbackTransport::new(broker.clone());
gateway.connect().await?;
gateway.subscribe(topic).await?;

// The fault injector is itself a transport wrapping a transport, so it composes
// anywhere a link does. This one fails its next send and passes the rest through.
let mut ladder = TransportLadder::new(MemoryStore::new())
    .rung(Faulty::new(LoopbackTransport::new(broker), 1));
ladder.connect().await?;

// The injected failure lands, so the reading is buffered rather than lost.
let first = ladder.send_text(topic, "20.1").await?;
let after_first = ladder.buffered().await.expect("a count");
println!("first reading: {first:?}, {after_first} queued");

// The next reading joins the back of the queue instead of overtaking it, even though
// the link would take it now. Order on the wire is the order the readings were taken.
let second = ladder.send_text(topic, "20.4").await?;
let queued = ladder.buffered().await.expect("a count");
println!("second reading: {second:?}, {queued} queued");

// Flushing forwards the backlog oldest first, and the subscriber sees it in order.
let forwarded = ladder.flush().await?;
let first_out = gateway.recv().await?.expect("a message");
let second_out = gateway.recv().await?.expect("a message");
let earlier = first_out.text().expect("text");
let later = second_out.text().expect("text");
println!("flush forwarded {forwarded}, gateway saw {earlier} then {later}");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/transport.ts#example -->
From [`bindings/node/guides/transport.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/transport.ts):

```typescript
import { Transport } from '@pamoja/core'
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

const TOPIC = 'sensors/1/temperature'

async function main() {
  // Whatever a link is underneath, MQTT, CoAP, or the in-process broker here, it reaches
  // the rest of the framework through one contract. Anything that takes a link works with
  // any of them, so a node is written once and pointed at whichever link it has.
  const broker = new LoopbackBroker()
  const gateway = broker.link()
  await gateway.connect()
  await gateway.subscribe(TOPIC)

  // The fault injector is itself a link wrapping a link, so it composes anywhere one does.
  // This one fails its next send and passes the rest through.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.faulty(broker.rung(), 1))
  await ladder.connect()

  // The injected failure lands, so the reading is buffered rather than lost.
  const first = await ladder.send(TOPIC, '20.1')
  console.log(`first reading: ${first}, ${await ladder.buffered()} queued`)

  // The next reading joins the back of the queue instead of overtaking it, even though the
  // link would take it now. Order on the wire is the order the readings were taken.
  const second = await ladder.send(TOPIC, '20.4')
  const queued = await ladder.buffered()
  console.log(`second reading: ${second}, ${queued} queued`)

  // Flushing forwards the backlog oldest first, and the subscriber sees it in order.
  const forwarded = await ladder.flush()
  const earlier = (await gateway.recv())!.text!
  const later = (await gateway.recv())!.text!
  console.log(`flush forwarded ${forwarded}, gateway saw ${earlier} then ${later}`)

  return { first, second, queued, forwarded, left: await ladder.buffered(), earlier, later }
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

TOPIC = "sensors/1/temperature"


async def main() -> None:
    # Whatever a link is underneath, MQTT, CoAP, or the in-process broker here, it reaches
    # the rest of the framework through one contract. Anything that takes a link works with
    # any of them, so a node is written once and pointed at whichever link it has.
    broker = LoopbackBroker()
    gateway = broker.link()
    await gateway.connect()
    await gateway.subscribe(TOPIC)

    # The fault injector is itself a link wrapping a link, so it composes anywhere one
    # does. This one fails its next send and passes the rest through.
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.faulty(broker.rung(), 1))
    await ladder.connect()

    # The injected failure lands, so the reading is buffered rather than lost.
    first = await ladder.send(TOPIC, "20.1")
    print(f"first reading: {first}, {await ladder.buffered()} queued")

    # The next reading joins the back of the queue instead of overtaking it, even though
    # the link would take it now. Order on the wire is the order the readings were taken.
    second = await ladder.send(TOPIC, "20.4")
    queued = await ladder.buffered()
    print(f"second reading: {second}, {queued} queued")

    # Flushing forwards the backlog oldest first, and the subscriber sees it in order.
    forwarded = await ladder.flush()
    earlier = (await gateway.recv()).text
    later = (await gateway.recv()).text
    print(f"flush forwarded {forwarded}, gateway saw {earlier} then {later}")

    return first, second, queued, forwarded, await ladder.buffered(), earlier, later


first, second, queued, forwarded, left, earlier, later = asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/TransportGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/TransportGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/TransportGuide.cs):

```csharp
const string Topic = "sensors/1/temperature";

// Whatever a link is underneath, MQTT, CoAP, or the in-process broker here, it
// reaches the rest of the framework through one contract. Anything that takes a
// link works with any of them, so a node is written once and pointed at whichever
// link it has.
using var broker = new LoopbackBroker();
using var gateway = broker.Link();
await gateway.ConnectAsync();
await gateway.SubscribeAsync(Topic);

// The fault injector is itself a link wrapping a link, so it composes anywhere one
// does. This one fails its next send and passes the rest through.
using var ladder = new Ladder(Store.Memory());
ladder.Rung(Transport.Faulty(broker.Rung(), 1));
await ladder.ConnectAsync();

// The injected failure lands, so the reading is buffered rather than lost.
Delivery first = await ladder.SendAsync(Topic, "20.1");
Console.WriteLine($"first reading: {first}, {await ladder.BufferedAsync()} queued");

// The next reading joins the back of the queue instead of overtaking it, even
// though the link would take it now. Order on the wire is the order they were
// taken.
Delivery second = await ladder.SendAsync(Topic, "20.4");
int queued = await ladder.BufferedAsync();
Console.WriteLine($"second reading: {second}, {queued} queued");

// Flushing forwards the backlog oldest first, and the subscriber sees it in order.
int forwarded = await ladder.FlushAsync();
TransportMessage earlier = (await gateway.ReceiveAsync())!;
TransportMessage later = (await gateway.ReceiveAsync())!;
Console.WriteLine(
    $"flush forwarded {forwarded}, gateway saw"
    + $" {earlier.Text} then"
    + $" {later.Text}");
```
<!-- end -->

## Reference

<!-- table: reference transport -->
- Rust: the `Transport` and `Receive` traits in [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-transport)
- TypeScript: [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-transport)
- Python: [`pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-transport)
- C#: [`Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-transport)
<!-- end -->
