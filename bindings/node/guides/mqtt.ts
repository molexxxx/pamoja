// The MQTT guide example; see docs/guides/mqtt.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
// ANCHOR_END: example
  .then(check)

function check(received: { topic: string; payload: Buffer }): void {
  assert.equal(received.topic, 'sensors/1/temperature')
  assert.equal(received.payload.toString(), '21.5')
}
