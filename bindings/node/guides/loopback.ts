// The in-process broker guide example; see docs/guides/loopback.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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

  // A `+` stands for exactly one level, so this takes the node's temperature but not the
  // raw reading a level below it.
  await subscriber.subscribe('sensors/+/temperature')
  await publisher.send('sensors/8/temperature/raw', Buffer.from('2150'))
  await publisher.send('sensors/8/temperature', Buffer.from('21.5'))

  const message = (await subscriber.recv())!
  console.log(`sensors/+/temperature took ${message.payload.toString()} from ${message.topic}`)

  // A `#` covers every level that remains, so a second link takes the whole subtree,
  // including the reading the single-level filter passed over.
  const watcher = broker.link()
  await watcher.connect()
  await watcher.subscribe('sensors/#')
  await publisher.send('sensors/8/temperature/raw', Buffer.from('2150'))

  const deep = (await watcher.recv())!
  console.log(`sensors/#             took ${deep.payload.toString()} from ${deep.topic}`)

  // A link that has been disconnected reports the failure instead of dropping the reading,
  // which is the case a test wants to reach without unplugging anything.
  await publisher.disconnect()
  try {
    await publisher.send('sensors/8/temperature', Buffer.from('21.6'))
    console.log('a disconnected link took a reading, which should never happen')
  } catch (error) {
    console.log(`disconnected refused the reading: ${(error as Error).message}`)
  }

  return { message, deep }
}

main()
// ANCHOR_END: example
  .then(check)

function check({ message, deep }: { message: { topic: string; payload: Buffer }; deep: { topic: string; payload: Buffer } }): void {
  assert.equal(message.topic, 'sensors/8/temperature')
  assert.equal(message.payload.toString(), '21.5')
  assert.equal(deep.topic, 'sensors/8/temperature/raw')
  assert.equal(deep.payload.toString(), '2150')
}
