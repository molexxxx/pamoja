// The mesh framing guide example; see docs/guides/mesh.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { BROADCAST, HEADER_LEN, SeenPackets, broadcast, parse, relayed } from '@pamoja/mesh'

// A river gauge floods a level reading to every node in range. The header is fixed and
// big-endian: version, source, destination, sequence id, hop limit, then the payload and
// a checksum over everything but the hop limit.
const RIVER_GAUGE = 305419896
const reading = broadcast(RIVER_GAUGE, 1, Buffer.from('level=high'))
console.log(`sent      ${reading.bytes.length} bytes to every node in range`)
console.log(`addressed to broadcast: ${reading.dst === BROADCAST}`)

// A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
// several times over; the source and sequence id decide which copy is the first.
const received = parse(reading.bytes)
console.log(`payload   ${received.payload.toString()}`)

const seen = new SeenPackets(64)
const first = seen.record(received.src, received.id)
const again = seen.record(received.src, received.id)
console.log(`first copy relayed: ${first}, second copy relayed: ${again}`)

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
// frame without recomputing it and the check stays end to end.
const forwarded = relayed(received.bytes)!
console.log(`relayed   hop limit ${forwarded.hopLimit}`)
const onward = parse(forwarded.bytes)
console.log(`onward    ${onward.payload.toString()}`)

// A frame that has run out of hops is not relayed again, which is what ends the flood.
const spent = relayed(broadcast(RIVER_GAUGE, 1, Buffer.from('level=high'), 0).bytes)
if (spent === null) {
  console.log('spent     hop limit reached, the flood stops here')
} else {
  console.log('a spent frame was relayed, which should never happen')
}

// A payload byte the air mangled fails the checksum rather than reaching the application
// as a plausible reading. The header is a fixed width, so the first byte past it is the
// first byte of the reading itself.
const mangled = Buffer.from(reading.bytes)
mangled[HEADER_LEN] ^= 0xff
try {
  parse(mangled)
  console.log('a mangled frame was accepted, which should never happen')
} catch (error) {
  console.log(`mangled   rejected: ${(error as Error).message}`)
}
// ANCHOR_END: example

// The bytes each specification fixes are pinned once, in the crate tests and the
// generated conformance vectors, so a guide asserts behaviour instead.
assert.deepEqual(received.payload, Buffer.from('level=high'))
assert.equal(first, true)
assert.equal(again, false)
assert.equal(forwarded.hopLimit, received.hopLimit - 1)
assert.deepEqual(onward.payload, received.payload)
assert.equal(spent, null)
assert.throws(() => parse(mangled))
