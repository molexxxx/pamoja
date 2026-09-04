// The MQTT guide example; see docs/guides/mqtt.md.

// ANCHOR: example
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
// ANCHOR_END: example
