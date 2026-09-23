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
console.log('agreed    both sides derived a key without sending one')

// The pump id is authenticated but not encrypted, so a router still reads it while any
// change to it fails the tag.
const pump = Buffer.from('pump-3')
const reading = Buffer.from('flow=41.2')
const sealed = uplink.seal(reading, pump)
const hidden = sealed.ciphertext.equals(reading) ? 'still' : 'no longer'
console.log(`sealed    counter ${sealed.counter}, and what goes on the wire is ${hidden} the reading`)
console.log(`opened    ${downlink.open(sealed, pump).toString()}`)

// The anti-replay window refuses a counter it has already accepted, so a frame captured
// off the air and sent again is not delivered a second time.
try {
  downlink.open(sealed, pump)
  console.log('a replayed frame was accepted, which should never happen')
} catch (error) {
  console.log(`replay    refused: ${(error as Error).message}`)
}

// A router that rewrites the pump id breaks the tag, so the gateway refuses the frame
// rather than file the reading under the wrong pump. A frame that fails to open leaves its
// counter unused.
const later = uplink.seal(Buffer.from('flow=41.3'), pump)
try {
  downlink.open(later, Buffer.from('pump-4'))
  console.log('a rewritten pump id was accepted, which should never happen')
} catch (error) {
  console.log(`altered   refused: ${(error as Error).message}`)
}

// Radio frames can arrive out of order. The window accepts any counter it has not seen
// among the 64 below the newest, so the frame that was held up still opens.
const newest = uplink.seal(Buffer.from('flow=41.5'), pump)
const first = downlink.open(newest, pump).toString()
const second = downlink.open(later, pump).toString()
console.log(`late      counter ${newest.counter} opened first, then counter ${later.counter}: ${first}, then ${second}`)

// The gateway answers on the same session. Its frames carry the other direction in their
// nonce, so a reply can never be taken for, or replayed as, one from the node.
const order = downlink.seal(Buffer.from('valve=close'), pump)
console.log(`reply     ${uplink.open(order, pump).toString()}, sealed by the gateway and opened by the node`)
// ANCHOR_END: example

assert.ok(!sealed.ciphertext.equals(reading))
assert.throws(() => downlink.open(sealed, pump))
