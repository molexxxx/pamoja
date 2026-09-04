# Event bus

Inside one node, the part that samples a sensor and the parts that care about the
reading should not have to know about each other. A sampler that holds a reference
to the logger, the alerting rule, and the uplink is a sampler that has to change
every time one of them does. The bus removes those references: something publishes
an event, and everything subscribed receives it.

Each subscriber has its own buffer, and that is the design decision worth knowing.
A subscriber that falls further behind than the buffer holds loses the events it
missed and resumes with the most recent ones. The publisher is never blocked by a
slow reader, which is the right trade on a device where the sampler is closer to
real time than the thing writing to flash. If an event must not be lost, it belongs
in a store, not on the bus.

A subscriber also only sees what is published after it subscribes. There is no
backlog to replay, so ordering between subscribers is by arrival rather than by
history.

## What the example does

It publishes to two subscribers, takes a third subscriber part-way through, and
then overruns a two-event buffer with five events.

It proves:

- Every subscriber receives the same published event, independently.
- A subscriber taken later starts from the next event and never sees what went
  out before it existed.
- A subscriber further behind than the buffer resumes at the most recent events
  rather than blocking the publisher or replaying the ones it missed.

## Rust

<!-- snippet: examples/tests/guides/bus.rs#example -->
From [`examples/tests/guides/bus.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/bus.rs):

```rust
use pamoja_bus::BroadcastBus;
use pamoja_core::EventBus;

// A sampler announces a reading and whatever cares about readings picks it up, with
// neither side holding a reference to the other.
let hub: BroadcastBus<&str> = BroadcastBus::new(8);
let mut sampler = hub.subscribe();
let mut logger = hub.subscribe();

hub.publish("battery.low").await.unwrap();
assert_eq!(sampler.next_event().await.unwrap(), Some("battery.low"));
assert_eq!(logger.next_event().await.unwrap(), Some("battery.low"));

// A subscriber taken later starts from the next event, so it never sees what went out
// before it existed.
let mut late = hub.subscribe();
hub.publish("link.up").await.unwrap();
assert_eq!(late.next_event().await.unwrap(), Some("link.up"));
assert_eq!(sampler.next_event().await.unwrap(), Some("link.up"));

// The buffer is per subscriber and bounded, so one further behind than the capacity
// drops what it missed and resumes with the most recent events. A slow reader costs
// itself, not the publisher.
let slow: BroadcastBus<u8> = BroadcastBus::new(2);
let mut reader = slow.subscribe();
for count in 0..5u8 {
    slow.publish(count).await.unwrap();
}
assert_eq!(reader.next_event().await.unwrap(), Some(3));
assert_eq!(reader.next_event().await.unwrap(), Some(4));
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/bus.ts#example -->
From [`bindings/node/guides/bus.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/bus.ts):

```typescript
import assert from 'node:assert/strict'

import { EventBus } from '@pamoja/bus'

async function main(): Promise<void> {
  // A sampler announces a reading and whatever cares about readings picks it up, with
  // neither side holding a reference to the other.
  const hub = new EventBus(8)
  const sampler = await hub.subscribe()
  const logger = await hub.subscribe()

  await hub.publish(Buffer.from('battery.low'))
  assert.equal((await sampler.next())!.toString(), 'battery.low')
  assert.equal((await logger.next())!.toString(), 'battery.low')

  // An endpoint taken later starts from the next event, so it never sees what went out
  // before it existed.
  const late = await hub.subscribe()
  await hub.publish(Buffer.from('link.up'))
  assert.equal((await late.next())!.toString(), 'link.up')
  assert.equal((await sampler.next())!.toString(), 'link.up')

  // The buffer is per endpoint and bounded, so an endpoint further behind than the
  // capacity drops what it missed and resumes with the most recent events.
  const slow = new EventBus(2)
  const reader = await slow.subscribe()
  for (let count = 0; count < 5; count += 1) {
    await slow.publish(Buffer.from([count]))
  }
  assert.equal((await reader.next())![0], 3)
  assert.equal((await reader.next())![0], 4)
}

main()
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/bus.py#example -->
From [`bindings/python/guides/bus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/bus.py):

```python
import asyncio

from pamoja.bus import EventBus


async def main() -> None:
    # A sampler announces a reading and whatever cares about readings picks it up,
    # with neither side holding a reference to the other.
    hub = EventBus(8)
    sampler = await hub.subscribe()
    logger = await hub.subscribe()

    await hub.publish(b"battery.low")
    assert await sampler.next_event() == b"battery.low"
    assert await logger.next_event() == b"battery.low"

    # An endpoint taken later starts from the next event, so it never sees what went
    # out before it existed.
    late = await hub.subscribe()
    await hub.publish(b"link.up")
    assert await late.next_event() == b"link.up"
    assert await sampler.next_event() == b"link.up"

    # The buffer is per endpoint and bounded, so an endpoint further behind than the
    # capacity drops what it missed and resumes with the most recent events.
    slow = EventBus(2)
    reader = await slow.subscribe()
    for count in range(5):
        await slow.publish(bytes([count]))
    assert await reader.next_event() == b"\x03"
    assert await reader.next_event() == b"\x04"


asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs):

```csharp
// A sampler announces a reading and whatever cares about readings picks it up,
// with neither side holding a reference to the other.
using EventBus hub = new EventBus(8);
using EventBus sampler = hub.Subscribe();
using EventBus logger = hub.Subscribe();

await hub.PublishAsync("battery.low"u8.ToArray());
Expect(
    (await sampler.NextAsync())!.AsSpan().SequenceEqual("battery.low"u8),
    "the sampler's endpoint received the event");
Expect(
    (await logger.NextAsync())!.AsSpan().SequenceEqual("battery.low"u8),
    "and so did the logger's");

// An endpoint taken later starts from the next event, so it never sees what went
// out before it existed.
using EventBus late = hub.Subscribe();
await hub.PublishAsync("link.up"u8.ToArray());
Expect(
    (await late.NextAsync())!.AsSpan().SequenceEqual("link.up"u8),
    "the endpoint taken last begins at the event after it");
Expect(
    (await sampler.NextAsync())!.AsSpan().SequenceEqual("link.up"u8),
    "an endpoint that was already there follows on in order");

// The buffer is per endpoint and bounded, so an endpoint further behind than the
// capacity drops what it missed and resumes with the most recent events.
using EventBus slow = new EventBus(2);
using EventBus reader = slow.Subscribe();
for (byte count = 0; count < 5; count++)
{
    await slow.PublishAsync(new byte[] { count });
}

Expect((await reader.NextAsync())![0] == 3, "the events it fell behind on were dropped");
Expect((await reader.NextAsync())![0] == 4, "and it resumes with the most recent");
```
<!-- end -->

## Reference

<!-- table: reference bus -->
- Rust: [`pamoja-bus`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html)
- TypeScript: [`@pamoja/bus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html)
- Python: [`pamoja.bus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html)
- C#: [`Pamoja.Bus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html)
<!-- end -->
