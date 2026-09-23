# @pamoja/update

Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/update.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html)

## Install

```sh
npm install @pamoja/update
```

This pulls in `@pamoja/native`, the compiled engine, and `@pamoja/security`. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/update.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/update.ts):

```typescript
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-update`](https://crates.io/crates/pamoja-update) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html), [docs.rs](https://docs.rs/pamoja-update), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-update) |
| TypeScript | [`@pamoja/update`](https://www.npmjs.com/package/@pamoja/update) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-update) |
| Python | [`pamoja-update`](https://pypi.org/project/pamoja-update/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-update) |
| C# | [`Pamoja.Update`](https://www.nuget.org/packages/Pamoja.Update) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-update) |

## Documentation

- [`@pamoja/update` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html), every class, function, and type this package exports.
- [The Signed updates guide](https://pamoja.molex.cloud/docs/guides/update.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
