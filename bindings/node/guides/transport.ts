// The engine surface guide example; see docs/guides/transport.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Transport } from '@pamoja/core'
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

const TOPIC = 'sensors/1/temperature'

async function main() {
  // Whatever a link is underneath, MQTT, CoAP, or the in-process broker here, it reaches
  // the rest of the framework through one contract. Anything that takes a link works with
  // any of them, so a node is written once and pointed at whichever link it has.
  const broker = new LoopbackBroker()
  const gateway = broker.link()
  await gateway.connect()
  await gateway.subscribe(TOPIC)

  // The fault injector is itself a link wrapping a link, so it composes anywhere one does.
  // This one fails its next send and passes the rest through.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.faulty(broker.rung(), 1))
  await ladder.connect()

  // The injected failure lands, so the reading is buffered rather than lost.
  const first = await ladder.send(TOPIC, Buffer.from('20.1'))
  console.log(`first reading: ${first}, ${await ladder.buffered()} queued`)

  // The next reading joins the back of the queue instead of overtaking it, even though the
  // link would take it now. Order on the wire is the order the readings were taken.
  const second = await ladder.send(TOPIC, Buffer.from('20.4'))
  const queued = await ladder.buffered()
  console.log(`second reading: ${second}, ${queued} queued`)

  // Flushing forwards the backlog oldest first, and the subscriber sees it in order.
  const forwarded = await ladder.flush()
  const earlier = (await gateway.recv())!.payload.toString()
  const later = (await gateway.recv())!.payload.toString()
  console.log(`flush forwarded ${forwarded}, gateway saw ${earlier} then ${later}`)

  return { first, second, queued, forwarded, left: await ladder.buffered(), earlier, later }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: {
  first: Delivery
  second: Delivery
  queued: number
  forwarded: number
  left: number
  earlier: string
  later: string
}): void {
  assert.equal(seen.first, Delivery.Buffered)
  assert.equal(seen.second, Delivery.Buffered)
  assert.equal(seen.queued, 2)
  assert.equal(seen.forwarded, 2)
  assert.equal(seen.left, 0)
  assert.equal(seen.earlier, '20.1')
  assert.equal(seen.later, '20.4')
}
