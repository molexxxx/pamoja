// The LoRaWAN guide example; see docs/guides/lorawan.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { device, grantAccept, session } from '@pamoja/lorawan'

// A join accept captured off a live EU868 network, the root key it was signed under, and
// the session keys an independent implementation derived from it. Published at
// https://github.com/anthonykirby/lora-packet/issues/10
const captured = Buffer.from(
  '204dd85ae608b87fc4889970b7d2042c9e72959b0057aed6094b16003df12de145',
  'hex'
)
const appKey = Buffer.from('b6b53f4a168a7a88bdf7ea135ce9cfca', 'hex')
const devNonce = 0xcc85

// The network half: the address and radio settings this network grants, encrypted and
// signed under the root key, are the frame that was captured.
const offer = {
  appNonce: 0x00e5063a,
  netId: 0x13,
  devAddr: 0x26012e43,
  dlSettings: 0x03,
  rxDelay: 0x01,
  cflist: Buffer.from('184f84e85684b85e84886684586e8400', 'hex'),
}
assert.deepEqual(grantAccept(offer, appKey, devNonce), captured)

// The device half. A join accept carries no EUI, so only the root key decides whether it
// verifies.
const node = device(Buffer.alloc(8), Buffer.alloc(8), appKey)
const accepted = node.acceptJoin(captured, devNonce)
assert.equal(accepted.devAddr, 0x26012e43)

// Neither side transmits a session key; both derive it from the two nonces. What the
// device derived is read back by a session holding the keys published with the capture.
const keys = Buffer.from(
  '2c96f7028184bb0be8aa49275290d4fcf3a5c8f0232a38c144029c165865802c',
  'hex'
)
const gateway = session(0x26012e43, keys.subarray(0, 16), keys.subarray(16))
const uplink = accepted.session().encodeUplink(1, 1, Buffer.from('real'))
assert.equal(gateway.decode(uplink, 1).payload.toString(), 'real')

// A single byte changed in the air fails the MIC, so no one else can admit the device.
const forged = Buffer.from(captured)
forged[1] ^= 0xff
assert.throws(() => node.acceptJoin(forged, devNonce))
// ANCHOR_END: example
