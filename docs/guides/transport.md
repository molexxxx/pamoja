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

In Rust the contract is two traits in `pamoja-core`. `Transport` has `connect`, `subscribe`,
`send`, and `send_text`; `Receive` adds `recv` for a link that delivers, which gives `Ok(None)`
once the link has ended. Every call is async and returns `pamoja_core::Result`, whose `Error`
names the failure. A received `Message` is a topic and a payload that reads itself with `text()`
and `number()`. A receive is cancel-safe, so `tokio::time::timeout(limit, link.recv())` gives up
without losing the message it would have taken. Anything that carries traffic is generic over the
traits, and composition is a move: `Faulty::new(link, 1)` and a ladder's `rung(link)` take the
link by value, so the compiler stops it being used after it has been handed on.

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

In TypeScript the contract is the `Transport` class in `@pamoja/core`. `Transport.mqtt(options)`
and `Transport.coap(options)` open the network links, `Transport.fromHandlers(handlers)` wraps one
written in JavaScript, and a broker's `rung()` gives the loopback. `connect`, `subscribe`, `send`,
and `recv` return promises; `recv` resolves with a `TransportMessage`, `{ topic, payload, text?,
number? }`, or `null` once the link has ended, and `recv(timeoutMs)` rejects when the time runs
out without losing the next message. Calls on one transport run one at a time.
`Transport.faulty`, `Transport.degraded`, and a ladder's `rung` take a transport and empty it:
`isAvailable` turns false, and a call on it rejects.

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
  // the rest of the framework as a Transport, driven with the same four calls: connect,
  // subscribe, send, and recv. A node is written once and pointed at whichever link it has.
  const broker = new LoopbackBroker()
  const gateway: Transport = broker.rung()
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

In Python it is the same `Transport` class, in `pamoja.core`: `Transport.mqtt(...)`,
`Transport.coap(...)`, `Transport.from_handlers(...)`, and a broker's `rung()`. `connect`,
`subscribe`, `send`, and `recv` are coroutines, and `recv` returns a `Message`, whose `payload` is
bytes and whose `text` and `number` raise `ValueError` when the payload is not one, or `None`
once the link has ended. `asyncio.wait_for(link.recv(), seconds)` gives up without losing the
next message. Composing empties the handle, `is_available` turns false, and using it raises
`PamojaError`, as every failed call does.

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
    # the rest of the framework as a Transport, driven with the same four calls: connect,
    # subscribe, send, and recv. A node is written once and pointed at whichever link it has.
    broker = LoopbackBroker()
    gateway: Transport = broker.rung()
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

In C# it is `Transport` in `Pamoja.Core`, disposable, with `ConnectAsync`, `SubscribeAsync`,
`SendAsync`, and `ReceiveAsync`, which returns a `TransportMessage` record, `(Topic, Payload)` with
`Text` and `Number`, or `null` once the link has ended. `ReceiveAsync(limit)` throws
`TimeoutException` when the time runs out without losing the next message, and calls on one
transport run one at a time. `MqttTransport.Open` and `CoapTransport.Open` open the network links
and `Transport.FromHandlers` wraps one written in .NET. `Transport.Faulty`, `Transport.Degraded`,
and a ladder's `Rung` take a transport and empty it; `IsAvailable` turns false, and a call on it
throws `PamojaException`, as every failure does.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/TransportGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/TransportGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/TransportGuide.cs):

```csharp
const string Topic = "sensors/1/temperature";

// Whatever a link is underneath, MQTT, CoAP, or the in-process broker here, it
// reaches the rest of the framework as a Transport, driven with the same four calls:
// connect, subscribe, send, and receive. A node is written once and pointed at
// whichever link it has.
using var broker = new LoopbackBroker();
using Transport gateway = broker.Rung();
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

## Values at a glance

**The contract, call by call:**

| Call | What it does | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- | --- |
| connect | establishes the link, which every other call needs | `connect().await?` | `await t.connect()` | `await t.connect()` | `await t.ConnectAsync()` |
| subscribe | asks for a topic, with `+` for one level and `#` for the rest | `subscribe(topic).await?` | `await t.subscribe(topic)` | `await t.subscribe(topic)` | `await t.SubscribeAsync(topic)` |
| send | publishes bytes or text to a topic | `send(topic, &bytes)`, `send_text(topic, text)` | `await t.send(topic, bytesOrText)` | `await t.send(topic, bytes_or_text)` | `await t.SendAsync(topic, bytesOrText)` |
| receive | the next message on a subscribed topic, or none once the link has ended | `recv().await?` | `await t.recv()` | `await t.recv()` | `await t.ReceiveAsync()` |
| receive with a limit | the same, giving up when the time runs out and leaving the next message queued | `timeout(limit, t.recv()).await` | `await t.recv(ms)` | `await asyncio.wait_for(t.recv(), seconds)` | `await t.ReceiveAsync(limit)` |

**The links that keep it:**

