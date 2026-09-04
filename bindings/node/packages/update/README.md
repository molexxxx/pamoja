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
import assert from 'node:assert/strict'

import { DeviceIdentity } from '@pamoja/security'
import {
  BootAction,
  FORMAT_RAW,
  STRUCTURE_VERSION,
  SlotState,
  Updater,
  signManifest,
  verifyEnvelope,
} from '@pamoja/update'

const vendor = Buffer.alloc(16, 0x0a)
const deviceClass = Buffer.alloc(16, 0x0b)
const publisher = DeviceIdentity.fromSeed(Buffer.alloc(32, 0x31))

// The image stands in for firmware. It is the 56-byte message FIPS 180-4 hashes in its
// second worked example, so the digest the manifest commits to is a published constant.
const image = Buffer.from('abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq')
const digest = Buffer.from(
  '248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1',
  'hex'
)

// A release says who it is for, which slot it belongs in, and what it hashes to. The
// publisher signs that statement; nothing else about the image is taken on trust.
const manifest = {
  structureVersion: STRUCTURE_VERSION,
  sequence: 2,
  vendorId: vendor,
  classId: deviceClass,
  format: FORMAT_RAW,
  storage: 1,
  digest,
  size: image.length,
  expires: 0,
}
const envelope = signManifest(manifest, publisher)
assert.deepEqual(verifyEnvelope(envelope, publisher.publicKey()).digest, digest)

// The device left the factory running sequence 1 from slot 0, so the release goes to
// the spare slot and the image it runs today stays where it is.
const fleet = new Updater(vendor, deviceClass, publisher.publicKey(), 2, 4096)
fleet.provision(0, 1)
assert.equal(fleet.begin(envelope), 1)
for (let at = 0; at < image.length; at += 16) {
  fleet.write(image.subarray(at, at + 16))
}
assert.equal(fleet.progress().written, image.length)
assert.equal(fleet.finish(), 1)

// The first boot into a new image is a trial. It reverts to slot 0 on the next boot
// unless it confirms itself.
assert.equal(fleet.onBoot().action, BootAction.Trying)
assert.equal(fleet.confirm(), 1)
assert.equal(fleet.slotRecord(1).state, SlotState.Confirmed)

// The same release, signed by a key this device is not anchored to, gets nowhere.
const impostor = DeviceIdentity.fromSeed(Buffer.alloc(32, 0x32))
assert.throws(() => fleet.stage(signManifest(manifest, impostor), image))
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
