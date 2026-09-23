// The codecs guide example; see docs/guides/codec.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Quantizer, fromCbor, packSamples, toCbor, unpackSamples } from '@pamoja/codec'
import { LoraRegion, planFor } from '@pamoja/lora'

// The gauge reports over LoRaWAN in the US915 plan, at the slowest data rate because it
// reaches farthest. An uplink there carries only a few bytes of payload.
const budget = planFor(LoraRegion.Us915).maxPayload(0)!.application
const fits = (bytes: number) => (bytes <= budget ? 'fits one uplink' : 'too big for one uplink')
console.log(`uplink    carries ${budget} bytes at the slowest US915 data rate`)

// One reading as the JSON a web service would take. CBOR carries the same document in
// fewer bytes, but every key name still rides along with every reading.
const reading = { depth_cm: 142.5, air_c: -6.5, battery_mv: 3712 }
const json = Buffer.from(JSON.stringify(reading))
const cbor = toCbor(reading)
console.log(`json      ${json.length} bytes, ${fits(json.length)}`)
console.log(`cbor      ${cbor.length} bytes, ${fits(cbor.length)}`)
console.log(`cbor      reads back as ${JSON.stringify(fromCbor(cbor))}`)

// A batch the gauge and the server agree on needs no key names. Six hourly depths, kept to
// the millimeter, pack to a count, the first depth, and five small steps.
const quantizer = new Quantizer(10)
const depths = [142.5, 143.8, 145.2, 146.0, 145.7, 145.5]
const depthBatch = quantizer.encode(depths)
const depthBytes = depthBatch.length
console.log(`depths    ${depths.length} readings in ${depthBytes} bytes, ${fits(depthBytes)}`)
const depthsBack = quantizer.decode(depthBatch).map((depth) => depth.toFixed(1))
console.log(`depths    read back as ${depthsBack.join(', ')}`)

// Battery millivolts are whole numbers already, so they pack with no scale, and a falling
// voltage packs as small as a rising one.
const battery = [3712, 3709, 3705, 3702, 3698, 3695]
const batteryBatch = packSamples(battery)
const batteryBytes = batteryBatch.length
console.log(`battery   ${battery.length} readings in ${batteryBytes} bytes, ${fits(batteryBytes)}`)
console.log(`battery   reads back as ${unpackSamples(batteryBatch).join(', ')}`)

// Heavy snowfall can swallow the sensor's echo, leaving no depth at all. The quantizer
// refuses the batch rather than send the gap as a depth.
try {
  quantizer.encode([145.5, NaN])
} catch (error) {
  console.log(`depths    refused a batch with a missing depth: ${(error as Error).message}`)
}
// ANCHOR_END: example

assert.ok(cbor.length < json.length)
assert.ok(cbor.length > budget)
assert.ok(depthBytes <= budget && batteryBytes <= budget)
assert.ok(JSON.stringify(fromCbor(cbor)).startsWith('{"air_c"'), 'keys come back sorted')
for (const [index, depth] of quantizer.decode(depthBatch).entries()) {
  assert.ok(Math.abs(depth - depths[index]!) <= 0.05)
}
assert.deepEqual(unpackSamples(batteryBatch), battery)
assert.throws(() => quantizer.encode([145.5, NaN]))
