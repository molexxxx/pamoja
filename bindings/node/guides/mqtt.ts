// The MQTT guide example; see docs/guides/mqtt.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
// ANCHOR_END: example
  .then(check)

function check({
  received,
  afterRefusal,
  afterDisconnect,
}: {
  received: { topic: string; text?: string }
  afterRefusal: boolean
  afterDisconnect: boolean
}): void {
  assert.equal(received.topic, 'sensors/1/temperature')
  assert.equal(received.text!, '21.5')
  assert.ok(afterRefusal, 'a refused send leaves the connection up')
  assert.ok(!afterDisconnect, 'a disconnected client says so')
}
