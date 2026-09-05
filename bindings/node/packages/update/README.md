# @pamoja/update

Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/update.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
console.log(`booting   ${fleet.onBoot().action}`)
fleet.confirm()
console.log(`confirmed slot ${slot} is now ${fleet.slotRecord(slot).state}`)

// The same release signed by a key this device is not anchored to gets nowhere.
const impostor = DeviceIdentity.fromSeed(Buffer.alloc(32, 90))
try {
  fleet.stage(signManifest(manifest, impostor), image)
  console.log('a forged release was accepted, which should never happen')
} catch (error) {
  console.log(`forged    refused: ${(error as Error).message}`)
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-update`](https://crates.io/crates/pamoja-update) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html), [docs.rs](https://docs.rs/pamoja-update) |
| TypeScript | [`@pamoja/update`](https://www.npmjs.com/package/@pamoja/update) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html) |
| Python | [`pamoja-update`](https://pypi.org/project/pamoja-update/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html) |
| C# | [`Pamoja.Update`](https://www.nuget.org/packages/Pamoja.Update) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html) |

## Documentation

- [`@pamoja/update` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html), every class, function, and type this package exports.
- [The Signed updates guide](https://pamoja.molex.cloud/docs/guides/update.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
