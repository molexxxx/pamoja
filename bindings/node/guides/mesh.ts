// The mesh framing guide example; see docs/guides/mesh.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { BROADCAST, HEADER_LEN, SeenPackets, broadcast, parse, relayed } from '@pamoja/mesh'

// A river gauge floods a level reading to every node in range. The header is fixed and
// big-endian: version, source, destination, sequence id, hop limit, then the payload and
// a checksum over everything but the hop limit.
const RIVER_GAUGE = 305419896
const reading = broadcast(RIVER_GAUGE, 1, Buffer.from('level=high'))
const to = reading.dst === BROADCAST ? 'every node in range' : 'one node'
console.log(`sent      ${reading.bytes.length} bytes to ${to}, hop limit ${reading.hopLimit}`)

// A neighbor hears it. Every node in range rebroadcasts, so the same packet arrives
// several times over; the source and sequence id decide which copy is the first.
const received = parse(reading.bytes)
console.log(`payload   ${received.payload.toString()}`)
const seen = new SeenPackets(64)
const first = seen.record(received.src, received.id)
const again = seen.record(received.src, received.id)
if (first && !again) {
  console.log('dedup     the first copy is relayed, and the second is dropped')
}

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
// frame without recomputing it and the check stays end to end.
const forwarded = relayed(received.bytes)!
const onward = parse(forwarded.bytes)
console.log(
  `relayed   hop limit ${forwarded.hopLimit}, and the checksum still holds: ${onward.payload.toString()}`,
)

// A frame that has run out of hops is not relayed again, which is what ends the flood.
if (relayed(broadcast(RIVER_GAUGE, 1, Buffer.from('level=high'), 0).bytes) === null) {
  console.log('spent     at hop limit 0 the frame goes no further')
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

assert.deepEqual(received.payload, Buffer.from('level=high'))
assert.equal(forwarded.hopLimit, received.hopLimit - 1)
assert.deepEqual(onward.payload, received.payload)

// ANCHOR: flood
import { DEFAULT_HOP_LIMIT, MAX_PAYLOAD } from '@pamoja/mesh'

// Six nodes down a river valley, each in range of its neighbors only. The gauge at the top,
// node 0, floods a reading, and every node that hears a packet for the first time takes it
// and relays it while hops remain. Copies that come back up the valley are echoes, which the
// memory of each node drops.
const nodes = 6
const memory = Array.from({ length: nodes }, () => new SeenPackets(64))
const flooded = broadcast(0, 1, Buffer.from('level=high'))
memory[0].record(flooded.src, flooded.id)
const onTheAir = [{ from: 0, bytes: flooded.bytes }]
let delivered = 0
let relays = 0
let echoes = 0
let farthest = 0
for (let sent = onTheAir.shift(); sent; sent = onTheAir.shift()) {
  for (const node of [sent.from - 1, sent.from + 1]) {
    if (node < 0 || node >= nodes) {
      continue
    }
    const heard = parse(sent.bytes)
    if (!memory[node].record(heard.src, heard.id)) {
      echoes += 1
      continue
    }
    delivered += 1
    farthest = Math.max(farthest, node)
    const onwardFrame = relayed(heard.bytes)
    if (onwardFrame) {
      relays += 1
      onTheAir.push({ from: node, bytes: onwardFrame.bytes })
    }
  }
}
console.log(`flood     ${delivered} nodes took the reading, ${relays} relayed it, ${echoes} echoes were dropped`)
console.log(
  `reach     node ${farthest} was the farthest, ${farthest} hops out, and node ${nodes - 1} never heard it`,
)

// A node whose memory holds two packets hears the reading, then two packets from other
// nodes, then a late copy of the reading by a longer path. It has forgotten the reading by
// then and relays it again; on a busy mesh that repeats without end.
const small = new SeenPackets(2)
const rain = broadcast(7, 1, Buffer.from('rain=4mm'))
const wind = broadcast(8, 1, Buffer.from('wind=12'))
small.record(flooded.src, flooded.id)
small.record(rain.src, rain.id)
small.record(wind.src, wind.id)
if (small.record(flooded.src, flooded.id)) {
  console.log('forgot    a memory of two packets relays the late copy again')
}

// A payload one byte past what a frame carries is refused before it is built. A frame is
// sized to the 250 bytes ESP-NOW carries, less the header and the checksum.
try {
  broadcast(0, 2, Buffer.alloc(MAX_PAYLOAD + 1))
  console.log('an oversized payload was framed, which should never happen')
} catch (error) {
  console.log(`too long  ${MAX_PAYLOAD + 1} bytes refused: ${(error as Error).message}`)
}
// ANCHOR_END: flood

assert.equal(farthest, DEFAULT_HOP_LIMIT + 1)
