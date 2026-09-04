// The codecs guide example; see docs/guides/codec.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { Quantizer, fromCbor, packSamples, toCbor, unpackSamples } from '@pamoja/codec'

// The same reading in CBOR instead of JSON, half the bytes. 21.5 rides as a half-precision
// float, the shortest form RFC 8949 allows for it, so these are the bytes the specification
// fixes rather than one encoder's dialect.
const reading = { c: 21.5, ok: true }
const cbor = toCbor(reading)
assert.deepEqual([...cbor], [0xa2, 0x61, 0x63, 0xf9, 0x4d, 0x60, 0x62, 0x6f, 0x6b, 0xf5])
assert.deepEqual(fromCbor(cbor), reading)

// A batch packs to a count, then the difference between each sample and the one before it,
// zigzagged and written as a LEB128 varint. The four small steps cost a byte each; the jump
// to 900 zigzags to 1776 and costs the two bytes 0xf0 0x0d.
const samples = [10, 11, 13, 12, 900]
const packed = packSamples(samples)
assert.deepEqual([...packed], [0x05, 0x14, 0x02, 0x04, 0x01, 0xf0, 0x0d])
assert.deepEqual(unpackSamples(packed), samples)

// A quantizer packs float readings the same way, rounding at the scale first. Nothing in
// the bytes records the scale, so encode and decode have to agree on it.
const quantizer = new Quantizer(100)
const readings = [20.0, 20.1, 20.2, 20.3]
const packedReadings = quantizer.encode(readings)
assert.deepEqual([...packedReadings], [0x04, 0xa0, 0x1f, 0x14, 0x14, 0x14])
for (const [index, value] of quantizer.decode(packedReadings).entries()) {
  assert.ok(Math.abs(value - readings[index]) <= 0.01)
}
// ANCHOR_END: example
