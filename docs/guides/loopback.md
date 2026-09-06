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
earlier under `/raw`.

A third link joins on the multi-level filter after both of those publishes have
gone out, and takes the next `/raw` reading. The publisher is disconnected
last, and one more send shows what an unusable link does with a reading.

It proves:

- A payload published on one link arrives on another carrying the topic it was
  sent to, with no port bound and no broker process running.
- `+` matches exactly one level, so the filter takes the temperature topic and
  leaves the `/raw` reading a level below it, even though that one went out
  first.
- `#` matches the levels that remain, so the second filter takes the deeper
  topic the single-level one passed over.
- A link can join a broker that has already routed traffic, take a filter of
  its own, and receive a reading published after it connects.
- A disconnected link fails the send rather than accepting a reading it has no
  way to deliver.

## Rust

<!-- snippet: examples/tests/guides/loopback.rs#example -->
From [`examples/tests/guides/loopback.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/loopback.rs):

```rust
use pamoja_core::Transport;
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};

// One broker and two links off it, all in this process. Nothing binds a port and
// nothing has to be running for the traffic below to flow, which is what makes this
// the link to develop a node against before it has a real one.
let broker = LoopbackBroker::new();
let mut publisher = LoopbackTransport::new(broker.clone());
let mut subscriber = LoopbackTransport::new(broker.clone());
publisher.connect().await.expect("the publisher connects");
subscriber.connect().await.expect("the subscriber connects");

// A `+` stands for exactly one level, so this takes the mixer's temperature but not
// the raw reading a level below it.
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
let reading = String::from_utf8_lossy(&message.payload);
println!("line/+/temp took {reading} from {}", message.topic);

// A `#` covers every level that remains, so a second link takes the whole subtree,
// including the reading the single-level filter passed over.
let mut watcher = LoopbackTransport::new(broker);
watcher.connect().await.expect("the watcher connects");
watcher.subscribe("line/#").await.expect("subscribe");
publisher
    .send("line/mixer/temp/raw", b"2150")
    .await
    .expect("send");

let deep = watcher.recv().await.expect("recv").expect("a message");
let raw = String::from_utf8_lossy(&deep.payload);
println!("line/#     took {raw} from {}", deep.topic);

// A link that has been disconnected reports the failure instead of dropping the
// reading, which is the case a test wants to reach without unplugging anything.
publisher.disconnect();
match publisher.send("line/mixer/temp", b"21.6").await {
    Ok(_) => println!("a disconnected link took a reading, which should never happen"),
    Err(error) => println!("disconnected refused the reading: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/loopback.ts#example -->
From [`bindings/node/guides/loopback.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/loopback.ts):

```typescript
import { LoopbackBroker } from '@pamoja/loopback'

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
  await publisher.send('line/mixer/temp/raw', Buffer.from('2150'))
  await publisher.send('line/mixer/temp', Buffer.from('21.5'))

  const message = (await subscriber.recv())!
  console.log(`line/+/temp took ${message.payload.toString()} from ${message.topic}`)

  // A `#` covers every level that remains, so a second link takes the whole subtree,
  // including the reading the single-level filter passed over.
  const watcher = broker.link()
  await watcher.connect()
  await watcher.subscribe('line/#')
  await publisher.send('line/mixer/temp/raw', Buffer.from('2150'))

  const deep = (await watcher.recv())!
  console.log(`line/#     took ${deep.payload.toString()} from ${deep.topic}`)

  // A link that has been disconnected reports the failure instead of dropping the reading,
  // which is the case a test wants to reach without unplugging anything.
  await publisher.disconnect()
  try {
    await publisher.send('line/mixer/temp', Buffer.from('21.6'))
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

<!-- snippet: bindings/python/guides/loopback.py#example -->
From [`bindings/python/guides/loopback.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/loopback.py):

```python
import asyncio

from pamoja.core import PamojaError
from pamoja.loopback import LoopbackBroker


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
    await publisher.send("line/mixer/temp/raw", b"2150")
    await publisher.send("line/mixer/temp", b"21.5")

    message = await subscriber.recv()
    print(f"line/+/temp took {message.payload.decode()} from {message.topic}")

    # A `#` covers every level that remains, so a second link takes the whole subtree,
    # including the reading the single-level filter passed over.
    watcher = broker.link()
    await watcher.connect()
    await watcher.subscribe("line/#")
    await publisher.send("line/mixer/temp/raw", b"2150")

    deep = await watcher.recv()
    print(f"line/#     took {deep.payload.decode()} from {deep.topic}")

    # A link that has been disconnected reports the failure instead of dropping the
    # reading, which is the case a test wants to reach without unplugging anything.
    await publisher.disconnect()
    try:
        await publisher.send("line/mixer/temp", b"21.6")
        print("a disconnected link took a reading, which should never happen")
    except PamojaError as error:
        print(f"disconnected refused the reading: {error}")

    return message, deep


message, deep = asyncio.run(main())
```
<!-- end -->

## C#

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
await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());
await publisher.SendAsync("line/mixer/temp", "21.5"u8.ToArray());

TransportMessage message = (await subscriber.ReceiveAsync())!;
Console.WriteLine(
    $"line/+/temp took {System.Text.Encoding.UTF8.GetString(message.Payload)}"
    + $" from {message.Topic}");

// A `#` covers every level that remains, so a second link takes the whole subtree,
// including the reading the single-level filter passed over.
using LoopbackTransport watcher = broker.Link();
await watcher.ConnectAsync();
await watcher.SubscribeAsync("line/#");
await publisher.SendAsync("line/mixer/temp/raw", "2150"u8.ToArray());

TransportMessage deep = (await watcher.ReceiveAsync())!;
Console.WriteLine(
    $"line/#     took {System.Text.Encoding.UTF8.GetString(deep.Payload)}"
    + $" from {deep.Topic}");

// A link that has been disconnected reports the failure instead of dropping the
// reading, which is the case a test wants to reach without unplugging anything.
await publisher.DisconnectAsync();
try
{
    await publisher.SendAsync("line/mixer/temp", "21.6"u8.ToArray());
    Console.WriteLine("a disconnected link took a reading, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"disconnected refused the reading: {error.Message}");
}
```
<!-- end -->

## Reference

<!-- table: reference loopback -->
- Rust: [`pamoja-loopback`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-loopback)
- TypeScript: [`@pamoja/loopback`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-loopback)
- Python: [`pamoja.loopback`](https://pamoja.molex.cloud/docs/reference/python/pamoja/loopback.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-loopback)
- C#: [`Pamoja.Loopback`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-loopback)
<!-- end -->
