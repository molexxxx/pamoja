# Loopback

Every transport in pamoja speaks the same publish-and-subscribe interface, so a
message flow can be exercised without the link that would normally carry it. The
loopback transport is that stand-in: a routing table living in the process,
shared by however many links are taken off one broker, matching topics with the
same `+` and `#` rules MQTT fixes. Nothing binds a port and nothing has to be
running, so a topic layout can be checked from a unit test instead of against a
deployment. It is also the link the [Transport ladder](ladder.md) and [Engine
surface](transport.md) pages run on, where the point being made is the
composition, a ladder or a fault injector, rather than the network underneath
it.

## What the example does

It builds one broker, takes a publisher and a subscriber off it, and moves a
temperature reading from one to the other. The subscriber's filter is
single-level, and the reading a level deeper is published first, so what comes
back is `21.5` from the temperature topic and not the `2150` sent a moment
earlier under `/raw`. The subscriber then waits 50 milliseconds for anything
more, which is how a test shows that the raw reading never reached it.

A third link joins on the multi-level filter after both of those publishes have
gone out, and takes the next `/raw` reading. The publisher is disconnected
last, and one more send shows what an unusable link does with a reading.

It proves:

- A payload published on one link arrives on another carrying the topic it was
  sent to, with no port bound and no broker process running.
- `+` matches exactly one level, so the filter takes the temperature topic and
  leaves the `/raw` reading a level below it, even though that one went out
  first.
- A receive with a time limit runs out when nothing more matches, so a test
  proves a reading did not arrive without hanging on it.
- `#` matches the levels that remain, so the second filter takes the deeper
  topic the single-level one passed over.
- A link can join a broker that has already routed traffic, take a filter of
  its own, and receive a reading published after it connects.
- A disconnected link fails the send rather than accepting a reading it has no
  way to deliver.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example loopback" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example loopback</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- loopback" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- loopback</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/loopback.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/loopback.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- loopback" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- loopback</code></div>
</div>
<!-- end -->

## Rust

In Rust, `LoopbackBroker::new()` makes the routing table and cloning it shares the table, so
every `LoopbackTransport::new(broker.clone())` is a link on the same broker. A link implements
the `Transport` and `Receive` traits, which gives it the calls every link keeps, `connect`,
`subscribe`, `send`, `send_text`, and `recv`, and it adds `is_connected()` and `disconnect()`. To
stop waiting, wrap `recv` in `tokio::time::timeout`: a receive is cancel-safe, so the message it
would have taken stays queued for the next one. `Faulty::new(link, n)` makes any link fail its
next `n` sends, and `fail_next(n)` arms it again.

<!-- snippet: examples/guides/loopback.rs#example -->
From [`examples/guides/loopback.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/loopback.rs):

```rust
use std::time::Duration;

use pamoja_core::{Receive, Transport};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};

// One broker and two links off it, all in this process. Nothing binds a port and
// nothing has to be running for the traffic below to flow, which is what makes this
// the link to develop a node against before it has a real one.
let broker = LoopbackBroker::new();
let mut publisher = LoopbackTransport::new(broker.clone());
let mut subscriber = LoopbackTransport::new(broker.clone());
publisher.connect().await?;
subscriber.connect().await?;

// A `+` stands for exactly one level, so this takes the mixer's temperature but not
// the raw reading a level below it.
subscriber.subscribe("line/+/temp").await?;
publisher.send_text("line/mixer/temp/raw", "2150").await?;
publisher.send_text("line/mixer/temp", "21.5").await?;

let message = subscriber.recv().await?.expect("a message");
let reading = message.text().expect("text");
println!("line/+/temp took {reading} from {}", message.topic);

// The raw reading went out first and never arrived, which a test proves by waiting a
// set time for anything more rather than forever. Giving up loses nothing: a message
// that came later would wait for the next receive.
let quiet = Duration::from_millis(50);
match tokio::time::timeout(quiet, subscriber.recv()).await {
    Ok(_) => println!("line/+/temp took a second reading, which should never happen"),
    Err(_) => println!(
        "line/+/temp heard nothing more within {} ms",
        quiet.as_millis()
    ),
}

