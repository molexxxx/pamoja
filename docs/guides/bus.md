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

// A sampler announces something and whatever cares picks it up, with neither side
// holding a reference to the other. This is how the parts of one node are wired.
let hub: BroadcastBus<&str> = BroadcastBus::new(8);
let mut control = hub.subscribe();
let mut logger = hub.subscribe();

hub.publish("battery.low").await.expect("published");
let to_control = control.next_event().await.expect("an event");
let to_logger = logger.next_event().await.expect("an event");
println!("control saw {to_control:?}, the logger saw {to_logger:?}");

// A subscriber taken later starts from the next event, so it never sees what went out
// before it existed.
let mut late = hub.subscribe();
hub.publish("link.up").await.expect("published");
let first_seen = late.next_event().await.expect("an event");
println!("the late subscriber's first event is {first_seen:?}");

// The buffer is per subscriber and bounded, so one further behind than the capacity
// drops what it missed and resumes with the most recent events. A slow reader costs
// itself, not the publisher.
let slow: BroadcastBus<u8> = BroadcastBus::new(2);
let mut reader = slow.subscribe();
for count in 0..5u8 {
    slow.publish(count).await.expect("published");
}
let resumed = reader.next_event().await.expect("an event");
println!("after five events into a buffer of two, the reader resumes at {resumed:?}");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/bus.ts#example -->
From [`bindings/node/guides/bus.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/bus.ts):

```typescript
import { EventBus } from '@pamoja/bus'

async function main() {
  // A sampler announces something and whatever cares picks it up, with neither side
  // holding a reference to the other. This is how the parts of one node are wired.
  const hub = new EventBus(8)
  const control = await hub.subscribe()
  const logger = await hub.subscribe()

  await hub.publish(Buffer.from('battery.low'))
  const toControl = (await control.next())!
  const toLogger = (await logger.next())!
  console.log(`control saw ${toControl.toString()}, the logger saw ${toLogger.toString()}`)

  // A subscriber taken later starts from the next event, so it never sees what went out
  // before it existed.
  const late = await hub.subscribe()
  await hub.publish(Buffer.from('link.up'))
  const firstSeen = (await late.next())!
  console.log(`the late subscriber's first event is ${firstSeen.toString()}`)

  // The buffer is per subscriber and bounded, so one further behind than the capacity
  // drops what it missed and resumes with the most recent events. A slow reader costs
  // itself, not the publisher.
  const slow = new EventBus(2)
  const reader = await slow.subscribe()
  for (let count = 0; count < 5; count += 1) {
    await slow.publish(Buffer.from([count]))
  }
  const resumed = (await reader.next())!
  console.log(`after five events into a buffer of two, the reader resumes at ${resumed[0]}`)

  return { toControl, toLogger, firstSeen, resumed }
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
    # A sampler announces something and whatever cares picks it up, with neither side
    # holding a reference to the other. This is how the parts of one node are wired.
    hub = EventBus(8)
    control = await hub.subscribe()
    logger = await hub.subscribe()

    await hub.publish(b"battery.low")
    to_control = await control.next_event()
    to_logger = await logger.next_event()
    print(f"control saw {to_control.decode()}, the logger saw {to_logger.decode()}")

    # A subscriber taken later starts from the next event, so it never sees what went out
    # before it existed.
    late = await hub.subscribe()
    await hub.publish(b"link.up")
    first_seen = await late.next_event()
    print(f"the late subscriber's first event is {first_seen.decode()}")

    # The buffer is per subscriber and bounded, so one further behind than the capacity
    # drops what it missed and resumes with the most recent events. A slow reader costs
    # itself, not the publisher.
    slow = EventBus(2)
    reader = await slow.subscribe()
    for count in range(5):
        await slow.publish(bytes([count]))
    resumed = await reader.next_event()
    print(f"after five events into a buffer of two, the reader resumes at {resumed[0]}")

    return to_control, to_logger, first_seen, resumed


to_control, to_logger, first_seen, resumed = asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs):

```csharp
// A sampler announces something and whatever cares picks it up, with neither side
// holding a reference to the other. This is how the parts of one node are wired.
using EventBus hub = new EventBus(8);
using EventBus control = hub.Subscribe();
using EventBus logger = hub.Subscribe();

await hub.PublishAsync("battery.low"u8.ToArray());
byte[] toControl = (await control.NextAsync())!;
byte[] toLogger = (await logger.NextAsync())!;
Console.WriteLine(
    $"control saw {System.Text.Encoding.UTF8.GetString(toControl)},"
    + $" the logger saw {System.Text.Encoding.UTF8.GetString(toLogger)}");

// A subscriber taken later starts from the next event, so it never sees what went
// out before it existed.
using EventBus late = hub.Subscribe();
await hub.PublishAsync("link.up"u8.ToArray());
byte[] firstSeen = (await late.NextAsync())!;
Console.WriteLine(
    $"the late subscriber's first event is"
    + $" {System.Text.Encoding.UTF8.GetString(firstSeen)}");

// The buffer is per subscriber and bounded, so one further behind than the
// capacity drops what it missed and resumes with the most recent events. A slow
// reader costs itself, not the publisher.
using EventBus slow = new EventBus(2);
using EventBus reader = slow.Subscribe();
for (byte count = 0; count < 5; count++)
{
    await slow.PublishAsync(new byte[] { count });
}

byte[] resumed = (await reader.NextAsync())!;
Console.WriteLine(
    $"after five events into a buffer of two, the reader resumes at {resumed[0]}");
```
<!-- end -->

## Reference

<!-- table: reference bus -->
- Rust: [`pamoja-bus`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html)
- TypeScript: [`@pamoja/bus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html)
- Python: [`pamoja.bus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html)
- C#: [`Pamoja.Bus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html)
<!-- end -->
