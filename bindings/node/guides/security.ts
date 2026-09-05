// The device identity guide example; see docs/guides/security.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { DeviceIdentity, fingerprint, verify } from '@pamoja/security'

// The seed is provisioned into the device once and never leaves it. A real one comes from
// the factory or a secure element; any 32 bytes stand in here.
const device = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))

// Only the 32-byte public key travels to the gateway. Its fingerprint is the short form an
// operator reads off a screen to tell one device from another.
const gatewayKey = device.publicKey()
console.log(`device     ${fingerprint(gatewayKey)}`)

// Signing is deterministic, so the same reading always produces the same 64 bytes and there
// is no randomness to get wrong on a microcontroller.
const reading = 'meter-4 1182.750 kWh'
const signature = device.sign(reading)
if (verify(gatewayKey, reading, signature)) {
  console.log(`accepted   ${reading}`)
} else {
  console.log('rejected   a reading the device really did sign, which should never happen')
}

// A digit changed in transit no longer matches what was signed.
const edited = 'meter-4 1082.750 kWh'
if (verify(gatewayKey, edited, signature)) {
  console.log('accepted   an edited reading, which should never happen')
} else {
  console.log(`rejected   ${edited}`)
}

// Nor does the same reading offered under another device's key.
const impostor = DeviceIdentity.fromSeed(Buffer.alloc(32, 90))
if (verify(impostor.publicKey(), reading, signature)) {
  console.log('accepted   an impostor, which should never happen')
} else {
  console.log("rejected   a signature offered under another device's key")
}
// ANCHOR_END: example

assert.deepEqual(device.sign(reading), signature)
assert.ok(verify(gatewayKey, reading, signature))
assert.ok(!verify(gatewayKey, edited, signature))
assert.ok(!verify(impostor.publicKey(), reading, signature))
