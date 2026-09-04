// The transport ladder guide example; see docs/guides/ladder.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { Transport } from '@pamoja/core'
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

async function main() {
  // Two links off the same node: a near mesh hop and a metered backhaul. Each is a
  // separate broker, so which one carried a reading is visible from its subscriber.
  const mesh = new LoopbackBroker()
  const backhaul = new LoopbackBroker()
  const gateway = backhaul.link()
  await gateway.connect()
  await gateway.subscribe('sensors/1/temperature')

  // Rungs are tried in the order they are added, cheapest first. The mesh hop loses every
  // packet here; the backhaul carries one send, then drops the next two.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.degraded(mesh.rung(), 1, 0, 0))
  await ladder.rung(Transport.degraded(backhaul.rung(), 0, 1, 2))
  await ladder.connect()

  // The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
  // broker only that rung publishes to.
  const topic = 'sensors/1/temperature'
  assert.equal(await ladder.send(topic, Buffer.from('21.5')), Delivery.Sent)
  assert.equal((await gateway.recv())?.payload.toString(), '21.5')

  // Now nothing will take a send, so the next reading is buffered rather than lost.
  assert.equal(await ladder.send(topic, Buffer.from('21.6')), Delivery.Buffered)
  assert.equal(await ladder.buffered(), 1)

  // A flush while the links are still down forwards nothing and leaves the backlog
  // intact, because a record is removed only once a rung has accepted it.
  assert.equal(await ladder.flush(), 0)
  assert.equal(await ladder.buffered(), 1)

  // The backhaul is reachable again, so the buffered reading goes out exactly once.
  assert.equal(await ladder.flush(), 1)
  assert.equal(await ladder.buffered(), 0)
  assert.equal((await gateway.recv())?.payload.toString(), '21.6')
}

main()
// ANCHOR_END: example
