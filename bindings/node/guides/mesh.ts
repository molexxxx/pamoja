// The mesh framing guide example; see docs/guides/mesh.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { BROADCAST, SeenPackets, broadcast, crc16, parse, relayed } from '@pamoja/mesh'

// A river gauge floods a reading to every node in range. The header is fixed and
// big-endian: version, source, destination, sequence id, hop limit, then the payload
// and a checksum over everything but the hop limit.
const reading = broadcast(0x12345678, 1, Buffer.from('level=high'))
assert.equal(reading.dst, BROADCAST)
assert.equal(
  reading.bytes.toString('hex'),
  '0112345678ffffffff0001036c6576656c3d686967683335'
)

// The checksum is CRC-16/CCITT-FALSE, whose published check value fixes the polynomial
// and the starting value.
assert.equal(crc16(Buffer.from('123456789')), 0x29b1)

// A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
// several times over; the source and sequence id decide which copy is the first.
const received = parse(reading.bytes)
assert.equal(received.payload.toString(), 'level=high')
const seen = new SeenPackets(64)
assert.ok(seen.record(received.src, received.id))
assert.ok(!seen.record(received.src, received.id))

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
// frame without recomputing it and the check stays end to end.
const forwarded = relayed(received.bytes)!
assert.equal(forwarded.hopLimit, received.hopLimit - 1)
assert.deepEqual(parse(forwarded.bytes).payload, received.payload)
assert.equal(relayed(broadcast(0x12345678, 1, Buffer.from('level=high'), 0).bytes), null)

// A payload byte the air mangled fails the checksum rather than reaching the application
// as a plausible reading.
const mangled = Buffer.from(reading.bytes)
mangled[12] ^= 0xff
assert.throws(() => parse(mangled))
// ANCHOR_END: example
