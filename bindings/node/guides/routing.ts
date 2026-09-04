// The mesh-routing guide example; see docs/guides/routing.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { ForwardAction, Router } from '@pamoja/routing'

// A node learns the way to another from traffic it already hears: a packet from 0x09
// that arrived through neighbour 0x05 proves 0x05 is the way back, at the cost the
// packet reports.
const router = new Router(0x01, 4)
assert.equal(router.observe(0x09, 0x05, 2), true)

// The table keeps the cheapest way it knows to each node, so the report of cost 1 takes
// over and the later cost-4 report changes nothing.
assert.equal(router.observe(0x09, 0x07, 1), true)
assert.equal(router.observe(0x09, 0x03, 4), false)
assert.equal(router.observe(0x0a, 0x05, 3), true)
const route = router.route(0x09)
assert.equal(route?.nextHop, 0x07)
assert.equal(route?.cost, 1)
assert.equal(router.size, 2)

// A packet gets one of three answers: deliver it here, relay it to the neighbour on the
// way, or flood it because no route is known yet.
assert.equal(router.forward(0x01).action, ForwardAction.Deliver)
assert.equal(router.forward(0x09).action, ForwardAction.Relay)
assert.equal(router.forward(0x09).nextHop, 0x07)
assert.equal(router.forward(0x20).action, ForwardAction.Flood)

// Forgetting a node that has gone quiet returns its traffic to flooding, so routing is
// an optimisation over flooding rather than a second thing that can fail.
router.forget(0x09)
assert.equal(router.forward(0x09).action, ForwardAction.Flood)
assert.equal(router.size, 1)
// ANCHOR_END: example
