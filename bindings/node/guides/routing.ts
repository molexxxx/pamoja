// The mesh-routing guide example; see docs/guides/routing.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { ForwardAction, Router } from '@pamoja/routing'

// The nodes on this mesh. An address is just a number; naming them is what makes the
// table below read as a map of the site rather than a list of numbers.
const GATEWAY = 1
const PUMP = 9
const TANK = 10
const NORTH_RELAY = 5
const EAST_RELAY = 7
const SOUTH_RELAY = 3
const SILO = 32

// A node learns the way to another from traffic it already hears: a packet from the pump
// that arrived through the north relay proves that relay is a way back, at the cost the
// packet reports.
const router = new Router(GATEWAY, 4)
router.observe(PUMP, NORTH_RELAY, 2)

// The table keeps only the cheapest way it knows to each node, so a cost-1 report through
// the east relay takes over and the later cost-4 report changes nothing.
router.observe(PUMP, EAST_RELAY, 1)
router.observe(PUMP, SOUTH_RELAY, 4)
router.observe(TANK, NORTH_RELAY, 3)

const route = router.route(PUMP)
console.log(`to the pump   via ${route?.nextHop} at cost ${route?.cost}`)
console.log(`routes held   ${router.size}`)

// Every packet gets one of three answers: deliver it here, relay it to the neighbour on
// the way, or flood it because no route is known yet.
for (const [name, address] of [
  ['gateway', GATEWAY],
  ['pump', PUMP],
  ['silo', SILO],
] as const) {
  const decision = router.forward(address)
  if (decision.action === ForwardAction.Deliver) {
    console.log(`for the ${name.padEnd(8)} deliver here`)
  } else if (decision.action === ForwardAction.Relay) {
    console.log(`for the ${name.padEnd(8)} relay via ${decision.nextHop}`)
  } else {
    console.log(`for the ${name.padEnd(8)} flood, no route known`)
  }
}

// Forgetting a node that has gone quiet returns its traffic to flooding, so routing is an
// optimisation over flooding rather than a second thing that can fail.
router.forget(PUMP)
const after = router.forward(PUMP)
console.log(`pump forgotten, so it floods again: ${after.action === ForwardAction.Flood}`)
// ANCHOR_END: example

assert.equal(route?.nextHop, EAST_RELAY)
assert.equal(route?.cost, 1)
assert.equal(router.size, 1)
assert.equal(after.action, ForwardAction.Flood)

const fresh = new Router(GATEWAY, 4)
assert.equal(fresh.observe(PUMP, NORTH_RELAY, 2), true)
assert.equal(fresh.observe(PUMP, EAST_RELAY, 1), true)
assert.equal(fresh.observe(PUMP, SOUTH_RELAY, 4), false)
assert.equal(fresh.observe(TANK, NORTH_RELAY, 3), true)
assert.equal(fresh.size, 2)
assert.equal(fresh.forward(GATEWAY).action, ForwardAction.Deliver)
assert.equal(fresh.forward(PUMP).action, ForwardAction.Relay)
assert.equal(fresh.forward(PUMP).nextHop, EAST_RELAY)
assert.equal(fresh.forward(SILO).action, ForwardAction.Flood)
