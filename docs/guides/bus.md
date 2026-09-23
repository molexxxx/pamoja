# Event bus

Inside one node, the part that samples a sensor and the parts that care about the
reading should not have to know about each other. A sampler that holds a reference
to the logger, the alerting rule, and the uplink is a sampler that has to change
every time one of them does. The bus removes those references: something publishes
an event, and everything subscribed receives it.

Each subscriber has its own place in a bounded buffer, and that is the design
decision worth knowing. A subscriber that falls further behind than the buffer holds
loses the events it missed, resumes with the most recent ones, and can count what it
lost. The publisher never waits for a slow reader, which is the right trade on a
device where the sampler is closer to real time than the part writing to flash. If an
event must not be lost, it belongs in a [store](sync.md), not on the bus.

A subscriber also sees only what is published after it subscribes. There is no
history to replay, so a part that needs the current state when it starts reads it
from wherever that state is kept, not from the bus.

## What the example does

It wires the parts of a solar weather station to one bus. A power monitor and a wind
sampler announce, a heater and a logger listen, and a radio joins after the others
are running. Each endpoint buffers two events, few enough that the logger falls
behind within the example.

It proves:

- One announcement reaches both listening parts, and the power monitor learns that it
  reached 2.
- Each part reads its own copy: the logger still has `battery.low` waiting after the
  heater took it.
- A part can publish while its own wait for the next event is open, and it hears
  what it publishes, so the heater's open wait takes the `heater.off` it sent.
- A part that joins late never sees what went out before it subscribed. The radio's
  first event is `battery.ok`, not `battery.low`.
- A reader that falls behind loses the oldest events, not the newest. Seven events go
  out after the logger's last read, the buffer holds two, and the logger counts the 5
  it missed and resumes at `wind 3`.
- Every publish returns at once while the logger is behind, so a slow reader costs
  itself, not the publisher.
- A wait with a limit gives up after 50 ms on a quiet bus and takes nothing.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example bus" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example bus</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- bus" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- bus</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/bus.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/bus.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- bus" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- bus</code></div>
</div>
<!-- end -->

## Rust

In Rust, `EventPublisher::new(capacity)` makes a bus with no subscribers yet and a publisher on
it, and `BroadcastBus::new(capacity)` makes one with an endpoint. `subscribe()` on either gives a
`BroadcastBus` endpoint, which implements the core `EventBus` trait: `publish(event).await` and
`next_event().await`. On this bus `next_event` returns `Some` and never `None`, because every
endpoint keeps the bus open. `publisher()` on either gives another `EventPublisher`, which is
`Clone` and `Send`, and its `publish` is not async: it returns at once with the number of
subscribers it reached. A part announces through one while it waits, because `next_event`
borrows its endpoint for the whole wait. `missed()` counts what an endpoint lost by falling
behind. To stop waiting, wrap `next_event` in `tokio::time::timeout` or a `select!`: the wait is
cancel-safe, so one given up takes no event. Events are any `Clone` type, so a Rust bus can carry
an enum of the node's events where the other languages carry text or bytes.

