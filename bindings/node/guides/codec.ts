// The codecs guide example; see docs/guides/codec.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Quantizer, fromCbor, packSamples, toCbor, unpackSamples } from '@pamoja/codec'

// The same reading as JSON and as CBOR. Nothing is lost, and 21.5 rides as a
// half-precision float, the shortest form RFC 8949 allows for it.
const reading = { c: 21.5, ok: true }
const asJson = Buffer.from(JSON.stringify(reading))
const cbor = toCbor(reading)
console.log(`json      ${asJson.length} bytes`)
console.log(`cbor      ${cbor.length} bytes`)

// A gateway that speaks JSON gets it back unchanged, so the compact form is a transport
// choice rather than a different data model.
const restored = fromCbor(cbor)
console.log(`back to json, unchanged: ${JSON.stringify(restored) === JSON.stringify(reading)}`)

// A batch of readings packs to a count, then the difference between each sample and the
// one before it. Successive readings differ by very little, so the differences cost about
// a byte each where the samples would cost eight.
const samples = [10, 11, 13, 12, 900]
const packed = packSamples(samples)
console.log(`batch     ${samples.length} samples in ${packed.length} bytes`)
console.log(`unpacked  ${unpackSamples(packed).join(', ')}`)

// Readings that arrive as floats pack the same way once a scale is chosen. Nothing in the
// bytes records that scale, so the sender and the receiver have to agree on it.
const quantizer = new Quantizer(100)
const celsius = [20.0, 20.1, 20.2, 20.3]
const packedCelsius = quantizer.encode(celsius)
const recovered = quantizer.decode(packedCelsius)
console.log(`degrees   ${celsius.length} readings in ${packedCelsius.length} bytes`)
console.log(`recovered ${[...recovered].map((v) => v.toFixed(1)).join(', ')}`)
// ANCHOR_END: example

// The bytes each specification fixes are pinned once, in the crate tests and the
// generated conformance vectors, so a guide asserts behaviour instead.
assert.ok(cbor.length < asJson.length)
assert.deepEqual(restored, reading)
assert.deepEqual(unpackSamples(packed), samples)
assert.ok(packed.length < samples.length * 8)
for (const [index, value] of [...recovered].entries()) {
  assert.ok(Math.abs(value - celsius[index]!) <= 0.01)
}
