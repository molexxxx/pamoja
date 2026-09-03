// The first example on the README and the site: a reading off a wire, smoothed,
// signed, and packed for a metered link, with nothing plugged in.

// ANCHOR: example
import assert from 'node:assert/strict'

import { packSamples, unpackSamples } from '@pamoja/codec'
import { Smoother } from '@pamoja/kit'
import { DeviceIdentity, verify } from '@pamoja/security'
import { ds18b20 } from '@pamoja/sensors'

// The nine bytes a DS18B20 sends, CRC last; a bad CRC is a rejected read.
const scratchpad = Buffer.from([0x91, 0x01, 0x4b, 0x46, 0x7f, 0xff, 0x0c, 0x10, 0x00])
scratchpad[8] = ds18b20.crc8(scratchpad.subarray(0, 8))
const celsius = ds18b20.parseScratchpad(scratchpad).microCelsius / 1e6
assert.equal(celsius, 25.0625)

// Smooth the noise out of successive readings.
const smoother = new Smoother(0.5)
smoother.update(celsius)
const smoothed = smoother.update(celsius + 1)
assert.ok(smoothed > celsius && smoothed < celsius + 1)

// Sign the reading so a gateway can prove which device sent it.
const device = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))
const payload = smoothed.toFixed(2)
const signature = device.sign(payload)
assert.ok(verify(device.publicKey(), payload, signature))

// Pack a batch of readings for a link where every byte costs money.
const samples = [2506, 2507, 2509, 2508, 2510]
const packed = packSamples(samples)
assert.ok(packed.length < samples.length * 8)
assert.deepEqual(unpackSamples(packed), samples)
// ANCHOR_END: example
