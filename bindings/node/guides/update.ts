// The signed update guide example; see docs/guides/update.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { DeviceIdentity } from '@pamoja/security'
import {
  type Boot,
  BootAction,
  SlotState,
  Updater,
  imageDigest,
  signManifest,
  verifyEnvelope,
} from '@pamoja/update'

// The publisher's key signs releases; devices in the field are anchored to its public half
// and will take firmware from nobody else.
const publisher = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))
const vendor = Buffer.alloc(16, 0x0a)
const deviceClass = Buffer.alloc(16, 0x0b)

// The release. A manifest says who the image is for, which slot it belongs in, how big it
// is and what it hashes to; nothing about the image itself is taken on trust.
const image = Buffer.from('firmware for a flow meter, version two')
const manifest = {
  sequence: 2,
  vendorId: vendor,
  classId: deviceClass,
  storage: 1,
  digest: imageDigest(image),
  size: image.length,
}
const envelope = signManifest(manifest, publisher)
console.log(`published sequence ${manifest.sequence} in a ${envelope.length}-byte envelope`)

// On the device. It checks the envelope against the key it was anchored to before it
// accepts a single byte of the image.
const opened = verifyEnvelope(envelope, publisher.publicKey())
console.log(`accepted  a release for slot ${opened.storage}`)

// It left the factory running sequence 1 from slot 0, so the release goes to the spare slot
// and the image it is running stays where it is.
const fleet = new Updater(vendor, deviceClass, publisher.publicKey(), 2, 4096)
fleet.provision(0, 1)
fleet.begin(envelope)
for (let at = 0; at < image.length; at += 16) {
  fleet.write(image.subarray(at, at + 16))
}
console.log(`staged    ${fleet.progress().written} of ${image.length} bytes`)
const slot = fleet.finish()
console.log(`written   to slot ${slot}, leaving the running image alone`)

// The first boot into a new image is a trial. It reverts on the next boot unless the device
// confirms that it came up, which is what makes a bad release survivable.
const said = (decision: Boot): string => {
  switch (decision.action) {
    case BootAction.Trying:
      return `slot ${decision.slot} on trial`
    case BootAction.Confirmed:
      return `slot ${decision.slot}, already confirmed`
    default:
      return `slot ${decision.slot} never confirmed, so the device runs slot ${decision.fallback} again`
  }
}
const decision = fleet.onBoot()
console.log(`booting   ${said(decision)}`)
fleet.confirm()
console.log(`confirmed slot ${slot} is now ${fleet.slotRecord(slot).state}`)

// The same release offered again would take the device nowhere new, so it is refused as a
// rollback, and so would any older one.
try {
  fleet.stage(envelope, image)
  console.log('an old release was accepted, which should never happen')
} catch (error) {
  console.log(`old       refused: ${(error as Error).message}`)
}

// The next release goes to the slot the device is not running, slot 0 now. An image damaged
// on the way still arrives in full, but it does not hash to what was signed.
const upgrade = Buffer.from('firmware for a flow meter, version three')
const third = { ...manifest, sequence: 3, storage: 0, digest: imageDigest(upgrade), size: upgrade.length }
const release = signManifest(third, publisher)
const damaged = Buffer.from(upgrade)
damaged[0] ^= 0xff
try {
  fleet.stage(release, damaged)
  console.log('a damaged image was accepted, which should never happen')
} catch (error) {
  console.log(`corrupt   refused: ${(error as Error).message}`)
}

// The same release signed by a key this device is not anchored to gets nowhere.
const impostor = DeviceIdentity.fromSeed(Buffer.alloc(32, 90))
try {
  fleet.stage(signManifest(third, impostor), upgrade)
  console.log('a forged release was accepted, which should never happen')
} catch (error) {
  console.log(`forged    refused: ${(error as Error).message}`)
}

// The genuine release stages and boots on trial, but never confirms: the next boot fails it
// and goes back to the image that worked.
fleet.stage(release, upgrade)
const trial = fleet.onBoot()
console.log(`booting   ${said(trial)}, running sequence ${third.sequence}`)
const after = fleet.onBoot()
console.log(`reverted  ${said(after)}`)

// A release that failed cannot be offered again, or a captured image could be replayed; the
// fix goes out as sequence 4.
try {
  fleet.stage(release, upgrade)
  console.log('a failed release was accepted again, which should never happen')
} catch (error) {
  console.log(`again     refused: ${(error as Error).message}`)
}
// ANCHOR_END: example

assert.deepEqual(manifest.digest, imageDigest(image))
assert.deepEqual(opened.digest, manifest.digest)
assert.equal(slot, 1)
assert.equal(decision.action, BootAction.Trying)
assert.equal(fleet.slotRecord(1).state, SlotState.Confirmed)
assert.equal(trial.action, BootAction.Trying)
assert.equal(trial.slot, 0)
assert.equal(after.action, BootAction.Reverted)
assert.equal(after.slot, 0)
assert.equal(after.fallback, 1)
assert.equal(fleet.slotRecord(0).state, SlotState.Failed)
assert.equal(fleet.installedSequence, 3)
