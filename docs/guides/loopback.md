# Loopback

Every transport in pamoja speaks the same publish-and-subscribe interface, so a
message flow can be exercised without the link that would normally carry it. The
loopback transport is that stand-in: a routing table living in the process,
shared by however many links are taken off one broker, matching topics with the
same `+` and `#` rules MQTT fixes. Nothing binds a port and nothing has to be
running, so a topic layout can be checked from a unit test instead of against a
deployment. It is also the link the MQTT and CoAP pages reach for when the point
being made is the composition, a ladder or a fault injector, rather than the
network underneath it.

## What the example does

It builds a broker, takes a publisher and a subscriber off it, subscribes to a
single-level pattern, and reads back what a publish on the other link produced.
A third link on a multi-level pattern then shows where the two wildcards differ,
and the publisher is disconnected to see what an unusable link does with a send.

It proves:

- A payload published on one link arrives on another carrying the topic it was
  sent to, with no broker, no network, and no hardware involved.
- `+` matches exactly one level: `line/+/temp` takes `line/mixer/temp` and
  leaves `line/mixer/temp/raw` for someone else.
- `#` matches the levels that remain, so `line/#` takes the deeper topic the
  single-level filter passed over.
- A disconnected link fails the send rather than accepting a reading it has no
  way to deliver.

## Rust

<!-- snippet: examples/tests/guides/loopback.rs#example -->
From [`examples/tests/guides/loopback.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/loopback.rs):

```rust
use pamoja_core::Transport;
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};

// One broker and two links off it, all in this process. Nothing binds a port and
// nothing has to be running for the traffic below to flow.
let broker = LoopbackBroker::new();
let mut publisher = LoopbackTransport::new(broker.clone());
let mut subscriber = LoopbackTransport::new(broker.clone());
publisher.connect().await.expect("connect");
subscriber.connect().await.expect("connect");

// A `+` stands for exactly one level, so the deeper topic is not delivered here and
// the first message this subscriber sees is the second publish.
subscriber
    .subscribe("line/+/temp")
    .await
    .expect("subscribe");
publisher
    .send("line/mixer/temp/raw", b"2150")
    .await
    .expect("send");
publisher
    .send("line/mixer/temp", b"21.5")
    .await
    .expect("send");

let message = subscriber.recv().await.expect("recv").expect("a message");
assert_eq!(message.topic, "line/mixer/temp");
assert_eq!(message.payload, b"21.5");

// A `#` covers the levels that remain, so a second link takes the whole subtree,
// including the reading the single-level filter passed over.
let mut watcher = LoopbackTransport::new(broker);
watcher.connect().await.expect("connect");
watcher.subscribe("line/#").await.expect("subscribe");
publisher
    .send("line/mixer/temp/raw", b"2150")
    .await
    .expect("send");

let deep = watcher.recv().await.expect("recv").expect("a message");
assert_eq!(deep.topic, "line/mixer/temp/raw");
assert_eq!(deep.payload, b"2150");