<!-- snippet: examples/guides/bus.rs#example -->
From [`examples/guides/bus.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/bus.rs):

```rust
use std::time::Duration;

use pamoja_bus::EventPublisher;
use pamoja_core::EventBus;

// The station's wiring makes one bus and hands each part what it needs: a
// publisher to announce, an endpoint to listen. No part holds a reference to
// another, so any of them can be replaced without touching the rest.
let bus: EventPublisher<String> = EventPublisher::new(2);
let power = bus.publisher();
let sampler = bus.publisher();
let mut heater = bus.subscribe();
let mut logger = bus.subscribe();

// One announcement reaches every part that listens, and each reads its own copy.
let reached = power.publish("battery.low".into());
println!("power     handed battery.low to {reached} parts");
let heater_took = heater.next_event().await?.expect("an event");
println!("heater    took {heater_took}");
let logger_took = logger.next_event().await?.expect("an event");
println!("logger    took {logger_took}");

// Publishing never waits, even while the part's own wait is open, and a part
// hears what it publishes. The wait borrows the endpoint, so in Rust the heater
// announces through a publisher of its own.
let heater_says = heater.publisher();
let (heard, _) = tokio::join!(heater.next_event(), async {
    heater_says.publish("heater.off".into())
});
let heard = heard?.expect("an event");
println!("heater    heard its own {heard}, sent while it waited");

// A part that joins late sees only what is published after it subscribes. There
// is no history to replay.
let mut radio = bus.subscribe();
power.publish("battery.ok".into());
let first = radio.next_event().await?.expect("an event");
println!("radio     joined late, so the first event it sees is {first}");

// Each endpoint buffers two events. The logger, busy writing to flash, falls
// behind while the sampler publishes five readings: it loses the oldest events,
// resumes with the newest, and counts what it lost.
for reading in 0..5 {
    sampler.publish(format!("wind {reading}"));
}
let resumed = logger.next_event().await?.expect("an event");
let missed = logger.missed();
println!("logger    missed {missed} and resumes at {resumed}");
let newest = logger.next_event().await?.expect("an event");
println!("logger    then took {newest}");

// A wait with a limit gives up without taking anything, so a part can do other
// work between events and lose nothing by it.
let quiet = Duration::from_millis(50);
match tokio::time::timeout(quiet, logger.next_event()).await {
    Ok(_) => println!("logger    took an event no one published, which should never happen"),
    Err(_) => println!(
        "logger    heard nothing more within {} ms",
        quiet.as_millis()
    ),
}
```
<!-- end -->

## TypeScript

In TypeScript, `new EventPublisher(capacity?)` from `@pamoja/bus` makes a bus and a publisher,
and `new EventBus(capacity?)` makes a bus and an endpoint, 64 events deep unless told otherwise.
`subscribe()`, `publisher()`, and `publish(event)` return at once. `publish` takes a `Buffer` or
a string, and on a publisher it returns how many subscribers it reached. Only the waits return
promises: `next(timeoutMs?)` resolves with a `Buffer` and `nextText(timeoutMs?)` with a string.
Given a limit, a wait rejects with `no event arrived within N ms` and leaves the next event for
the next call. Without one it waits as long as it takes, and a wait still open keeps Node from
exiting. `missed` counts what an endpoint lost.

<!-- snippet: bindings/node/guides/bus.ts#example -->
From [`bindings/node/guides/bus.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/bus.ts):

```typescript
import { EventPublisher } from '@pamoja/bus'

const QUIET_MS = 50

async function main() {
  // The station's wiring makes one bus and hands each part what it needs: a publisher
  // to announce, an endpoint to listen. No part holds a reference to another, so any of
  // them can be replaced without touching the rest.
  const bus = new EventPublisher(2)
  const power = bus.publisher()
  const sampler = bus.publisher()
  const heater = bus.subscribe()
  const logger = bus.subscribe()

  // One announcement reaches every part that listens, and each reads its own copy.
  const reached = power.publish('battery.low')
  console.log(`power     handed battery.low to ${reached} parts`)
  const heaterTook = await heater.nextText()
  console.log(`heater    took ${heaterTook}`)
  const loggerTook = await logger.nextText()
  console.log(`logger    took ${loggerTook}`)

  // Publishing never waits, even while the part's own wait is open, and a part hears
  // what it publishes.
  const waiting = heater.nextText()
  heater.publish('heater.off')
  const heard = await waiting
  console.log(`heater    heard its own ${heard}, sent while it waited`)

  // A part that joins late sees only what is published after it subscribes. There is
  // no history to replay.
  const radio = bus.subscribe()
  power.publish('battery.ok')
  const first = await radio.nextText()
  console.log(`radio     joined late, so the first event it sees is ${first}`)

  // Each endpoint buffers two events. The logger, busy writing to flash, falls behind
  // while the sampler publishes five readings: it loses the oldest events, resumes with
  // the newest, and counts what it lost.
  for (let reading = 0; reading < 5; reading += 1) {
    sampler.publish(`wind ${reading}`)
  }
  const resumed = await logger.nextText()
  const missed = logger.missed
  console.log(`logger    missed ${missed} and resumes at ${resumed}`)
  const newest = await logger.nextText()
  console.log(`logger    then took ${newest}`)

  // A wait with a limit gives up without taking anything, so a part can do other work
  // between events and lose nothing by it.
  try {
    await logger.nextText(QUIET_MS)
    console.log('logger    took an event no one published, which should never happen')
  } catch {
    console.log(`logger    heard nothing more within ${QUIET_MS} ms`)
  }

  return { reached, heaterTook, loggerTook, heard, first, missed, resumed, newest }
}

main()
```
<!-- end -->

