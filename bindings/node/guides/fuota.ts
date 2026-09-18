// The firmware update over the air guide example; see docs/guides/fuota.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { DeviceIdentity } from '@pamoja/security'
import { fragment, multicast } from '@pamoja/lorawan'
import { frameBlock, imageDigest, signManifest, splitBlock, Updater } from '@pamoja/update'

// The publisher signs releases; the devices in the field are anchored to its public half
// and will take firmware from nobody else, however it reaches them.
const publisher = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))
const vendor = Buffer.alloc(16, 10)
const flowMeter = Buffer.alloc(16, 11)

const image = Buffer.from('firmware for a flow meter, version two, long enough to need fragmenting')
const envelope = signManifest(
  {
    sequence: 2,
    vendorId: vendor,
    classId: flowMeter,
    storage: 1,
    digest: imageDigest(image),
    size: image.length,
  },
  publisher,
)

// One block carries the release: the signed manifest and the image behind a header that
// says where each begins. The transport moves bytes and vouches for none of them.
const block = frameBlock(envelope, image)
console.log(`release   ${block.length} bytes: a signed manifest and ${image.length} of image`)

// The group every device in the field belongs to. Its key travels wrapped under a key each
// device derives from its own root key, so the broadcast key is never in the clear.
const deviceRootKey = Buffer.alloc(16, 0x2b)
const groupKey = Buffer.alloc(16, 0x77)
const groupAddr = 0x26010042
const keKey = multicast.keKey(multicast.rootKey(deviceRootKey))
const wrapped = multicast.wrapKey(keKey, groupKey)
const unwrapped = multicast.key(keKey, wrapped)
console.log(
  `group     0x${groupAddr.toString(16).toUpperCase()} keyed by a wrapped key the device unwraps: ` +
    `${unwrapped.equals(groupKey)}`,
)
multicast.appSKey(groupKey, groupAddr)

// The server cuts the block into fragments and sends more than there are, so a device that
// misses some can still finish. Each coded fragment is the exclusive-or of a pseudo-random
// half of the originals.
const fragSize = 32
const { nbFrag, padding } = fragment.session(block.length, fragSize)
console.log(`session   ${nbFrag} fragments of ${fragSize} bytes, ${padding} of padding`)

// The device puts it back together in storage it set aside once: the block itself, and room
// to solve for a handful of losses.
const receiver = new fragment.Defragmenter(nbFrag, fragSize, 8)

// The link drops every fourth fragment. The session keeps going until the block is whole.
let sent = 0
let coded = 0
for (let n = 1; n <= nbFrag * 2; n++) {
  if (n % 4 === 0) {
    continue
  }
  sent += 1
  if (n > nbFrag) {
    coded += 1
  }
  if (receiver.fragment(n, fragment.fragment(block, fragSize, n))) {
    break
  }
}
console.log(`received  ${sent} fragments, ${coded} of them coded, and the block is whole`)

// What the device built is checked against the code the session setup carried, taken over
// the block a piece at a time so the image is never held twice.
const blockKey = fragment.dataBlockIntKey(deviceRootKey)
const descriptor = Buffer.from('PJU1')
const expected = new fragment.BlockMic(blockKey, 1, 0, descriptor, block.length)
expected.update(block)
const built = new fragment.BlockMic(blockKey, 1, 0, descriptor, block.length)
built.update(receiver.block.subarray(0, block.length))
const intact = built.finish().equals(expected.finish())
console.log(`checked   the block the device built is the one the server sent: ${intact}`)

// Only now does the update itself get a say. The header says where the manifest ends;
// everything after that is the manifest's decision, exactly as for a wired update.
const carried = splitBlock(receiver.block.subarray(0, block.length))
const updater = new Updater(vendor, flowMeter, publisher.publicKey(), 2, 4096)
updater.provision(0, 1)
const slot = updater.stage(carried.envelope, carried.image)
console.log(`staged    into slot ${slot}, leaving the running image alone`)

// A release broadcast to everyone is still refused by anyone it is not for. This one is
// signed by another key.
const impostor = DeviceIdentity.fromSeed(Buffer.alloc(32, 90))
const forged = signManifest(
  {
    sequence: 2,
    vendorId: vendor,
    classId: flowMeter,
    storage: 1,
    digest: imageDigest(image),
    size: image.length,
  },
  impostor,
)
try {
  updater.stage(forged, image)
  console.log('a forged release was accepted, which should never happen')
} catch (error) {
  console.log(`forged    refused: ${(error as Error).message}`)
}
// ANCHOR_END: example

assert.equal(intact, true)
assert.equal(slot, 1)
assert.deepEqual(carried.image, image)
