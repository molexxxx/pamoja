// The secured session guide example; see docs/guides/session.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { AgreementKey, Role, Session } from '@pamoja/session'

// Each device is provisioned with a 32-byte seed and publishes the key it derives. These are
// the X25519 pair RFC 7748 section 6.1 publishes, so the derivation is pinned to the
// specification rather than checked against itself.
const node = new AgreementKey(
  Buffer.from('77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a', 'hex')
)
const gateway = new AgreementKey(
  Buffer.from('5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb', 'hex')
)
assert.equal(
  node.publicKey().toString('hex'),
  '8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a'
)

// Neither side sends the session key. Both derive it from the shared secret, a salt that
// travels in the clear, and both public keys. The roles have to be opposite.
const salt = Buffer.alloc(16, 0x09)
const uplink = new Session(node, gateway.publicKey(), salt, Role.Initiator)
const downlink = new Session(gateway, node.publicKey(), salt, Role.Responder)

// The pump id is authenticated but not encrypted, so a router still reads it while any change
// to it fails the tag.
const label = Buffer.from('pump-3')
const sealed = uplink.seal(Buffer.from('flow=41.2'), label)
assert.notEqual(sealed.ciphertext.toString(), 'flow=41.2')
assert.equal(downlink.open(sealed, label).toString(), 'flow=41.2')

// The anti-replay window refuses a counter it has already accepted, so a frame captured off
// the air and sent again is not delivered a second time.
assert.throws(() => downlink.open(sealed, label))
// ANCHOR_END: example
