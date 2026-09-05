// The transport ladder guide example; see docs/guides/ladder.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Transport } from '@pamoja/core'
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

const TOPIC = 'sensors/1/temperature'

async function main() {
  // Two links off the same node: a near mesh hop and a metered backhaul. Each is a
  // separate broker, so which one carried a reading is visible from its subscriber.
  const mesh = new LoopbackBroker()
  const backhaul = new LoopbackBroker()
  const gateway = backhaul.link()
  await gateway.connect()
  await gateway.subscribe(TOPIC)

  // Rungs are tried in the order they are added, cheapest first. The mesh hop loses every
  // packet here; the backhaul carries one send, then drops the next two.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.degraded(mesh.rung(), 1, 0, 0))
  await ladder.rung(Transport.degraded(backhaul.rung(), 0, 1, 2))
  await ladder.connect()

  // The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
  // broker only that rung publishes to.
  const first = await ladder.send(TOPIC, Buffer.from('21.5'))
  const arrived = (await gateway.recv())!
  console.log(`first reading: ${first}, gateway got ${arrived.payload.toString()}`)

  // Now nothing will take a send, so the next reading is buffered rather than lost.
  const second = await ladder.send(TOPIC, Buffer.from('21.6'))
  const waiting = await ladder.buffered()
  console.log(`second reading: ${second}, ${waiting} waiting in the queue`)

  // A flush while the links are still down forwards nothing and leaves the backlog
  // intact, because a record is removed only once a rung has accepted it.
  const whileDown = await ladder.flush()
  console.log(`flush while down forwarded ${whileDown}, queue still ${await ladder.buffered()}`)

  // The backhaul is reachable again, so the buffered reading goes out exactly once.
  const whenUp = await ladder.flush()
  const late = (await gateway.recv())!
  console.log(`flush when up forwarded ${whenUp}, gateway got ${late.payload.toString()}`)

  return { first, second, waiting, whileDown, whenUp, left: await ladder.buffered(), late }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: {
  first: Delivery
  second: Delivery
  waiting: number
  whileDown: number
  whenUp: number
  left: number
  late: { payload: Buffer }
}): void {
  assert.equal(seen.first, Delivery.Sent)
  assert.equal(seen.second, Delivery.Buffered)
  assert.equal(seen.waiting, 1)
  assert.equal(seen.whileDown, 0)
  assert.equal(seen.whenUp, 1)
  assert.equal(seen.left, 0)
  assert.equal(seen.late.payload.toString(), '21.6')
}
