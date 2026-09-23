# MQTT

MQTT is how most gateways reach whatever is upstream of them: one long-lived TCP
connection to a broker, carrying every publish and every subscription. pamoja's
client is that connection behind the same transport surface the rest of the
framework uses, so a node publishes and subscribes without knowing which link it
got. Once connected the transport owns a background task that answers keep-alive
pings, completes delivery handshakes, and queues inbound messages for the caller
to drain.

The three delivery guarantees are the knob that matters. At most once hands the
message to the network and forgets it, at least once is acknowledged and may
arrive twice, and exactly once costs a four-step handshake. The client carries one
default and applies it to what it publishes and what it subscribes to.

A connection that cannot be made is an ordinary outcome with a defined result,
not a client left looking connected, so a retry loop has something to test. When
messages need to flow with no broker installed at all, the in-process transport
in the [Loopback](loopback.md) guide implements the same topic and wildcard rules
and delivers between clients in the same process.

## What the example does

It runs a site's telemetry path over a broker: a gateway subscribes to every
node's temperature with a single-level wildcard, a node publishes a reading under
that pattern, and the gateway reads it back. The node then tries to send two days
of readings as one message, which is over the connection's packet limit, and stays
connected when that is refused. Last, the node disconnects, and a client aimed at
a port with nothing listening on it is refused.

The Rust example starts an in-process broker on whatever spare port the machine
hands out, which is where the `port` in the snippet comes from, so it needs
nothing running; its `connect` helper retries while that broker starts. The
binding examples talk to a broker on localhost, which CI starts and `just broker`
starts locally. The client that gets refused aims at port 1, where nothing
listens.

It proves:

- A subscription with a `+` in it takes a reading published under a concrete
  name, so a gateway follows every node's temperature without naming one.
- What arrives is the topic the node published to, `sensors/1/temperature`,
  rather than the `sensors/+/temperature` filter that matched it, and the payload
  is the bytes the node sent.
- A message over the packet limit is refused before anything is sent, with the
  size its packet would have been, and the node is still connected afterwards.
- A client that has disconnected reports itself not connected, so code deciding
  whether to reconnect is not reading a stale flag.
- A broker that is not there fails the connect and leaves the client not
  connected, which is what a retry loop tests.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example mqtt" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example mqtt</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- mqtt" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- mqtt</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/mqtt.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/mqtt.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- mqtt" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- mqtt</code></div>
</div>
<!-- end -->

## Rust

In Rust, `MqttConfig::new(client_id, host, port)` holds the settings, refined with
`keep_alive`, `capacity`, `qos`, and `max_packet_size`, and `MqttTransport::new(config)` makes
the client. It implements the `Transport` and `Receive` traits, so `connect`, `subscribe`,
`send`, `send_text`, and `recv` are the calls every link keeps, and it adds `is_connected()` and
`disconnect()`. Every call is async and fails with a `pamoja_core::Error`: `Transport` for a
broker that cannot be reached, a topic MQTT does not allow, a message over the packet limit, or a
connection that ended on its own, and `Closed` for a client that is not connected.

