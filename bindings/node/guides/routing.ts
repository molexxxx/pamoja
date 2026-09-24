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
// that arrived through a relay proves that relay is a way back, at the cost the packet
// reports. The table keeps the cheapest way it has heard, and a tie keeps the way in use so
// two equal paths do not flap. Word from the relay already in use is taken even when it is
// worse, which is how a failing link lets a detour win.
const router = new Router(GATEWAY, 4)
for (const [via, cost] of [
  [NORTH_RELAY, 2],
  [EAST_RELAY, 1],
  [SOUTH_RELAY, 4],
  [NORTH_RELAY, 1],
  [EAST_RELAY, 3],
  [NORTH_RELAY, 2],
]) {
  const changed = router.observe(PUMP, via, cost)
  const route = router.route(PUMP)
  const outcome = changed ? 'so the route is' : 'and the route stays'
  console.log(
    `heard     the pump via ${via} at cost ${cost}, ${outcome} ${route?.nextHop} at cost ${route?.cost}`,
  )
}

// The table lists what it holds, one route for each node it has heard from.
router.observe(TANK, NORTH_RELAY, 3)
const held = router
  .routes()
  .map((route) => `to ${route.dst} via ${route.nextHop} at cost ${route.cost}`)
console.log(`table     ${router.size} routes of ${router.capacity}: ${held.join(', ')}`)

// Every packet gets one of three answers: deliver it here, relay it to the neighbor on the
// way, or flood it because no route is known yet.
for (const [name, address] of [
  ['gateway', GATEWAY],
  ['pump', PUMP],
  ['silo', SILO],
] as const) {
  const decision = router.forward(address)
  if (decision.action === ForwardAction.Deliver) {
    console.log(`${name.padEnd(10)}deliver here`)
  } else if (decision.action === ForwardAction.Relay) {
    console.log(`${name.padEnd(10)}relay via ${decision.nextHop}`)
  } else {
    console.log(`${name.padEnd(10)}flood, no route known`)
  }
}

// The table keeps no clock, so a route through a relay that has gone quiet stays until the
// caller forgets it, typically when a relayed packet goes unanswered. Forgetting returns the
// node's traffic to flooding, the answer that always works.
router.forget(PUMP)
if (router.forward(PUMP).action === ForwardAction.Flood) {
  console.log(`forgot    the pump, so it floods again, and ${router.size} route is left`)
}
// ANCHOR_END: example

assert.equal(router.size, 1)
assert.equal(router.nextHop(TANK), NORTH_RELAY)

// ANCHOR: limits
// A table has a fixed number of slots. Once they are full, a cheaper route takes the slot of
// the costliest one held and a costlier route is refused, so a small table keeps the nodes
// nearest to it and floods to the rest.
const WELL = 11
const GATE = 12
const small = new Router(GATEWAY, 2)
small.observe(TANK, NORTH_RELAY, 3)
small.observe(SILO, SOUTH_RELAY, 5)
const kept = small
  .routes()
  .map((route) => `to ${route.dst} via ${route.nextHop} at cost ${route.cost}`)
console.log(`full      ${small.size} routes of ${small.capacity}: ${kept.join(', ')}`)
if (small.observe(WELL, EAST_RELAY, 2) && small.route(SILO) === null) {
  console.log('evicted   the well at cost 2 took the slot of the silo, the costliest held')
}
if (!small.observe(GATE, EAST_RELAY, 6) && small.forward(GATE).action === ForwardAction.Flood) {
  console.log('refused   the gate at cost 6 costs more than every route held, so it floods')
}

// A flood echoes, and the gateway hears its own packets come back through the relays. A
// route to the node itself is never learned, whatever it costs.
if (!router.observe(GATEWAY, EAST_RELAY, 2)) {
  console.log("echo      the gateway's own packet coming back teaches it nothing")
}

// A table with no slots is flooding with nothing remembered, which a node with no memory to
// spare can still do. A packet for the node itself is still delivered.
const none = new Router(GATEWAY, 0)
const learned = none.observe(PUMP, EAST_RELAY, 1)
if (
  !learned &&
  none.forward(PUMP).action === ForwardAction.Flood &&
  none.forward(GATEWAY).action === ForwardAction.Deliver
) {
  console.log(
    'no room   a table of 0 learns nothing: the pump floods, and the gateway still delivers',
  )
}
// ANCHOR_END: limits