// A `#` covers every level that remains, so a second link takes the whole subtree,
// including the reading the single-level filter passed over.
let mut watcher = LoopbackTransport::new(broker);
watcher.connect().await?;
watcher.subscribe("line/#").await?;
publisher.send_text("line/mixer/temp/raw", "2150").await?;

let deep = watcher.recv().await?.expect("a message");
let raw = deep.text().expect("text");
println!("line/#     took {raw} from {}", deep.topic);

// A link that has been disconnected reports the failure instead of dropping the
// reading, which is the case a test wants to reach without unplugging anything.
publisher.disconnect();
match publisher.send_text("line/mixer/temp", "21.6").await {
    Ok(_) => println!("a disconnected link took a reading, which should never happen"),
    Err(error) => println!("disconnected refused the reading: {error}"),
}
```
<!-- end -->

## TypeScript

In TypeScript, `new LoopbackBroker()` from `@pamoja/loopback` makes the broker, `broker.link()`
gives a `LoopbackTransport` to drive directly, and `broker.rung()` gives a `Transport` to hand to
a ladder or a wrapper. Every call returns a promise: `connect`, `subscribe`,
`send(topic, bufferOrText)`, `recv(timeoutMs?)`, `isConnected`, and `disconnect`. `recv`
resolves with a `TransportMessage`, `{ topic, payload, text?, number? }`. Given a limit, it
rejects once the time runs out and the next message waits for the next receive, where racing a
plain `recv()` against a timer would leave it running to take that message.

<!-- snippet: bindings/node/guides/loopback.ts#example -->
From [`bindings/node/guides/loopback.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/loopback.ts):

```typescript
import { LoopbackBroker, type TransportMessage } from '@pamoja/loopback'

const QUIET_MS = 50

async function main() {
  // One broker and two links off it, all in this process. Nothing binds a port and nothing
  // has to be running for the traffic below to flow, which is what makes this the link to
  // develop a node against before it has a real one.
  const broker = new LoopbackBroker()
  const publisher = broker.link()
  const subscriber = broker.link()
  await publisher.connect()
  await subscriber.connect()

  // A `+` stands for exactly one level, so this takes the mixer's temperature but not the
  // raw reading a level below it.
  await subscriber.subscribe('line/+/temp')
  await publisher.send('line/mixer/temp/raw', '2150')
  await publisher.send('line/mixer/temp', '21.5')

  const message = (await subscriber.recv())!
  console.log(`line/+/temp took ${message.text!} from ${message.topic}`)

  // The raw reading went out first and never arrived, which a test proves by waiting a
  // set time for anything more rather than forever. Giving up loses nothing: a message
  // that came later would wait for the next receive.
  try {
    await subscriber.recv(QUIET_MS)
    console.log('line/+/temp took a second reading, which should never happen')
  } catch {
    console.log(`line/+/temp heard nothing more within ${QUIET_MS} ms`)
  }

  // A `#` covers every level that remains, so a second link takes the whole subtree,
  // including the reading the single-level filter passed over.
  const watcher = broker.link()
  await watcher.connect()
  await watcher.subscribe('line/#')
  await publisher.send('line/mixer/temp/raw', '2150')

  const deep = (await watcher.recv())!
  console.log(`line/#     took ${deep.text!} from ${deep.topic}`)

  // A link that has been disconnected reports the failure instead of dropping the reading,
  // which is the case a test wants to reach without unplugging anything.
  await publisher.disconnect()
  try {
    await publisher.send('line/mixer/temp', '21.6')
    console.log('a disconnected link took a reading, which should never happen')
  } catch (error) {
    console.log(`disconnected refused the reading: ${(error as Error).message}`)
  }

  return { message, deep }
}

main()
```
<!-- end -->

## Python

In Python, `LoopbackBroker()` from `pamoja.loopback` makes the broker, `broker.link()` a link to
drive, and `broker.rung()` a `Transport` to compose. The calls are coroutines: `connect`,
`subscribe`, `send(topic, bytes_or_text)`, `recv`, `is_connected`, and `disconnect`. `recv`
returns a `Message` whose `payload` is bytes and whose `text` and `number` read it.
`asyncio.wait_for(link.recv(), seconds)` stops waiting, and the receive it cancels leaves its
message queued for the next one. A failure raises `PamojaError`.

<!-- snippet: bindings/python/guides/loopback.py#example -->
From [`bindings/python/guides/loopback.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/loopback.py):

