// The first example on the README and the site: one field node's reading taken off a
// wire, smoothed, signed, and packed for a link that charges by the byte, start to
// finish with nothing plugged in.

import assert from 'node:assert/strict'

// ANCHOR: example
import { packSamples, unpackSamples } from '@pamoja/codec'
import { Smoother } from '@pamoja/kit'
import { DeviceIdentity, verify } from '@pamoja/security'
import { ds18b20 } from '@pamoja/sensors'

// A stand-in for the thermometer. On a running node these nine bytes arrive from the
// 1-Wire bus; here the library builds what a part sitting at 25.0625 C would send, so the
// program runs with nothing plugged in.
const offTheBus = ds18b20.buildScratchpad(25.0625, 12, 75, -10)

// Everything below is the node's own code, and none of it cares where the bytes came from.
// The part checksums every read, so a value mangled on a long run comes back as an error
// instead of a plausible temperature a couple of degrees off.
const celsius = ds18b20.parseScratchpad(offTheBus).microCelsius / 1e6
console.log(`read      ${celsius.toFixed(4)} C`) // read      25.0625 C

// Readings jitter. A smoother follows the trend without keeping a history to do it, which
// matters on a part with kilobytes of RAM.
const smoother = new Smoother(0.5)
smoother.update(celsius)
const smoothed = smoother.update(celsius + 1)
console.log(`smoothed  ${smoothed.toFixed(4)} C`) // smoothed  25.5625 C

// Sign it, so the gateway can tell this device's readings from anyone else's.
const device = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))
const reading = smoothed.toFixed(2)
const signature = device.sign(reading)
if (!verify(device.publicKey(), reading, signature)) {
  throw new Error('the gateway would reject this reading')
}
console.log(`signed    ${reading} C, and the signature checks out`)

// Send a batch rather than a reading at a time. Successive samples differ by very little,
// so writing down the differences costs a fraction of eight bytes each.
const batch = [2506, 2507, 2509, 2508, 2510]
const packed = packSamples(batch)
console.log(`packed    ${batch.length} readings into ${packed.length} bytes`)
// ANCHOR_END: example

assert.equal(celsius, 25.0625)
assert.ok(smoothed > celsius && smoothed < celsius + 1)
assert.ok(verify(device.publicKey(), reading, signature))
assert.ok(packed.length < batch.length * 8)
assert.deepEqual(unpackSamples(packed), batch)