| Link | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| MQTT | `MqttTransport::new(config)` | `Transport.mqtt(options)` | `Transport.mqtt(...)` | `MqttTransport.Open(options)` |
| CoAP | `CoapTransport::new(config)` | `Transport.coap(options)` | `Transport.coap(...)` | `CoapTransport.Open(options)` |
| loopback | `LoopbackTransport::new(broker)` | `broker.rung()` | `broker.rung()` | `broker.Rung()` |
| your own | `impl Transport` | `Transport.fromHandlers(handlers)` | `Transport.from_handlers(handlers)` | `Transport.FromHandlers(handlers)` |
| a fault injector | `Faulty::new(link, failures)` | `Transport.faulty(link, failures)` | `Transport.faulty(link, failures)` | `Transport.Faulty(link, failures)` |
| a degraded link | `DegradedLink::new(link)` | `Transport.degraded(link, faults)` | `Transport.degraded(link, ...)` | `Transport.Degraded(link, ...)` |
| a ladder | `TransportLadder::new(store)` | `new Ladder(store)` | `Ladder(store)` | `new Ladder(store)` |

Rust also carries Zenoh and a LoRa mesh radio behind the same traits. [Your own link](link.md)
writes one in each language, and [Simulators](sim.md) has the degraded link.

**What a received message holds:**

| Part | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| the topic it was published to | `message.topic` | `message.topic` | `message.topic` | `message.Topic` |
| the payload | `message.payload`, a `Vec<u8>` | `message.payload`, a `Buffer` | `message.payload`, `bytes` | `message.Payload`, a `byte[]` |
| the payload as text | `message.text()?` | `message.text`, or absent | `message.text`, or `ValueError` | `message.Text` |
| the payload as a number | `message.number()?` | `message.number`, or absent | `message.number`, or `ValueError` | `message.Number`, or `null` |

## When it goes wrong

What the contract refuses, and what it says:

| What happened | The message | What to check |
| --- | --- | --- |
| a transport used after it was handed on | `this transport was already added to a ladder or a wrapper` | build another, or drive the thing it was handed to |
| a transport handed on while a call runs | `this transport is busy with a call` | await the call first |
| a link used before `connect`, or after it closed | `resource is closed` | `connect` first, or again once it has closed |
| a receive given a limit ran out of time | `no message arrived within 250 ms` in TypeScript and C# | nothing had arrived; the next message waits for the next receive |
| a fault injector refused a send | `transport error: simulated link failure` | nothing: that is its job |
| a payload read as text that is not | `the payload is not UTF-8 text`, in Python | read `payload`, the bytes |
| a payload read as a number that is not | `the payload is not a number`, in Python | read `text`, or check it first |

How each language hands a failure over:

| Language | A failed call | A receive out of time | A spent transport |
| --- | --- | --- | --- |
| Rust | `Err(pamoja_core::Error)`: `Transport`, `Closed`, `Codec`, `Io`, `Auth`, or `Unsupported` | `Err(Elapsed)` from `tokio::time::timeout` | a compile error, since the link was moved |
| TypeScript | a rejected promise with an `Error` | a rejected promise | the same, and `isAvailable` is false |
| Python | `PamojaError` | `asyncio.TimeoutError` from `asyncio.wait_for` | the same, and `is_available` is false |
| C# | `PamojaException` | `TimeoutException` | the same, and `IsAvailable` is false |

The mistakes that cost an afternoon:

- **A reading goes nowhere and nothing complains.** A link was sent to after it was handed to a
  ladder. In Rust the compiler catches it; in the bindings the call fails, so do not swallow it.
- **A subscriber hears nothing.** It subscribed after the reading went out. A link hears what is
  published after its `subscribe`, and an MQTT broker hands it an older message only when that
  one was retained.
- **A link that listens stops sending.** A receive holds the link until a message arrives, and a
  handle runs one call at a time, so a send on the same handle waits behind it. Receive with a
  limit in a loop and send between receives, or give a task that listens a link of its own.
- **A message vanished after a timer won a race.** In TypeScript and C#, a receive raced against a
  timer with `Promise.race` or `Task.WhenAny` keeps running after it loses, and takes the next
  message. Pass the limit to the receive instead, which gives up without taking anything.
- **A message arrives and the program reads nonsense.** A payload is bytes; `text` and `number`
  only work when the sender wrote text. Agree on the encoding at both ends, or use
  [Codecs](codec.md).
- **Every send fails in a test, and not in the field.** A fault injector is still in the chain.
  It fails the number of sends it was told to, then passes the rest through.

## Where next

<!-- table: next transport -->
- [Your own link](link.md): A link pamoja does not ship, written in your language against the transport contract and composed like any other.
- [Loopback](loopback.md): An in-process transport with topic matching, a fault injector, and outages on demand, for testing with no broker.
- [Transport ladder](ladder.md): Cheapest reachable link first, buffering to a store when every link is down.
- Also in Transports and testing: [MQTT](mqtt.md), [CoAP](coap.md), [Store and forward](sync.md), [Event bus](bus.md), [Simulators](sim.md).
<!-- end -->

## Reference

<!-- table: reference transport -->
- Rust: the `Transport` and `Receive` traits in [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-transport)
- TypeScript: [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-transport)
- Python: [`pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-transport)
- C#: [`Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-transport)
<!-- end -->