```python
import asyncio

from pamoja.core import PamojaError
from pamoja.loopback import LoopbackBroker

QUIET_MS = 50


async def main() -> None:
    # One broker and two links off it, all in this process. Nothing binds a port and
    # nothing has to be running for the traffic below to flow, which is what makes this
    # the link to develop a node against before it has a real one.
    broker = LoopbackBroker()
    publisher = broker.link()
    subscriber = broker.link()
    await publisher.connect()
    await subscriber.connect()

    # A `+` stands for exactly one level, so this takes the mixer's temperature but not the
    # raw reading a level below it.
    await subscriber.subscribe("line/+/temp")
    await publisher.send("line/mixer/temp/raw", "2150")
    await publisher.send("line/mixer/temp", "21.5")

    message = await subscriber.recv()
    print(f"line/+/temp took {message.text} from {message.topic}")

    # The raw reading went out first and never arrived, which a test proves by waiting a
    # set time for anything more rather than forever. Giving up loses nothing: a message
    # that came later would wait for the next receive.
    try:
        await asyncio.wait_for(subscriber.recv(), QUIET_MS / 1000)
        print("line/+/temp took a second reading, which should never happen")
    except asyncio.TimeoutError:
        print(f"line/+/temp heard nothing more within {QUIET_MS} ms")

    # A `#` covers every level that remains, so a second link takes the whole subtree,
    # including the reading the single-level filter passed over.
    watcher = broker.link()
    await watcher.connect()
    await watcher.subscribe("line/#")
    await publisher.send("line/mixer/temp/raw", "2150")

    deep = await watcher.recv()
    print(f"line/#     took {deep.text} from {deep.topic}")

    # A link that has been disconnected reports the failure instead of dropping the
    # reading, which is the case a test wants to reach without unplugging anything.
    await publisher.disconnect()
    try:
        await publisher.send("line/mixer/temp", "21.6")
        print("a disconnected link took a reading, which should never happen")
    except PamojaError as error:
        print(f"disconnected refused the reading: {error}")

    return message, deep


message, deep = asyncio.run(main())
```
<!-- end -->

## C#

In C#, `new LoopbackBroker()` in `Pamoja.Loopback` makes the broker, `Link()` a
`LoopbackTransport` to drive, and `Rung()` a `Transport` to compose, all disposable. The calls
are `ConnectAsync`, `SubscribeAsync`, `SendAsync(topic, bytesOrText)`, `ReceiveAsync()`,
`ReceiveAsync(limit)`, `IsConnectedAsync`, and `DisconnectAsync`, and calls on one link run one at
a time. `ReceiveAsync(limit)` throws `TimeoutException` when the time runs out and leaves the next
message for the next receive, where a `ReceiveAsync()` abandoned through `Task.WhenAny` would take
it. A failure throws `PamojaException`.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LoopbackGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/LoopbackGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LoopbackGuide.cs):

```csharp
// One broker and two links off it, all in this process. Nothing binds a port and
// nothing has to be running for the traffic below to flow, which is what makes
// this the link to develop a node against before it has a real one.
using var broker = new LoopbackBroker();
using LoopbackTransport publisher = broker.Link();
using LoopbackTransport subscriber = broker.Link();
await publisher.ConnectAsync();
await subscriber.ConnectAsync();

// A `+` stands for exactly one level, so this takes the mixer's temperature but
// not the raw reading a level below it.
await subscriber.SubscribeAsync("line/+/temp");
await publisher.SendAsync("line/mixer/temp/raw", "2150");
await publisher.SendAsync("line/mixer/temp", "21.5");

TransportMessage message = (await subscriber.ReceiveAsync())!;
Console.WriteLine(
    $"line/+/temp took {message.Text}"
    + $" from {message.Topic}");