<!-- snippet: examples/guides/mqtt.rs#example -->
From [`examples/guides/mqtt.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/mqtt.rs):

```rust
use pamoja_core::{Receive, Transport};
use pamoja_mqtt::{MqttConfig, MqttTransport, QualityOfService};

let connection = |connected: bool| {
    if connected {
        "still connected"
    } else {
        "not connected"
    }
};

// The gateway takes every temperature on the site. A `+` stands for exactly one level,
// so this matches every node's temperature and nothing deeper.
let gateway_config = MqttConfig::new("site-gateway", "127.0.0.1", port)
    .keep_alive(Duration::from_secs(5))
    .qos(QualityOfService::AtLeastOnce);
let mut gateway = connect(gateway_config).await;
gateway
    .subscribe("sensors/+/temperature")
    .await
    .expect("the broker accepts the subscription");
println!("gateway   subscribed to sensors/+/temperature");

// A node publishes under that pattern. At least once has the broker acknowledge each
// message, where at most once would send it and forget it.
let node_config = MqttConfig::new("node-1", "127.0.0.1", port)
    .keep_alive(Duration::from_secs(5))
    .qos(QualityOfService::AtLeastOnce);
let mut node = connect(node_config).await;
node.send_text("sensors/1/temperature", "21.5")
    .await
    .expect("the broker takes the reading");
println!("node      published 21.5 to sensors/1/temperature");

// The gateway receives it with the topic attached, which is how it knows which node
// sent the reading without the payload having to repeat it.
let received = gateway
    .recv()
    .await
    .expect("the link is up")
    .expect("a message arrives");
let reading = received.text().expect("text");
let topic = &received.topic;
println!("gateway   got {reading} on {topic}");

// Two days of readings saved at one a minute, sent as one message, make a packet over
// the connection's 10 KiB limit. The send is refused before anything leaves and the
// connection stays up; a node that must send it raises the limit on every client that
// shares the topic, or splits it.
let backlog = vec!["21.5"; 2 * 24 * 60].join(",");
match node.send_text("sensors/1/backlog", &backlog).await {
    Ok(()) => println!("node      sent an oversized backlog, which should never happen"),
    Err(error) => println!("node      backlog refused: {error}"),
}
let after_refusal = node.is_connected();
println!("node      {}", connection(after_refusal));

// Disconnecting leaves the transport reusable, so a node that loses its link can
// reconnect the same object when the broker comes back.
node.disconnect().await.expect("a clean disconnect");
let after_disconnect = node.is_connected();
println!(
    "node      {} after disconnecting",
    connection(after_disconnect)
);

// A broker that is not there is reported rather than leaving a client that looks
// connected, so a retry loop has something to test. Nothing listens on port 1.
let mut nowhere = MqttTransport::new(
    MqttConfig::new("node-2", "127.0.0.1", 1).qos(QualityOfService::ExactlyOnce),
);
match nowhere.connect().await {
    Ok(()) => {
        println!("an unreachable broker accepted a connection, which should never happen")
    }
    Err(error) => println!("unreachable broker refused: {error}"),
}
```
<!-- end -->

## TypeScript

In TypeScript, `new MqttClient(options)` from `@pamoja/mqtt` takes `{ clientId, host, port,
keepAliveSecs?, capacity?, qos?, maxPacketSize? }`, with `Qos.AtMostOnce`, `Qos.AtLeastOnce`, and
`Qos.ExactlyOnce`. `connect`, `subscribe`, `publish(topic, textOrBytes)`, `recv(timeoutMs?)`,
`isConnected`, and `disconnect` return promises, and `for await (const message of client)` reads
until the connection ends. A message is `{ topic, payload, text?, number? }`, with `payload` a
`Buffer`. For a ladder, `Transport.mqtt(options)` in `@pamoja/core` takes the same options.

<!-- snippet: bindings/node/guides/mqtt.ts#example -->
From [`bindings/node/guides/mqtt.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mqtt.ts):

```typescript
import { MqttClient, Qos } from '@pamoja/mqtt'

// The broker on the site. The guide's CI runs one on localhost; point these at yours and
// nothing else changes.
const BROKER = '127.0.0.1'
const PORT = 1883

const connection = (connected: boolean) => (connected ? 'still connected' : 'not connected')

async function main() {
  // The gateway takes every temperature on the site. A `+` stands for exactly one level,
  // so this matches every node's temperature and nothing deeper.
  const gateway = new MqttClient({
    clientId: 'site-gateway',
    host: BROKER,
    port: PORT,
    qos: Qos.AtLeastOnce,
  })
  await gateway.connect()
  await gateway.subscribe('sensors/+/temperature')
  console.log('gateway   subscribed to sensors/+/temperature')

  // A node publishes under that pattern. At least once has the broker acknowledge each
  // message, where at most once would send it and forget it.
  const node = new MqttClient({
    clientId: 'node-1',
    host: BROKER,
    port: PORT,
    qos: Qos.AtLeastOnce,
  })
  await node.connect()
  await node.publish('sensors/1/temperature', '21.5')
  console.log('node      published 21.5 to sensors/1/temperature')

  // The gateway receives it with the topic attached, which is how it knows which node
  // sent the reading without the payload having to repeat it.
  const received = (await gateway.recv())!
  console.log(`gateway   got ${received.text!} on ${received.topic}`)

  // Two days of readings saved at one a minute, sent as one message, make a packet over
  // the connection's 10 KiB limit. The send is refused before anything leaves and the
  // connection stays up; a node that must send it raises the limit on every client that
  // shares the topic, or splits it.
  const backlog = Array(2 * 24 * 60).fill('21.5').join(',')
  try {
    await node.publish('sensors/1/backlog', backlog)
    console.log('node      sent an oversized backlog, which should never happen')
  } catch (error) {
    console.log(`node      backlog refused: ${(error as Error).message}`)
  }
  const afterRefusal = await node.isConnected()
  console.log(`node      ${connection(afterRefusal)}`)

  // Disconnecting leaves the client reusable, so a node that loses its link can reconnect
  // the same object when the broker comes back.
  await node.disconnect()
  const afterDisconnect = await node.isConnected()
  console.log(`node      ${connection(afterDisconnect)} after disconnecting`)
  await gateway.disconnect()

  // A broker that is not there is reported rather than leaving a client that looks
  // connected, so a retry loop has something to test.
  const nowhere = new MqttClient({ clientId: 'node-2', host: BROKER, port: 1, keepAliveSecs: 1 })
  try {
    await nowhere.connect()
    console.log('an unreachable broker accepted a connection, which should never happen')
  } catch (error) {
    console.log(`unreachable broker refused: ${(error as Error).message}`)
  }

  return { received, afterRefusal, afterDisconnect }
}

main()
```
<!-- end -->

