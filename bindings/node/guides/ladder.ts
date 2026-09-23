// The transport ladder guide example; see docs/guides/ladder.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

const REPORT = 'vessel/7/report'
const ORDERS = 'vessel/7/orders'
const QUIET_MS = 50

async function main() {
  // Three networks a vessel can reach: the harbor's wifi, the coast's cellular network,
  // and a satellite. Each is a broker with an office ashore listening on it, so which one
  // carried a report is read off that office rather than assumed.
  const harbor = new LoopbackBroker()
  const coast = new LoopbackBroker()
  const sky = new LoopbackBroker()
  const harborOffice = harbor.link()
  const coastOffice = coast.link()
  const skyOffice = sky.link()
  for (const office of [harborOffice, coastOffice, skyOffice]) {
    await office.connect()
    await office.subscribe(REPORT)
  }

  // Rungs go on cheapest first. The satellite only sends, so it goes on as an uplink,
  // which the ladder never subscribes or listens on.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(harbor.rung())
  await ladder.rung(coast.rung())
  await ladder.uplink(sky.rung())
  await ladder.connect()
  await ladder.subscribe(ORDERS)

  // In the harbor, the cheapest link takes the report.
  const first = await ladder.send(REPORT, 'report 1')
  console.log(`harbor    carried ${(await harborOffice.recv())!.text!}`)

  // Past the breakwater the wifi is out of reach and the report falls through to the
  // coast, and further out to the satellite.
  harbor.reachable = false
  await ladder.send(REPORT, 'report 2')
  console.log(`coast     carried ${(await coastOffice.recv())!.text!}, with the harbor out of reach`)
  coast.reachable = false
  await ladder.send(REPORT, 'report 3')
  console.log(`sky       carried ${(await skyOffice.recv())!.text!}, with the coast out of reach too`)

  // In a storm nothing is in reach, and the report waits in the store rather than being
  // lost.
  sky.reachable = false
  const stormy = await ladder.send(REPORT, 'report 4')
  const waiting = await ladder.buffered()
  console.log(`vessel    buffered report 4 with every link out of reach, ${waiting} waiting`)

  // A flush with every link still out of reach forwards nothing, because a record leaves
  // the store only once a link has taken it.
  const idle = await ladder.flush()
  const still = await ladder.buffered()
  console.log(`vessel    flushed ${idle} while every link was out of reach, ${still} still waiting`)

  // Back in reach of the coast, a flush sends the backlog, oldest first.
  coast.reachable = true
  const forwarded = await ladder.flush()
  const late = (await coastOffice.recv())!
  const left = await ladder.buffered()
  console.log(`coast     carried ${late.text!} on a flush of ${forwarded}, ${left} waiting`)

  // Orders from shore come back over whichever listening link is in reach.
  await coastOffice.send(ORDERS, 'return to port')
  const order = await ladder.recv()
  console.log(`vessel    took ${order.text!} over the coast network`)

  // A ladder does one thing at a time, so a vessel that listens and reports waits for
  // orders with a limit and reports between waits.
  try {
    await ladder.recv(QUIET_MS)
  } catch {
    console.log(`vessel    heard nothing more from shore within ${QUIET_MS} ms`)
  }
  await ladder.send(REPORT, 'report 5')
  const last = (await coastOffice.recv())!
  console.log(`coast     carried ${last.text!} between waits`)

  return { first, stormy, counts: [waiting, still, left], order: order.text, last: last.text }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: {
  first: Delivery
  stormy: Delivery
  counts: number[]
  order?: string
  last?: string
}): void {
  assert.equal(seen.first, Delivery.Sent)
  assert.equal(seen.stormy, Delivery.Buffered)
  assert.deepEqual(seen.counts, [1, 1, 0])
  assert.equal(seen.order, 'return to port')
  assert.equal(seen.last, 'report 5')
}
