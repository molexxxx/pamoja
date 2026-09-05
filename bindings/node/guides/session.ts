// The secured session guide example; see docs/guides/session.md.

import assert from 'node:assert/strict'
import { randomBytes } from 'node:crypto'

// ANCHOR: example
import { AgreementKey, Role, Session } from '@pamoja/session'

// Each device is provisioned with a 32-byte seed and publishes the key it derives. A real
// seed comes from the factory or a secure element; any 32 bytes stand in here.
const node = new AgreementKey(Buffer.alloc(32, 7))
const gateway = new AgreementKey(Buffer.alloc(32, 9))

// Neither side sends the session key. Both derive it from the shared secret, a salt that
// travels in the clear, and both public keys, with opposite roles.
//
// The salt must be fresh for every session: reusing one derives the same key from the same
// pair of devices twice. The initiator draws it and sends it in the clear, so the responder
// uses the salt it received rather than one of its own.
const salt = randomBytes(16)
const uplink = new Session(node, gateway.publicKey(), salt, Role.Initiator)
const downlink = new Session(gateway, node.publicKey(), salt, Role.Responder)
console.log('both sides derived a key without sending one')

// The pump id is authenticated but not encrypted, so a router still reads it while any
// change to it fails the tag.
const reading = Buffer.from('flow=41.2')
const sealed = uplink.seal(reading, Buffer.from('pump-3'))
console.log(`sealed    the reading is no longer readable: ${!sealed.ciphertext.equals(reading)}`)
console.log(`opened    ${downlink.open(sealed, Buffer.from('pump-3')).toString()}`)

// The anti-replay window refuses a counter it has already accepted, so a frame captured
// off the air and sent again is not delivered a second time.
try {
  downlink.open(sealed, Buffer.from('pump-3'))
  console.log('a replayed frame was accepted, which should never happen')
} catch (error) {
  console.log(`replay    refused: ${(error as Error).message}`)
}
// ANCHOR_END: example

assert.ok(!sealed.ciphertext.equals(reading))