## Python

In Python, `MqttClient(client_id=..., host=..., port=...)` from `pamoja.mqtt` takes the settings
as keywords, with `keep_alive_secs`, `capacity`, `qos`, and `max_packet_size` optional and
`Qos.AT_MOST_ONCE`, `Qos.AT_LEAST_ONCE`, and `Qos.EXACTLY_ONCE` as the guarantees. The calls are
coroutines, `async for message in client` reads until the connection ends, and `async with`
connects and disconnects around a block. A message has `topic`, `payload` as bytes, `text`, and
`number`. `asyncio.wait_for` puts a limit on a receive, and a failure raises `PamojaError`. For a
ladder, `Transport.mqtt(...)` in `pamoja.core` takes the same keywords.

<!-- snippet: bindings/python/guides/mqtt.py#example -->
From [`bindings/python/guides/mqtt.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mqtt.py):

```python
import asyncio

from pamoja.core import PamojaError
from pamoja.mqtt import MqttClient, Qos

# The broker on the site. The guide's CI runs one on localhost; point these at yours and
# nothing else changes.
BROKER = "127.0.0.1"
PORT = 1883


def connection(connected: bool) -> str:
    return "still connected" if connected else "not connected"


async def main() -> None:
    # The gateway takes every temperature on the site. A `+` stands for exactly one level,
    # so this matches every node's temperature and nothing deeper.
    gateway = MqttClient(
        client_id="site-gateway", host=BROKER, port=PORT, qos=Qos.AT_LEAST_ONCE
    )
    await gateway.connect()
    await gateway.subscribe("sensors/+/temperature")
    print("gateway   subscribed to sensors/+/temperature")

    # A node publishes under that pattern. At least once has the broker acknowledge each
    # message, where at most once would send it and forget it.
    node = MqttClient(client_id="node-1", host=BROKER, port=PORT, qos=Qos.AT_LEAST_ONCE)
    await node.connect()
    await node.publish("sensors/1/temperature", "21.5")
    print("node      published 21.5 to sensors/1/temperature")

    # The gateway receives it with the topic attached, which is how it knows which node
    # sent the reading without the payload having to repeat it.
    received = await gateway.recv()
    print(f"gateway   got {received.text} on {received.topic}")

    # Two days of readings saved at one a minute, sent as one message, make a packet over
    # the connection's 10 KiB limit. The send is refused before anything leaves and the
    # connection stays up; a node that must send it raises the limit on every client that
    # shares the topic, or splits it.
    backlog = ",".join(["21.5"] * (2 * 24 * 60))
    try:
        await node.publish("sensors/1/backlog", backlog)
        print("node      sent an oversized backlog, which should never happen")
    except PamojaError as error:
        print(f"node      backlog refused: {error}")
    after_refusal = await node.is_connected()
    print(f"node      {connection(after_refusal)}")

    # Disconnecting leaves the client reusable, so a node that loses its link can
    # reconnect the same object when the broker comes back.
    await node.disconnect()
    after_disconnect = await node.is_connected()
    print(f"node      {connection(after_disconnect)} after disconnecting")
    await gateway.disconnect()

    # A broker that is not there is reported rather than leaving a client that looks
    # connected, so a retry loop has something to test.
    nowhere = MqttClient(client_id="node-2", host=BROKER, port=1, keep_alive_secs=1)
    try:
        await nowhere.connect()
        print("an unreachable broker accepted a connection, which should never happen")
    except PamojaError as error:
        print(f"unreachable broker refused: {error}")

    return received, after_refusal, after_disconnect


received, after_refusal, after_disconnect = asyncio.run(main())
```
<!-- end -->

## C#

In C#, `new MqttClient(new MqttClientOptions { ... })` from `Pamoja.Mqtt` takes `ClientId`,
`Host`, and `Port`, with `KeepAliveSecs`, `Capacity`, `Qos`, and `MaxPacketSize` optional. The
client is `IAsyncDisposable`, with `ConnectAsync`, `SubscribeAsync`,
`PublishAsync(topic, textOrBytes)`, `RecvAsync()`, `RecvAsync(limit)`, `IsConnectedAsync`, and
`DisconnectAsync`, and calls on one client run one at a time. `await foreach` reads until the
connection ends, can publish from its body, and stops within a quarter of a second of being
canceled. A message has `Topic`, `Payload` as `ReadOnlyMemory<byte>`, `Text`, and `Number`. For a
ladder, `MqttTransport.Open(options)` takes the same options.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs):

```csharp
// The broker on the site. The guide's CI runs one on localhost; point these at
// yours and nothing else changes.
const string Broker = "127.0.0.1";
const ushort Port = 1883;

static string Connection(bool connected) =>
    connected ? "still connected" : "not connected";

// The gateway takes every temperature on the site. A `+` stands for exactly one
// level, so this matches every node's temperature and nothing deeper.
await using var gateway = new MqttClient(new MqttClientOptions
{
    ClientId = "site-gateway",
    Host = Broker,
    Port = Port,
    Qos = Qos.AtLeastOnce,
});
await gateway.ConnectAsync();
await gateway.SubscribeAsync("sensors/+/temperature");
Console.WriteLine("gateway   subscribed to sensors/+/temperature");

// A node publishes under that pattern. At least once has the broker acknowledge
// each message, where at most once would send it and forget it.
await using var node = new MqttClient(new MqttClientOptions
{
    ClientId = "node-1",
    Host = Broker,
    Port = Port,
    Qos = Qos.AtLeastOnce,
});
await node.ConnectAsync();
await node.PublishAsync("sensors/1/temperature", "21.5");
Console.WriteLine("node      published 21.5 to sensors/1/temperature");

// The gateway receives it with the topic attached, which is how it knows which
// node sent the reading without the payload having to repeat it.
MqttMessage received = (await gateway.RecvAsync())!;
Console.WriteLine(
    $"gateway   got {received.Text}"
    + $" on {received.Topic}");

// Two days of readings saved at one a minute, sent as one message, make a packet
// over the connection's 10 KiB limit. The send is refused before anything leaves
// and the connection stays up; a node that must send it raises the limit on every
// client that shares the topic, or splits it.
string backlog = string.Join(",", Enumerable.Repeat("21.5", 2 * 24 * 60));
try
{
    await node.PublishAsync("sensors/1/backlog", backlog);
    Console.WriteLine("node      sent an oversized backlog, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"node      backlog refused: {error.Message}");
}

bool afterRefusal = await node.IsConnectedAsync();
Console.WriteLine($"node      {Connection(afterRefusal)}");

// Disconnecting leaves the client reusable, so a node that loses its link can
// reconnect the same object when the broker comes back.
await node.DisconnectAsync();
bool afterDisconnect = await node.IsConnectedAsync();
Console.WriteLine($"node      {Connection(afterDisconnect)} after disconnecting");

// A broker that is not there is reported rather than leaving a client that looks
// connected, so a retry loop has something to test.
await using var nowhere = new MqttClient(new MqttClientOptions
{
    ClientId = "node-2",
    Host = Broker,
    Port = 1,
    KeepAliveSecs = 1,
});
try
{
    await nowhere.ConnectAsync();
    Console.WriteLine("an unreachable broker accepted a connection, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"unreachable broker refused: {error.Message}");
}
```
<!-- end -->

## Values at a glance

**The settings,** each with its default:

| Setting | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| client id, host, and port, required | `MqttConfig::new(id, host, port)` | `clientId`, `host`, `port` | `client_id`, `host`, `port` | `ClientId`, `Host`, `Port` |
| keep-alive, 30 seconds | `.keep_alive(duration)` | `keepAliveSecs` | `keep_alive_secs` | `KeepAliveSecs` |
| requests queued toward the broker, 64 | `.capacity(n)` | `capacity` | `capacity` | `Capacity` |
| delivery guarantee, at least once | `.qos(QualityOfService::AtLeastOnce)` | `qos: Qos.AtLeastOnce` | `qos=Qos.AT_LEAST_ONCE` | `Qos = Qos.AtLeastOnce` |
| packet limit each way, 10,240 bytes | `.max_packet_size(bytes)` | `maxPacketSize` | `max_packet_size` | `MaxPacketSize` |

**The calls:**

| Call | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| connect | `link.connect().await?` | `await client.connect()` | `await client.connect()` | `await client.ConnectAsync()` |
| subscribe | `link.subscribe(filter).await?` | `await client.subscribe(filter)` | `await client.subscribe(filter)` | `await client.SubscribeAsync(filter)` |
| publish | `link.send(topic, &bytes)`, `link.send_text(topic, text)` | `await client.publish(topic, textOrBytes)` | `await client.publish(topic, text_or_bytes)` | `await client.PublishAsync(topic, textOrBytes)` |
| receive | `link.recv().await?` | `await client.recv()` | `await client.recv()` | `await client.RecvAsync()` |
| receive with a limit | `timeout(limit, link.recv()).await` | `await client.recv(ms)` | `await asyncio.wait_for(client.recv(), seconds)` | `await client.RecvAsync(limit)` |
| read until it ends | `while let Some(m) = link.recv().await? {}` | `for await (const m of client)` | `async for m in client` | `await foreach (var m in client)` |
| is it connected | `link.is_connected()` | `await client.isConnected()` | `await client.is_connected()` | `await client.IsConnectedAsync()` |
| disconnect | `link.disconnect().await?` | `await client.disconnect()` | `await client.disconnect()` | `await client.DisconnectAsync()` |

**The delivery guarantees.** The client speaks MQTT 3.1.1, and the section numbers here and
below are those of the OASIS standard:

| Guarantee | On the wire | The message arrives |
| --- | --- | --- |
| at most once | PUBLISH (4.3.1) | once, or not at all |
| at least once | PUBLISH, then PUBACK (4.3.2) | at least once, so possibly twice |
| exactly once | PUBLISH, PUBREC, PUBREL, then PUBCOMP (4.3.3) | once |

| Guarantee | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| at most once | `QualityOfService::AtMostOnce` | `Qos.AtMostOnce` | `Qos.AT_MOST_ONCE` | `Qos.AtMostOnce` |
| at least once | `QualityOfService::AtLeastOnce` | `Qos.AtLeastOnce` | `Qos.AT_LEAST_ONCE` | `Qos.AtLeastOnce` |
| exactly once | `QualityOfService::ExactlyOnce` | `Qos.ExactlyOnce` | `Qos.EXACTLY_ONCE` | `Qos.ExactlyOnce` |

A subscriber receives a message at the lower of the guarantee it was published with and the one
granted to its subscription (3.8.4). A send returns once the message is queued for the broker,
before any acknowledgment. The session is clean, so a message still waiting for its
acknowledgment when the connection drops is not sent again (4.4).

**What the client checks before it sends.** A topic to publish to holds no `+` or `#`
(4.7.1). A filter has `#` only alone in its last level and `+` only as a whole level
(4.7.1.2 and 4.7.1.3). Both are one to 65,535 bytes with no null character (4.7.3). A packet is
the topic and the payload plus up to nine bytes of framing, and one over the limit is refused
before it leaves:

| Payload | Topic | At most once | At least once, exactly once |
| --- | --- | --- | --- |
| 100 bytes | `sensors/1/temperature`, 21 bytes | 125-byte packet | 127-byte packet |
| 10,000 bytes | `sensors/1/temperature` | 10,026-byte packet | 10,028-byte packet |
| 10,215 bytes | `sensors/1/temperature` | 10,241 bytes, over the default limit | 10,243 bytes, over the default limit |

**What the broker decides:**

| When | What happens | Section |
| --- | --- | --- |
| another client connects with the same id | the broker ends the earlier connection | 3.1.4 |
| a connection ends | with a clean session, its subscriptions end with it | 3.1.2.4 |
| a subscription is placed | the broker sends the last retained message on each matching topic | 3.3.1.3 |
| a client sends nothing for one and a half keep-alive periods | the broker ends the connection; the client pings so that it does not | 3.1.2.10 |
| a publish matches two of a client's filters | at least one copy arrives, and the broker may send one per filter | 3.3.5 |
| a filter starts with a wildcard | it never matches a topic that starts with `$` | 4.7.2 |

pamoja publishes without the retain flag, so a retained message a subscriber receives came from
another client.

## When it goes wrong

What the client refuses, and what it says:

| What happened | The message | What to check |
| --- | --- | --- |
| nothing answers at the address | `transport error: I/O: ...`, then the operating system's words for it | the host, the port, and that the broker runs |
| a topic to publish to holds a wildcard | `transport error: the topic sensors/+/temperature holds a wildcard, which only a subscription may use` | publish to the concrete topic |
| a filter puts a wildcard in the wrong place | `transport error: the filter sensors+ places a wildcard where MQTT does not allow one: ...` | `#` alone in the last level, `+` as a whole level |
| a message is over the packet limit | `transport error: the message makes a 14423-byte packet, over this connection's 10240-byte limit` | raise the limit on every client that shares the topic, or split the message |
| the connection ended on its own | `transport error: the connection to the broker ended: ...`, from one receive | the reason after the colon, then connect and subscribe again |
| a call on a client that is not connected | `resource is closed` | connect first, or again once the connection has ended |
| a receive given a limit ran out of time | `no message arrived within 250 ms` in TypeScript and C# | the topics were quiet; the next message waits for the next receive |

The mistakes that cost an afternoon:

- **Two devices keep dropping each other.** They share a client id, and the broker ends the older
  connection whenever the other connects (3.1.4). The dropped client's receive reports it, as
  `the connection to the broker ended: Mqtt state: Connection closed by peer abruptly` from
  Mosquitto. Give every connection its own id, two clients in one program included.
- **Nothing arrives after a reconnect.** The session is clean, so the subscriptions ended with the
  old connection (3.1.2.4). Subscribe again after every `connect`.
- **A subscriber's connection ends when a large message arrives.** Its packet limit is smaller than
  the publisher's, and a packet over the limit ends the connection that receives it. Raise the
  limit on every client that shares the topic.
- **A client that listens never publishes.** A receive holds the client until a message arrives,
  and a publish on the same client waits behind it. Receive with a limit in a loop and publish
  between receives, publish from the body of the loop that reads, or use a second client with its
  own id.
- **A reading is handled twice.** At least once allows a second delivery (4.3.2). Make the handler
  safe to repeat, or ask for exactly once.
- **A new subscriber gets yesterday's reading.** Another client published it with the retain flag,
  and the broker hands the last retained message to each new subscription (3.3.1.3).
- **A quiet connection drops.** Something on the path, often a NAT or a firewall, closed an idle
  TCP session before the next keep-alive ping. Set the keep-alive below its idle timeout.
- **A `#` subscription never sees the broker's `$SYS` topics.** A filter that starts with a
  wildcard never matches a topic that starts with `$` (4.7.2). Subscribe to `$SYS/#` by name.
- **A reading was lost when the connection dropped.** A send returns once the message is queued,
  and a message in flight when a clean-session connection drops is not sent again (4.4). A reading
  that has to arrive needs an answer from whatever receives it.

## Where next

<!-- table: next mqtt -->
- [Codecs](codec.md): CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links.
- [Store and forward](sync.md): Offline-first queues.
- [Transport ladder](ladder.md): Cheapest reachable link first, buffering to a store when every link is down.
- Also in Transports and testing: [CoAP](coap.md), [Loopback](loopback.md), [Event bus](bus.md), [Engine surface](transport.md), [Your own link](link.md), [Simulators](sim.md).
<!-- end -->

## Reference

<!-- table: reference mqtt -->
- Rust: [`pamoja-mqtt`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-mqtt)
- TypeScript: [`@pamoja/mqtt`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-mqtt)
- Python: [`pamoja.mqtt`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-mqtt)
- C#: [`Pamoja.Mqtt`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-mqtt)
<!-- end -->
