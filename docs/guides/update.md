# Signed updates

A device in the field has to be fixable, and the thing that fixes it is the most
dangerous input it will ever accept. pamoja treats a release as a signed
statement about an image rather than as the image itself: a manifest names the
devices it is for, the slot it belongs in, and the digest the image must hash to,
and the publisher signs that. The device verifies the signature against a key it
was anchored to, streams the image while hashing it, and refuses anything whose
digest does not match what was signed.

An update also has to survive being wrong. The image is written to the slot the
device is not running from, and the first boot into it is a trial: unless the new
image confirms itself, the next boot goes back to the one that worked.

## What the example does

It signs a manifest for an image, stages it into the spare slot while the running
one is left alone, and confirms it after a trial boot. Then it offers the same
release signed by a key the device is not anchored to.

It proves:

- The digest the manifest commits to is the one FIPS 180-4 publishes for its
  second worked example, so the hash is checked against a published constant
  rather than against itself.
- A verified envelope carries back the digest that was signed.
- The release lands in the slot the device is not running from, so the working
  image is never overwritten.
- A first boot into a new image reports itself as a trial, and confirming it is
  what makes it permanent.
- A release signed by an untrusted key is refused.

## Rust

<!-- snippet: examples/tests/guides/update.rs#example -->
From [`examples/tests/guides/update.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/update.rs):

```rust
use pamoja_security::DeviceIdentity;
use pamoja_update::{
    Boot, Device, Envelope, Manifest, MemoryStore, PayloadFormat, SlotState, SlotStore,
    Updater, ENVELOPE_MAX, STRUCTURE_VERSION,
};

let publisher = DeviceIdentity::from_seed(&[0x31; 32]);

// The image stands in for firmware. It is the 56-byte message FIPS 180-4 hashes in its
// second worked example, so the digest the manifest commits to is a published constant.
let image = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
let size = image.len() as u32;
let manifest = Manifest {
    structure_version: STRUCTURE_VERSION,
    sequence: 2,
    vendor_id: [0x0A; 16],
    class_id: [0x0B; 16],
    format: PayloadFormat::Raw,
    storage: 1,
    digest: [
        0x24, 0x8d, 0x6a, 0x61, 0xd2, 0x06, 0x38, 0xb8, 0xe5, 0xc0, 0x26, 0x93, 0x0c, 0x3e,
        0x60, 0x39, 0xa3, 0x3c, 0xe4, 0x59, 0x64, 0xff, 0x21, 0x67, 0xf6, 0xec, 0xed, 0xd4,
        0x19, 0xdb, 0x06, 0xc1,
    ],
    size,
    expires: 0,
};

// A release says who it is for, which slot it belongs in, and what it hashes to. The
// publisher signs that statement; nothing else about the image is taken on trust.
let mut buf = [0u8; ENVELOPE_MAX];
let written = manifest
    .sign(&publisher, &mut buf)
    .expect("a signed release");
let envelope = &buf[..written];
let opened = Envelope::decode(envelope).expect("a well-formed envelope");
assert_eq!(
    opened.verify(&publisher.public()).expect("the signature"),
    manifest
);

// The device left the factory running sequence 1 from slot 0, so the release goes to the
// spare slot and the image it runs today stays where it is.
let device = Device {
    vendor_id: manifest.vendor_id,
    class_id: manifest.class_id,
    anchor: publisher.public(),
};
let mut updater = Updater::new(device, MemoryStore::new(2, 4096));
updater.provision(0, 1).expect("the shipped image");
let mut staging = updater.begin(envelope).expect("a release for this device");
for piece in image.chunks(16) {
    staging.write(piece).expect("the next piece");
}
assert_eq!(staging.progress(), (size, size));
assert_eq!(staging.finish().expect("the image matched its digest"), 1);

// The first boot into a new image is a trial. It reverts to slot 0 on the next boot
// unless it confirms itself.
assert_eq!(updater.on_boot().expect("a decision"), Boot::Trying(1));
assert_eq!(updater.confirm().expect("it came up"), 1);
assert_eq!(
    updater.store().record(1).expect("slot 1").state,
    SlotState::Confirmed
);

// The same release, signed by a key this device is not anchored to, gets nowhere.
let impostor = DeviceIdentity::from_seed(&[0x32; 32]);
let mut forged = [0u8; ENVELOPE_MAX];
let signed = manifest
    .sign(&impostor, &mut forged)
    .expect("a signed release");
assert!(updater.stage(&forged[..signed], image).is_err());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/update.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/update.py#example -->
From [`bindings/python/guides/update.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/update.py):

```python
from pamoja.core import PamojaError
from pamoja.security import DeviceIdentity
from pamoja.update import (
    BootAction, Manifest, SlotState, Updater, sign_manifest, verify_envelope,
)

vendor = bytes([0x0A]) * 16
device_class = bytes([0x0B]) * 16
publisher = DeviceIdentity.from_seed(bytes([0x31]) * 32)

# The image stands in for firmware. It is the 56-byte message FIPS 180-4 hashes in its
# second worked example, so the digest the manifest commits to is a published constant.
image = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
digest = bytes.fromhex(
    "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
)

# A release says who it is for, which slot it belongs in, and what it hashes to. The
# publisher signs that statement; nothing else about the image is taken on trust.
manifest = Manifest(
    sequence=2, vendor_id=vendor, class_id=device_class, storage=1, digest=digest,
    size=len(image),
)
envelope = sign_manifest(manifest, publisher)
assert verify_envelope(envelope, publisher.public_key).digest == digest

# The device left the factory running sequence 1 from slot 0, so the release goes to
# the spare slot and the image it runs today stays where it is.
fleet = Updater(vendor, device_class, publisher.public_key, 2, 4096)
fleet.provision(0, 1)
assert fleet.begin(envelope) == 1
for at in range(0, len(image), 16):
    fleet.write(image[at : at + 16])
assert fleet.progress().written == len(image)
assert fleet.finish() == 1

# The first boot into a new image is a trial. It reverts to slot 0 on the next boot
# unless it confirms itself.
assert fleet.on_boot().action == BootAction.TRYING
assert fleet.confirm() == 1
assert fleet.slot_record(1).state == SlotState.CONFIRMED

# The same release, signed by a key this device is not anchored to, gets nowhere.
impostor = DeviceIdentity.from_seed(bytes([0x32]) * 32)
try:
    fleet.stage(sign_manifest(manifest, impostor), image)
except PamojaError:
    pass
else:
    raise AssertionError("a release signed by an untrusted key should be refused")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/UpdateGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/UpdateGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/UpdateGuide.cs):

```csharp
byte[] vendor = Enumerable.Repeat((byte)0x0A, 16).ToArray();
byte[] deviceClass = Enumerable.Repeat((byte)0x0B, 16).ToArray();
using var publisher = new DeviceIdentity(Enumerable.Repeat((byte)0x31, 32).ToArray());

// The image stands in for firmware. It is the 56-byte message FIPS 180-4 hashes in
// its second worked example, so the digest the manifest commits to is a published
// constant rather than a value checked against itself.
byte[] image = Encoding.ASCII.GetBytes(
    "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
byte[] digest = Convert.FromHexString(
    "248D6A61D20638B8E5C026930C3E6039A33CE45964FF2167F6ECEDD419DB06C1");

// A release says who it is for, which slot it belongs in, and what it hashes to. The
// publisher signs that statement; nothing else about the image is taken on trust.
var manifest = new Manifest(
    Sequence: 2,
    VendorId: vendor,
    ClassId: deviceClass,
    Storage: 1,
    Digest: digest,
    Size: (uint)image.Length);
byte[] envelope = Update.SignManifest(manifest, publisher);
Expect(
    Update.VerifyEnvelope(envelope, publisher.PublicKey).Digest.SequenceEqual(digest),
    "the release verifies against the key that signed it");

// The device left the factory running sequence 1 from slot 0, so the release goes to
// the spare slot and the image it runs today stays where it is.
using var fleet = new Updater(vendor, deviceClass, publisher.PublicKey, 2, 4096);
fleet.Provision(0, 1);
Expect(fleet.Begin(envelope) == 1, "the release names the spare slot");
for (int at = 0; at < image.Length; at += 16)
{
    fleet.Write(image.AsSpan(at, Math.Min(16, image.Length - at)));
}
Expect(fleet.CurrentProgress().Written == image.Length, "every byte arrived");
Expect(fleet.Finish() == 1, "and the image matched what was promised");

// The first boot into a new image is a trial. It reverts to slot 0 on the next boot
// unless it confirms itself.
Expect(fleet.OnBoot().Action == BootAction.Trying, "a new image is on trial");
Expect(fleet.Confirm() == 1, "and confirms once it has run");
Expect(fleet.Record(1).State == SlotState.Confirmed, "so the slot holds it from now on");

// The same release, signed by a key this device is not anchored to, gets nowhere.
using var impostor = new DeviceIdentity(Enumerable.Repeat((byte)0x32, 32).ToArray());
bool refused = false;
try
{
    fleet.Stage(Update.SignManifest(manifest with { Sequence = 3 }, impostor), image);
}
catch (PamojaException)
{
    refused = true;
}
Expect(refused, "a release signed by an untrusted key is refused");
```
<!-- end -->

## Reference

<!-- table: reference update -->
- Rust: [`pamoja-update`](https://docs.rs/pamoja-update) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html))
- TypeScript: [`@pamoja/update`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html)
- Python: [`pamoja.update`](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html)
- C#: [`Update`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.Update.html), [`Updater`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.Updater.html), [`Manifest`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.Manifest.html), [`ImageVerifier`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.ImageVerifier.html), [`Progress`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.Progress.html), [`Delegation`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.Delegation.html), [`SlotRecord`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.SlotRecord.html), [`SlotState`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.SlotState.html), [`BootDecision`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.BootDecision.html), [`BootAction`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.BootAction.html)
<!-- end -->
