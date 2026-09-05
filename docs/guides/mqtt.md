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
that pattern, and the gateway reads it back. Then the node disconnects, and a
client aimed at a port with nothing listening on it is refused.

The Rust example starts an in-process broker on whatever spare port the machine
hands out, which is where the `port` in the snippet comes from, so it needs
nothing running. The binding examples talk to a broker on localhost, which CI
starts and `just broker` starts locally. The client that gets refused aims at
port 1, where nothing listens.

It proves:

- A subscription with a `+` in it takes a reading published under a concrete
  name, so a gateway follows every node's temperature without naming one.
- What arrives is the topic the node published to, `sensors/1/temperature`,
  rather than the `sensors/+/temperature` filter that matched it, and the payload
  is the bytes the node sent.
- An at-least-once publish returns once the broker has acknowledged it, so a node
  knows its reading was taken rather than hoping.
- A client that has disconnected reports itself disconnected, so code deciding
  whether to reconnect is not reading a stale flag.
- A broker that is not there fails the connect and leaves the client not
  connected, which is what a retry loop tests.

## Rust

<!-- snippet: examples/tests/guides/mqtt.rs#example -->
From [`examples/tests/guides/mqtt.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/mqtt.rs):

```rust
use pamoja_core::Transport;
use pamoja_mqtt::{MqttConfig, MqttTransport, QualityOfService};

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

// A node publishes under that pattern. At-least-once means the broker acknowledges
// the message, so a node knows its reading was taken rather than hoping.
let node_config = MqttConfig::new("node-1", "127.0.0.1", port)
    .keep_alive(Duration::from_secs(5))
    .qos(QualityOfService::AtLeastOnce);
let mut node = connect(node_config).await;
node.send("sensors/1/temperature", b"21.5")
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
let reading = String::from_utf8_lossy(&received.payload);
let topic = &received.topic;
println!("gateway   got {reading} on {topic}");

// Disconnecting leaves the transport reusable, so a node that loses its link can
// reconnect the same object when the broker comes back.
node.disconnect().await.expect("a clean disconnect");
let still_up = node.is_connected();
println!("node      disconnected, still connected: {still_up}");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/mqtt.ts#example -->
From [`bindings/node/guides/mqtt.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mqtt.ts):

```typescript
import { MqttClient, Qos } from '@pamoja/mqtt'

// The broker on the site. The guide's CI runs one on localhost; point these at yours and
// nothing else changes.
const BROKER = '127.0.0.1'
const PORT = 1883

async function main(): Promise<{ topic: string; payload: Buffer }> {
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

  // A node publishes under that pattern. At-least-once means the broker acknowledges the
  // message, so a node knows its reading was taken rather than hoping.
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
  console.log(`gateway   got ${received.payload.toString()} on ${received.topic}`)

  // Disconnecting leaves the client reusable, so a node that loses its link can reconnect
  // the same object when the broker comes back.
  await node.disconnect()
  console.log(`node      disconnected, still connected: ${await node.isConnected()}`)
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

  return received
}

main()
```
<!-- end -->

## Python

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


async def main() -> None:
    # The gateway takes every temperature on the site. A `+` stands for exactly one level,
    # so this matches every node's temperature and nothing deeper.
    gateway = MqttClient(
        client_id="site-gateway", host=BROKER, port=PORT, qos=Qos.AT_LEAST_ONCE
    )
    await gateway.connect()
    await gateway.subscribe("sensors/+/temperature")
    print("gateway   subscribed to sensors/+/temperature")

    # A node publishes under that pattern. At-least-once means the broker acknowledges the
    # message, so a node knows its reading was taken rather than hoping.
    node = MqttClient(client_id="node-1", host=BROKER, port=PORT, qos=Qos.AT_LEAST_ONCE)
    await node.connect()
    await node.publish("sensors/1/temperature", "21.5")
    print("node      published 21.5 to sensors/1/temperature")

    # The gateway receives it with the topic attached, which is how it knows which node
    # sent the reading without the payload having to repeat it.
    received = await gateway.recv()
    print(f"gateway   got {received.payload.decode()} on {received.topic}")

    # Disconnecting leaves the client reusable, so a node that loses its link can
    # reconnect the same object when the broker comes back.
    await node.disconnect()
    print(f"node      disconnected, still connected: {await node.is_connected()}")
    await gateway.disconnect()

    # A broker that is not there is reported rather than leaving a client that looks
    # connected, so a retry loop has something to test.
    nowhere = MqttClient(client_id="node-2", host=BROKER, port=1, keep_alive_secs=1)
    try:
        await nowhere.connect()
        print("an unreachable broker accepted a connection, which should never happen")
    except PamojaError as error:
        print(f"unreachable broker refused: {error}")

    return received


received = asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs):

```csharp
// The broker on the site. The guide's CI runs one on localhost; point these at
// yours and nothing else changes.
const string Broker = "127.0.0.1";
const ushort Port = 1883;

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

// A node publishes under that pattern. At-least-once means the broker
// acknowledges the message, so a node knows its reading was taken.
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
    $"gateway   got {System.Text.Encoding.UTF8.GetString(received.Payload.Span)}"
    + $" on {received.Topic}");

// Disconnecting leaves the client reusable, so a node that loses its link can
// reconnect the same object when the broker comes back.
await node.DisconnectAsync();
Console.WriteLine($"node      disconnected, still connected: {await node.IsConnectedAsync()}");

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
    Console.WriteLine("an unreachable broker accepted a connection, which cannot be");
}
catch (PamojaException error)
{
    Console.WriteLine($"unreachable broker refused: {error.Message}");
}
```
<!-- end -->

## Reference

<!-- table: reference mqtt -->
- Rust: [`pamoja-mqtt`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html)
- TypeScript: [`@pamoja/mqtt`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html)
- Python: [`pamoja.mqtt`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html)
- C#: [`Pamoja.Mqtt`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html)
<!-- end -->
