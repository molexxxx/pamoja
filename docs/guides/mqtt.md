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

None of this needs a broker to exercise. A refused connection is an ordinary
outcome with a defined result, which is what the example below checks. When you
want messages to actually flow with nothing installed, the in-process transport in
the [Loopback](loopback.md) guide implements the topic and wildcard rules and
delivers between clients in the same process.

## What the example does

It checks the delivery guarantees against the levels the protocol numbers, then
configures a client for a broker on a port nothing is listening on and connects.
The connection is refused, and the example checks what the binding does with that.

It proves:

- The three delivery guarantees are the protocol's, in the order MQTT numbers them
  0, 1 and 2.
- Constructing a client reaches no network: it reports itself disconnected before
  any call is made.
- An unreachable broker raises the binding's own error type rather than handing
  back a client that looks connected.
- A failed connect leaves the client disconnected, so the same object can be
  retried once the broker is back.

## Rust

<!-- snippet: examples/tests/guides/mqtt.rs#example -->
From [`examples/tests/guides/mqtt.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/mqtt.rs):

```rust
use std::time::Duration;

use pamoja_core::{Error, Transport};
use pamoja_mqtt::{MqttConfig, MqttTransport, QualityOfService};

// MQTT numbers its three delivery guarantees 0, 1 and 2 on the wire.
assert_eq!(QualityOfService::AtMostOnce as u8, 0);
assert_eq!(QualityOfService::AtLeastOnce as u8, 1);
assert_eq!(QualityOfService::ExactlyOnce as u8, 2);

// Nothing listens on this port, so the broker is unreachable. Building the transport
// touches nothing; only connecting does.
let config = MqttConfig::new("guide-node", "127.0.0.1", 47811)
    .keep_alive(Duration::from_secs(1))
    .qos(QualityOfService::ExactlyOnce);
let mut transport = MqttTransport::new(config);
assert!(!transport.is_connected());

// A refused connection surfaces as a transport error and leaves the transport as it was,
// so the same object can be retried once the broker is back.
let outcome = transport.connect().await;
assert!(matches!(outcome, Err(Error::Transport(_))));
assert!(!transport.is_connected());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/mqtt.ts#example -->
From [`bindings/node/guides/mqtt.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mqtt.ts):

```typescript
import assert from 'node:assert/strict'

import { MqttClient, Qos } from '@pamoja/mqtt'

async function main() {
  // MQTT numbers its three delivery guarantees 0, 1 and 2 on the wire; the binding names
  // them, in that order.
  assert.deepEqual(Object.values(Qos), ['AtMostOnce', 'AtLeastOnce', 'ExactlyOnce'])

  // Nothing listens on this port, so the broker is unreachable. Constructing the client
  // touches nothing; only connecting does.
  const client = new MqttClient({
    clientId: 'guide-node',
    host: '127.0.0.1',
    port: 47811,
    keepAliveSecs: 1,
    qos: Qos.ExactlyOnce,
  })
  assert.equal(await client.isConnected(), false)

  // A refused connection surfaces as a transport error and leaves the client as it was, so
  // the same object can be retried once the broker is back.
  await assert.rejects(() => client.connect(), /transport error/)
  assert.equal(await client.isConnected(), false)
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


async def main() -> None:
    # MQTT numbers its three delivery guarantees 0, 1 and 2 on the wire; the binding
    # names them, in that order.
    assert [level.value for level in Qos] == ["AtMostOnce", "AtLeastOnce", "ExactlyOnce"]

    # Nothing listens on this port, so the broker is unreachable. Constructing the client
    # touches nothing; only connecting does.
    client = MqttClient(
        client_id="guide-node",
        host="127.0.0.1",
        port=47811,
        keep_alive_secs=1,
        qos=Qos.EXACTLY_ONCE,
    )
    assert await client.is_connected() is False

    # A refused connection surfaces as a transport error and leaves the client as it was,
    # so the same object can be retried once the broker is back.
    try:
        await client.connect()
    except PamojaError as error:
        assert str(error).startswith("transport error")
    else:
        raise AssertionError("connecting to a closed port should raise")

    assert await client.is_connected() is False


asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs):

```csharp
// MQTT numbers its three delivery guarantees 0, 1 and 2 on the wire.
Expect((int)Qos.AtMostOnce == 0, "at most once is level 0");
Expect((int)Qos.AtLeastOnce == 1, "at least once is level 1");
Expect((int)Qos.ExactlyOnce == 2, "exactly once is level 2");

// Nothing listens on this port, so the broker is unreachable. Constructing the
// client touches nothing; only connecting does.
await using var client = new MqttClient(new MqttClientOptions
{
    ClientId = "guide-node",
    Host = "127.0.0.1",
    Port = 47811,
    KeepAliveSecs = 1,
    Qos = Qos.ExactlyOnce,
});
Expect(!await client.IsConnectedAsync(), "a fresh client holds no connection");

// A refused connection surfaces as a transport error and leaves the client as it
// was, so the same object can be retried once the broker is back.
bool refused = false;
try
{
    await client.ConnectAsync();
}
catch (PamojaException error)
{
    refused = error.Message.StartsWith("transport error", StringComparison.Ordinal);
}

Expect(refused, "an unreachable broker is reported, not swallowed");
Expect(
    !await client.IsConnectedAsync(),
    "a failed connect leaves the client disconnected");
```
<!-- end -->

## Reference

<!-- table: reference mqtt -->
- Rust: [`pamoja-mqtt`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html)
- TypeScript: [`@pamoja/mqtt`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html)
- Python: [`pamoja.mqtt`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html)
- C#: [`Pamoja.Mqtt`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html)
<!-- end -->