// The raw reading went out first and never arrived, which a test proves by waiting
// a set time for anything more rather than forever. Giving up loses nothing: a
// message that came later would wait for the next receive.
TimeSpan quiet = TimeSpan.FromMilliseconds(50);
try
{
    await subscriber.ReceiveAsync(quiet);
    Console.WriteLine("line/+/temp took a second reading, which should never happen");
}
catch (TimeoutException)
{
    Console.WriteLine($"line/+/temp heard nothing more within {quiet.TotalMilliseconds} ms");
}

// A `#` covers every level that remains, so a second link takes the whole subtree,
// including the reading the single-level filter passed over.
using LoopbackTransport watcher = broker.Link();
await watcher.ConnectAsync();
await watcher.SubscribeAsync("line/#");
await publisher.SendAsync("line/mixer/temp/raw", "2150");

TransportMessage deep = (await watcher.ReceiveAsync())!;
Console.WriteLine(
    $"line/#     took {deep.Text}"
    + $" from {deep.Topic}");

// A link that has been disconnected reports the failure instead of dropping the
// reading, which is the case a test wants to reach without unplugging anything.
await publisher.DisconnectAsync();
try
{
    await publisher.SendAsync("line/mixer/temp", "21.6");
    Console.WriteLine("a disconnected link took a reading, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"disconnected refused the reading: {error.Message}");
}
```
<!-- end -->

## Values at a glance

**The broker and its links:**

| What | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| a broker | `LoopbackBroker::new()`, cloned to share | `new LoopbackBroker()` | `LoopbackBroker()` | `new LoopbackBroker()` |
| a link to drive | `LoopbackTransport::new(broker.clone())` | `broker.link()` | `broker.link()` | `broker.Link()` |
| a link to compose | the same link, moved into the ladder | `broker.rung()` | `broker.rung()` | `broker.Rung()` |
| connect | `link.connect().await?` | `await link.connect()` | `await link.connect()` | `await link.ConnectAsync()` |
| subscribe | `link.subscribe(filter).await?` | `await link.subscribe(filter)` | `await link.subscribe(filter)` | `await link.SubscribeAsync(filter)` |
| send | `link.send(topic, &bytes)`, `link.send_text(topic, text)` | `await link.send(topic, bytesOrText)` | `await link.send(topic, bytes_or_text)` | `await link.SendAsync(topic, bytesOrText)` |
| receive | `link.recv().await?` | `await link.recv()` | `await link.recv()` | `await link.ReceiveAsync()` |
| receive with a limit | `timeout(limit, link.recv()).await` | `await link.recv(ms)` | `await asyncio.wait_for(link.recv(), seconds)` | `await link.ReceiveAsync(limit)` |
| is it connected | `link.is_connected()` | `await link.isConnected()` | `await link.is_connected()` | `await link.IsConnectedAsync()` |
| disconnect | `link.disconnect()` | `await link.disconnect()` | `await link.disconnect()` | `await link.DisconnectAsync()` |
| fail the next sends | `Faulty::new(link, n)`, then `fail_next(n)` | `Transport.faulty(broker.rung(), n)` | `Transport.faulty(broker.rung(), n)` | `Transport.Faulty(broker.Rung(), n)` |
| take every link out of reach | `broker.set_reachable(false)` | `broker.reachable = false` | `broker.reachable = False` | `broker.Reachable = false` |

Out of reach, every link on the broker fails to connect, send, or subscribe with `transport
error: the broker is out of reach`, including links a ladder or a wrapper now owns, and keeps its
connection and filters until the broker is back. The [Transport ladder](ladder.md) guide takes
three networks out of reach this way.

**How a filter matches a topic.** The rules are MQTT's, from sections 4.7.1 and 4.7.2 of the
OASIS MQTT 3.1.1 standard, and they are the same on the loopback as on a broker:

| Filter | Topic | Delivered | Why |
| --- | --- | --- | --- |
| `line/+/temp` | `line/mixer/temp` | yes | `+` takes exactly one level |
| `line/+/temp` | `line/mixer/temp/raw` | no | the topic has a level the filter does not |
| `line/+` | `line` | no | `+` needs a level to take, and there is none |
| `line/#` | `line/mixer/temp/raw` | yes | `#` takes every level that remains |
| `line/#` | `line` | yes | `#` takes the parent level too |
| `+/+` | `/finance` | yes | a leading `/` makes an empty first level, and `+` takes it |
| `+` | `/finance` | no | the topic has two levels |
| `#` | `$SYS/broker/uptime` | no | a filter that starts with a wildcard never takes a `$` topic |
| `$SYS/#` | `$SYS/broker/uptime` | yes | naming `$SYS` first lets the rest match |

**What the loopback does, and what a broker adds.** Code that passes here meets these
differences when it moves to [MQTT](mqtt.md); the section numbers are the MQTT 3.1.1
standard's:

| Behavior | Loopback | An MQTT broker |
| --- | --- | --- |
| who hears a publish | every link whose filters match, the publisher included | every client with a matching subscription, the publisher included (3.3.5) |
| a publish that matches two of a link's filters | one copy | at least one, and it may send one per filter (3.3.5) |
| a message published before the subscribe | never delivered | never delivered, unless it was published to be retained (3.3.1.3) |
| a link slow to read | its queue grows without a limit | the broker's own limits apply |
| a disconnect | drops what was queued; the filters stay for the next connect | with pamoja's clean session, the subscriptions end too (3.1.2.4) |
| a topic the publisher may not use | there is no access control | the broker acknowledges and drops it, or closes the connection (3.3.5) |

## When it goes wrong

What the loopback refuses, and what it says:

| What happened | The message | What to check |
| --- | --- | --- |
| a call on a link that is not connected | `resource is closed` | `connect` first, or again after `disconnect` |
| a fault injector refused a send | `transport error: simulated link failure` | nothing: that is its job |
| no message within the limit | `no message arrived within 50 ms` in TypeScript and C#, `asyncio.TimeoutError` in Python, `Elapsed` in Rust | nothing matched, which may be the point of the test |
| a composed link used after it was handed on | `this transport was already added to a ladder or a wrapper` | build another with `rung()` |
| a composed link handed on while a call runs | `this transport is busy with a call` | await the call first |

The mistakes that cost an afternoon:

- **A test hangs instead of failing.** It waits on a receive for a message that never comes, and
  a connected loopback link never ends on its own. Give the receive a limit, as the example does.
- **A test raced a receive against a timer and lost the next message.** In TypeScript and C# a
  receive nobody awaits any more keeps running, and it takes the next message. Pass the limit to
  `recv` or `ReceiveAsync` instead. Rust and Python cancel the receive when the time runs out, so
  the message stays queued.
- **Two links that should talk never do.** They were taken off two brokers: `LoopbackBroker::new()`
  called twice rather than one broker cloned, or a second `new LoopbackBroker()`. Take every link
  off the same broker.
- **A link receives its own messages.** A link subscribed to a topic it publishes on hears itself,
  as an MQTT client would. Keep commands and readings on topics of their own.
- **Memory climbs over a long run.** A link that subscribes and never reads keeps every matching
  message, because its queue has no limit. Read what a link subscribes to, or leave it
  unsubscribed.
- **A reconnected link missed what was sent while it was away.** Disconnecting drops what was
  queued for the link. Its filters stay, and apply again from the reconnect.
- **Everything passes here and fails against a real broker.** The loopback has no retained
  messages, no access control, and one copy per publish, as the table above sets out. Run the
  same code against a broker before it ships, which the [MQTT](mqtt.md) guide does.

## Where next

<!-- table: next loopback -->
- [Simulators](sim.md): Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose.
- [MQTT](mqtt.md): An MQTT client with the topic and wildcard rules, as the core transport.
- [Your own device](device.md): A sensor and an actuator pamoja has never heard of, written against the core traits, run against a rule, and published with nothing plugged in.
- Also in Transports and testing: [CoAP](coap.md), [Store and forward](sync.md), [Transport ladder](ladder.md), [Event bus](bus.md), [Engine surface](transport.md), [Your own link](link.md).
<!-- end -->

## Reference

<!-- table: reference loopback -->
- Rust: [`pamoja-loopback`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-loopback)
- TypeScript: [`@pamoja/loopback`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-loopback)
- Python: [`pamoja.loopback`](https://pamoja.molex.cloud/docs/reference/python/pamoja/loopback.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-loopback)
- C#: [`Pamoja.Loopback`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-loopback)
<!-- end -->