## Python

In Python, `EventPublisher(capacity=64)` and `EventBus(capacity=64)` come from `pamoja.bus`.
`subscribe()`, `publisher()`, and `publish(event)` are plain calls that return at once, so a
callback on another thread can publish without an event loop. `publish` takes `str` or `bytes`,
and on a publisher it returns how many subscribers it reached. Only the waits are awaited:
`next_event()` gives `bytes` and `next_text()` gives `str`. `asyncio.wait_for(endpoint.next_text(),
seconds)` stops waiting, and the wait it cancels takes no event. `missed` is a property that
counts what an endpoint lost. A failure raises `PamojaError`.

<!-- snippet: bindings/python/guides/bus.py#example -->
From [`bindings/python/guides/bus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/bus.py):

```python
import asyncio

from pamoja.bus import EventPublisher

QUIET_MS = 50


async def main() -> None:
    # The station's wiring makes one bus and hands each part what it needs: a publisher
    # to announce, an endpoint to listen. No part holds a reference to another, so any
    # of them can be replaced without touching the rest.
    bus = EventPublisher(2)
    power = bus.publisher()
    sampler = bus.publisher()
    heater = bus.subscribe()
    logger = bus.subscribe()

    # One announcement reaches every part that listens, and each reads its own copy.
    reached = power.publish("battery.low")
    print(f"power     handed battery.low to {reached} parts")
    heater_took = await heater.next_text()
    print(f"heater    took {heater_took}")
    logger_took = await logger.next_text()
    print(f"logger    took {logger_took}")

    # Publishing never waits, even while the part's own wait is open, and a part hears
    # what it publishes.
    waiting = heater.next_text()
    heater.publish("heater.off")
    heard = await waiting
    print(f"heater    heard its own {heard}, sent while it waited")

    # A part that joins late sees only what is published after it subscribes. There is
    # no history to replay.
    radio = bus.subscribe()
    power.publish("battery.ok")
    first = await radio.next_text()
    print(f"radio     joined late, so the first event it sees is {first}")

    # Each endpoint buffers two events. The logger, busy writing to flash, falls behind
    # while the sampler publishes five readings: it loses the oldest events, resumes
    # with the newest, and counts what it lost.
    for reading in range(5):
        sampler.publish(f"wind {reading}")
    resumed = await logger.next_text()
    missed = logger.missed
    print(f"logger    missed {missed} and resumes at {resumed}")
    newest = await logger.next_text()
    print(f"logger    then took {newest}")

    # A wait with a limit gives up without taking anything, so a part can do other work
    # between events and lose nothing by it.
    try:
        await asyncio.wait_for(logger.next_text(), QUIET_MS / 1000)
        print("logger    took an event no one published, which should never happen")
    except asyncio.TimeoutError:
        print(f"logger    heard nothing more within {QUIET_MS} ms")

    return reached, heater_took, logger_took, heard, first, missed, resumed, newest


seen = asyncio.run(main())
```
<!-- end -->

## C#

In C#, `new EventPublisher(capacity)` and `new EventBus(capacity)` in `Pamoja.Bus` are
disposable and 64 events deep by default. `Subscribe()`, `Publisher()`, and
`Publish(textOrBytes)` return at once and are safe to call from any thread, and
`EventPublisher.Publish` returns how many subscribers it reached. `NextAsync()` gives an event's
bytes and `NextTextAsync()` its text. Each has an overload that takes a `TimeSpan` and throws
`TimeoutException` when the time runs out, leaving the next event for the next call. `Missed`
counts what an endpoint lost. A wait holds a thread-pool thread until it returns. A failure
throws `PamojaException`.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs):

```csharp
// The station's wiring makes one bus and hands each part what it needs: a
// publisher to announce, an endpoint to listen. No part holds a reference to
// another, so any of them can be replaced without touching the rest.
using var bus = new EventPublisher(2);
using EventPublisher power = bus.Publisher();
using EventPublisher sampler = bus.Publisher();
using EventBus heater = bus.Subscribe();
using EventBus logger = bus.Subscribe();

// One announcement reaches every part that listens, and each reads its own copy.
int reached = power.Publish("battery.low");
Console.WriteLine($"power     handed battery.low to {reached} parts");
string heaterTook = await heater.NextTextAsync();
Console.WriteLine($"heater    took {heaterTook}");
string loggerTook = await logger.NextTextAsync();
Console.WriteLine($"logger    took {loggerTook}");

// Publishing never waits, even while the part's own wait is open, and a part
// hears what it publishes.
Task<string> waiting = heater.NextTextAsync();
heater.Publish("heater.off");
string heard = await waiting;
Console.WriteLine($"heater    heard its own {heard}, sent while it waited");

// A part that joins late sees only what is published after it subscribes.
// There is no history to replay.
using EventBus radio = bus.Subscribe();
power.Publish("battery.ok");
string first = await radio.NextTextAsync();
Console.WriteLine($"radio     joined late, so the first event it sees is {first}");

// Each endpoint buffers two events. The logger, busy writing to flash, falls
// behind while the sampler publishes five readings: it loses the oldest
// events, resumes with the newest, and counts what it lost.
for (int reading = 0; reading < 5; reading++)
{
    sampler.Publish($"wind {reading}");
}

string resumed = await logger.NextTextAsync();
long missed = logger.Missed;
Console.WriteLine($"logger    missed {missed} and resumes at {resumed}");
string newest = await logger.NextTextAsync();
Console.WriteLine($"logger    then took {newest}");

// A wait with a limit gives up without taking anything, so a part can do
// other work between events and lose nothing by it.
TimeSpan quiet = TimeSpan.FromMilliseconds(50);
try
{
    await logger.NextTextAsync(quiet);
    Console.WriteLine("logger    took an event no one published, which should never happen");
}
catch (TimeoutException)
{
    Console.WriteLine($"logger    heard nothing more within {quiet.TotalMilliseconds} ms");
}
```
<!-- end -->

## Values at a glance

**The bus and its handles:**

| What | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| a bus and a publisher | `EventPublisher::new(capacity)` | `new EventPublisher(capacity?)` | `EventPublisher(capacity=64)` | `new EventPublisher(capacity)` |
| a bus and an endpoint | `BroadcastBus::new(capacity)` | `new EventBus(capacity?)` | `EventBus(capacity=64)` | `new EventBus(capacity)` |
| another endpoint | `subscribe()` | `subscribe()` | `subscribe()` | `Subscribe()` |
| another publisher | `publisher()` | `publisher()` | `publisher()` | `Publisher()` |
| announce from a publisher | `publish(event)` | `publish(event)` | `publish(event)` | `Publish(event)` |
| announce from an endpoint | `publish(event).await?` | `publish(event)` | `publish(event)` | `Publish(event)` |
| wait for the next event | `next_event().await?` | `await next()`, `await nextText()` | `await next_event()`, `await next_text()` | `await NextAsync()`, `await NextTextAsync()` |
| wait with a limit | `timeout(limit, next_event()).await` | `await nextText(ms)` | `await asyncio.wait_for(next_text(), seconds)` | `await NextTextAsync(limit)` |
| events lost by falling behind | `missed()` | `missed` | `missed` | `Missed` |

**What each handle does:**

| | An endpoint | A publisher |
| --- | --- | --- |
| its type | `BroadcastBus` in Rust, `EventBus` in the others | `EventPublisher` |
| publishes | yes, and hears what it publishes | yes |
| receives | what is published after it subscribed | nothing |
| falls behind | yes, and counts what it lost | never: it has nothing to fill |
| `publish` returns | nothing | how many subscribers it reached |

**How far an endpoint can fall behind.** The capacity is rounded up to a power of two and
capped at 1,048,576 events:

| Capacity asked for | Events an endpoint can fall behind |
| --- | --- |
| 0 or 1 | 1 |
| 2 | 2 |
| 3 or 4 | 4 |
| 5 to 8 | 8 |
| 64, the default in TypeScript, Python, and C# | 64 |
| 100 | 128 |
| more than 1,048,576 | 1,048,576 |

The buffer is one ring shared by every endpoint on the bus, set aside in full when the bus is
made, and each endpoint reads it from its own position. Another subscriber costs a position, not
a copy of the buffer, and an event's memory is freed once every endpoint has read it or the ring
has come round to its slot again.

## When it goes wrong

What the bus refuses, and what it says:

| What happened | The message | What to check |
| --- | --- | --- |
| no event within the limit | `no event arrived within 50 ms` in TypeScript and C#, `asyncio.TimeoutError` in Python, `Elapsed` in Rust | nothing was published, which may be what the part was checking |
| an event read as text that is not UTF-8 | `the event is not UTF-8 text` in TypeScript and Python; C# puts U+FFFD in place of the bad bytes | read it as bytes with `next`, `next_event`, or `NextAsync` |
| a negative time limit | `a time limit must be 0 ms or more` in TypeScript, `ArgumentOutOfRangeException` in C#; Python gives up at once | the arithmetic that made the limit |
| a negative capacity | `a capacity must be 0 or more` in TypeScript, `OverflowError` in Python, `ArgumentOutOfRangeException` in C# | the arithmetic that made the capacity |
| a publish of something that is neither text nor bytes | `Value is none of these types` in TypeScript, `TypeError` in Python | write it out as text first, or encode it with CBOR |

The mistakes that cost an afternoon:

- **A part never sees an event it should have.** It subscribed after the event went out. Take
  every endpoint before starting anything that publishes, and have a part that needs the current
  state when it starts read that state from where it is kept.
- **The log has holes.** The logger fell further behind than the buffer holds, and `missed` says
  by how many. Raise the capacity to cover the slowest reader's worst burst, take slow work such
  as flash writes out of the reading loop, or keep events that must never be lost in a
  [store](sync.md).
- **A part reacts to its own announcement.** An endpoint hears what it publishes, as the heater
  does in the example. A part that only announces takes a publisher, which hears nothing, and a
  part that both listens and announces skips the events it sent.
- **Two parts share an endpoint and each sees about half the events.** Waits on one endpoint take
  turns, and each event goes to whichever wait is first in line. Give every part its own endpoint
  with `subscribe()`.
- **In Rust, the heater cannot publish while it waits.** `next_event` borrows the endpoint
  mutably for the whole wait, so publishing on the same endpoint fails to compile with ``cannot
  borrow `heater` as immutable because it is also borrowed as mutable``. Take a `publisher()`
  first and announce through it, as the example does.
- **A TypeScript program never exits.** A `next()` with no limit is still waiting, and an open
  wait keeps Node running. Give the last wait a limit.
- **A wait raced against a timer took the next event.** In TypeScript and C#, a wait that nobody
  awaits any more keeps running, and it takes the next event. Pass the limit to `next` or
  `NextAsync` instead of racing with `Promise.race` or `Task.WhenAny`. Rust and Python cancel the
  wait, so the event stays for the next one.
- **In C#, many waiting parts slow unrelated work down.** Each open wait holds a thread-pool
  thread, and once the pool's minimum is used up it adds threads slowly. Keep the number of waits
  open at once small, for example one endpoint that reads and hands each event to the parts that
  need it.

## Where next

<!-- table: next bus -->
- [Rules](rules.md): Rules between nodes as a file.
- [Your own device](device.md): A sensor and an actuator pamoja has never heard of, written against the core traits, run against a rule, and published with nothing plugged in.
- [Loopback](loopback.md): An in-process transport with topic matching, a fault injector, and outages on demand, for testing with no broker.
- Also in Transports and testing: [MQTT](mqtt.md), [CoAP](coap.md), [Store and forward](sync.md), [Transport ladder](ladder.md), [Engine surface](transport.md), [Your own link](link.md), [Simulators](sim.md).
<!-- end -->

## Reference

<!-- table: reference bus -->
- Rust: [`pamoja-bus`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-bus)
- TypeScript: [`@pamoja/bus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-bus)
- Python: [`pamoja.bus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-bus)
- C#: [`Pamoja.Bus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-bus)
<!-- end -->