assert.equal(small.nextHop(WELL), EAST_RELAY)
assert.equal(none.isEmpty, true)

// ANCHOR: site
import {
  DEFAULT_HOP_LIMIT,
  type MeshFrame,
  SeenPackets,
  broadcast,
  frame,
  parse,
  relayed,
} from '@pamoja/mesh'

// Who hears whom on the site. The gateway hears the three relays, and each relay hears the
// one node beyond it; the pump is out of the gateway's range.
const site: Record<number, number[]> = {
  [GATEWAY]: [NORTH_RELAY, EAST_RELAY, SOUTH_RELAY],
  [NORTH_RELAY]: [GATEWAY, TANK],
  [EAST_RELAY]: [GATEWAY, PUMP],
  [SOUTH_RELAY]: [GATEWAY, SILO],
  [TANK]: [NORTH_RELAY],
  [PUMP]: [EAST_RELAY],
  [SILO]: [SOUTH_RELAY],
}
const nodes = Object.keys(site).map(Number)

// Sends a frame from one node and plays out what the site does with it. A node that hears a
// frame for the first time learns the way back to its source, then asks its own table what
// to do: deliver it, relay it to the one neighbor on the way, or flood it to every neighbor
// in range, spending a hop each time it goes on. Every frame here starts at the default hop
// limit, and one heard straight from its source still has all of it, so the hops a frame has
// come are what it has spent, plus one. Returns how many times a radio sent, and the nodes
// that took the frame.
function send(
  tables: Record<number, Router>,
  from: number,
  first: MeshFrame,
): [number, number[]] {
  const seen = Object.fromEntries(nodes.map((node) => [node, new SeenPackets(64)]))
  seen[from].record(first.src, first.id)
  let sends = 0
  const reached: number[] = []
  const onTheAir = [{ sender: from, sent: first }]
  for (let next = onTheAir.shift(); next; next = onTheAir.shift()) {
    const { sender, sent } = next
    const decision = tables[sender].forward(sent.dst)
    if (decision.action === ForwardAction.Deliver) {
      continue
    }
    const to = decision.nextHop === null ? site[sender] : [decision.nextHop]
    sends += 1
    for (const node of to) {
      const heard = parse(sent.bytes)
      if (!seen[node].record(heard.src, heard.id)) {
        continue
      }
      reached.push(node)
      tables[node].observe(heard.src, sender, DEFAULT_HOP_LIMIT - heard.hopLimit + 1)
      const onward = relayed(heard.bytes)
      if (onward) {
        onTheAir.push({ sender: node, sent: onward })
      }
    }
  }
  return [sends, reached]
}

// The pump floods a reading, and every node that takes it learns the way back.
const tables = Object.fromEntries(nodes.map((node) => [node, new Router(node, 8)]))
const reading = broadcast(PUMP, 1, Buffer.from('flow=12'))
const [sends, reached] = send(tables, PUMP, reading)
console.log(
  `flood     the pump's reading took ${sends} sends to reach ${reached.length} nodes, and each learned the way back`,
)

// The gateway answers the pump. Each node on the way relays to the one neighbor its table
// names, so the answer reaches only the nodes on the path.
const answer = frame(GATEWAY, PUMP, 1, Buffer.from('run=10min'))
const [routed, path] = send(tables, GATEWAY, answer)
console.log(`routed    the gateway's answer reached only ${path.join(' and ')}, in ${routed} sends`)

// The same answer on a site that has learned nothing floods, and every node relays it.
const blank = Object.fromEntries(nodes.map((node) => [node, new Router(node, 8)]))
const [flooded, everyone] = send(blank, GATEWAY, answer)
console.log(
  `flooded   with nothing learned, the same answer reached all ${everyone.length} other nodes in ${flooded} sends`,
)
// ANCHOR_END: site

assert.equal(reached.length, nodes.length - 1)
assert.deepEqual(path, [EAST_RELAY, PUMP])
assert.ok(routed < flooded)
assert.equal(tables[GATEWAY].nextHop(PUMP), EAST_RELAY)
assert.equal(tables[TANK].cost(PUMP), 4)
