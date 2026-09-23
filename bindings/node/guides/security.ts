// The device identity guide example; see docs/guides/security.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { DeviceIdentity, fingerprint, verify, verifyMessage } from '@pamoja/security'

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

// On a link the signature and the reading usually travel as one message, signature first,
// and the gateway gets the reading back only once it has checked it.
const message = device.signMessage(reading)
const size = message.length
console.log(`message    ${size} bytes on the wire, the signature and the reading together`)
const carried = verifyMessage(gatewayKey, message)
if (carried) {
  console.log(`accepted   ${carried.toString()}, read out of the message`)
} else {
  console.log('rejected   a message the device really did sign, which should never happen')
}

// A message that lost its last byte on the way is refused whole.
if (verifyMessage(gatewayKey, message.subarray(0, size - 1))) {
  console.log('accepted   a message cut short, which should never happen')
} else {
  console.log('rejected   a message that lost its last byte on the way')
}
// ANCHOR_END: example

assert.deepEqual(device.sign(reading), signature)
assert.ok(verify(gatewayKey, reading, signature))
assert.ok(!verify(gatewayKey, edited, signature))
assert.ok(!verify(impostor.publicKey(), reading, signature))
assert.equal(message.length, 64 + Buffer.byteLength(reading))
assert.equal(verifyMessage(gatewayKey, message)?.toString(), reading)
assert.equal(verifyMessage(gatewayKey, message.subarray(0, message.length - 1)), null)
