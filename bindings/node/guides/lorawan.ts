// The LoRaWAN activation guide example; see docs/guides/lorawan.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { device, grantAccept, grantSession } from '@pamoja/lorawan'

// The root key is provisioned into the device at the factory and known to the network
// server. It is the only secret either side starts with; any 16 bytes stand in here.
const appKey = Buffer.alloc(16, 7)

// The device asks to join with a nonce it has not used before, which is what stops an old
// accept being replayed at it.
const devNonce = 1
const node = device(Buffer.alloc(8), Buffer.alloc(8), appKey)

// The network grants the join. It draws its own nonce, names the network the device is
// joining, and assigns the address the device will answer to from then on.
const devAddr = 0x26012e43
const offer = { appNonce: 2, netId: 19, devAddr }
const accept = grantAccept(offer, appKey, devNonce)
console.log(`granted   address 0x${devAddr.toString(16).toUpperCase()} in a ${accept.length}-byte accept`)

// The device verifies it against the root key. A join accept carries no device identifier,
// so only that key decides whether it is for this device.
const joined = node.acceptJoin(accept, devNonce)
console.log(`joined    the device took address 0x${joined.devAddr.toString(16).toUpperCase()}`)

// Neither side transmits a session key. Both derive the same pair from the root key and the
// two nonces, so the network reads what the device sends without ever having been told how.
const network = grantSession(offer, appKey, devNonce)
const uplink = joined.session().encodeUplink(1, 1, Buffer.from('level=high'))
const received = network.decode(uplink, 1)
console.log(`uplink    the network read ${received.payload.toString()}`)

// A single byte changed in the air fails that check, so no one else can admit the device or
// put words in its mouth.
const forged = Buffer.from(accept)
forged[1] ^= 0xff
try {
  node.acceptJoin(forged, devNonce)
  console.log('a forged accept was taken, which should never happen')
} catch (error) {
  console.log(`forged    accept refused: ${(error as Error).message}`)
}
// ANCHOR_END: example

assert.equal(joined.devAddr, devAddr)
assert.equal(received.payload.toString(), 'level=high')