// A link that has been disconnected reports the failure instead of dropping the
// reading, which is the case a test wants to reach without unplugging anything.
publisher.disconnect();
assert!(publisher.send("line/mixer/temp", b"21.6").await.is_err());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/loopback.ts#example -->
From [`bindings/node/guides/loopback.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/loopback.ts):

```typescript
import assert from 'node:assert/strict'

import { LoopbackBroker } from '@pamoja/loopback'

async function main() {
  // One broker and two links off it, all in this process. Nothing binds a port and
  // nothing has to be running for the traffic below to flow.
  const broker = new LoopbackBroker()
  const publisher = broker.link()
  const subscriber = broker.link()
  await publisher.connect()
  await subscriber.connect()

  // A `+` stands for exactly one level, so the deeper topic is not delivered here and
  // the first message this subscriber sees is the second publish.
  await subscriber.subscribe('line/+/temp')
  await publisher.send('line/mixer/temp/raw', Buffer.from('2150'))
  await publisher.send('line/mixer/temp', Buffer.from('21.5'))

  const message = await subscriber.recv()
  assert.equal(message?.topic, 'line/mixer/temp')
  assert.equal(message?.payload.toString(), '21.5')

  // A `#` covers the levels that remain, so a second link takes the whole subtree,
  // including the reading the single-level filter passed over.
  const watcher = broker.link()
  await watcher.connect()
  await watcher.subscribe('line/#')
  await publisher.send('line/mixer/temp/raw', Buffer.from('2150'))

  const deep = await watcher.recv()
  assert.equal(deep?.topic, 'line/mixer/temp/raw')
  assert.equal(deep?.payload.toString(), '2150')

  // A link that has been disconnected reports the failure instead of dropping the
  // reading, which is the case a test wants to reach without unplugging anything.
  await publisher.disconnect()
  await assert.rejects(() => publisher.send('line/mixer/temp', Buffer.from('21.6')))
}

main()
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/loopback.py#example -->
From [`bindings/python/guides/loopback.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/loopback.py):

```python
import asyncio

from pamoja.core import PamojaError
from pamoja.loopback import LoopbackBroker


async def main() -> None:
    # One broker and two links off it, all in this process. Nothing binds a port and
    # nothing has to be running for the traffic below to flow.
    broker = LoopbackBroker()
    publisher = broker.link()
    subscriber = broker.link()
    await publisher.connect()
    await subscriber.connect()

    # A `+` stands for exactly one level, so the deeper topic is not delivered here and
    # the first message this subscriber sees is the second publish.
    await subscriber.subscribe("sensors/+/temperature")
    await publisher.send("sensors/8/temperature/raw", b"2150")
    await publisher.send("sensors/8/temperature", b"21.5")

    message = await subscriber.recv()
    assert message.topic == "sensors/8/temperature"
    assert message.payload == b"21.5"

    # A `#` covers the levels that remain, so a second link takes the whole subtree,
    # including the reading the single-level filter passed over.
    watcher = broker.link()
    await watcher.connect()
    await watcher.subscribe("sensors/#")
    await publisher.send("sensors/8/temperature/raw", b"2150")

    deep = await watcher.recv()
    assert deep.topic == "sensors/8/temperature/raw"
    assert deep.payload == b"2150"

    # A link that has been disconnected reports the failure instead of dropping the
    # reading, which is the case a test wants to reach without unplugging anything.
    await publisher.disconnect()
    try:
        await publisher.send("sensors/8/temperature", b"21.6")
    except PamojaError:
        pass
    else:
        raise AssertionError("a disconnected link should refuse to publish")


asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LoopbackGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/LoopbackGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LoopbackGuide.cs):

```csharp
// One broker and two links off it, all in this process. Nothing binds a port
// and nothing has to be running for the traffic below to flow.
using var broker = new LoopbackBroker();
using LoopbackTransport publisher = broker.Link();
using LoopbackTransport subscriber = broker.Link();
await publisher.ConnectAsync();
await subscriber.ConnectAsync();

// A `+` stands for exactly one level, so the deeper topic is not delivered
// here and the first message this subscriber sees is the second publish.
await subscriber.SubscribeAsync("line/+/temp");
await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());
await publisher.SendAsync("line/mixer/temp", "21.5"u8.ToArray());

TransportMessage? message = await subscriber.ReceiveAsync();
Expect(message?.Topic == "line/mixer/temp", "the topic survives the trip");
Expect(
    message!.Payload.AsSpan().SequenceEqual("21.5"u8),
    "and so does the reading");

// A `#` covers the levels that remain, so a second link takes the whole
// subtree, including the reading the single-level filter passed over.
using LoopbackTransport watcher = broker.Link();
await watcher.ConnectAsync();
await watcher.SubscribeAsync("line/#");
await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());

TransportMessage? deep = await watcher.ReceiveAsync();
Expect(deep?.Topic == "line/mixer/temp/raw", "the deeper topic arrives here");
Expect(deep!.Payload.AsSpan().SequenceEqual("2150"u8), "with its own payload");

// A link that has been disconnected reports the failure instead of dropping
// the reading, which is the case a test wants to reach without unplugging.
await publisher.DisconnectAsync();
bool refused = false;
try
{
    await publisher.SendAsync("line/mixer/temp", "21.6"u8.ToArray());
}
catch (PamojaException)
{
    refused = true;
}
Expect(refused, "a disconnected link refuses to publish");
```
<!-- end -->

## Reference

<!-- table: reference loopback -->
- Rust: [`pamoja-loopback`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html)
- TypeScript: [`@pamoja/loopback`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html)
- Python: [`pamoja.loopback`](https://pamoja.molex.cloud/docs/reference/python/pamoja/loopback.html)
- C#: [`Pamoja.Loopback`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html)
<!-- end -->
