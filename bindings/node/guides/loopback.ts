// The loopback guide example; see docs/guides/loopback.md.

// ANCHOR: example
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
// ANCHOR_END: example
