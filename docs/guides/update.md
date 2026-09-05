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

It signs a manifest for an image, checks the envelope on the device against the
key that device is anchored to, stages the image into the spare slot while the
running one is left alone, and confirms it after a trial boot. Then it offers the
same release signed by a key the device does not trust.

The manifest commits to a SHA-256 over the image, and the library computes it, so
a publisher does not add a hashing dependency just to name the image it is
releasing.

It proves:

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
    image_digest, Device, Envelope, Manifest, MemoryStore, PayloadFormat, SlotState, SlotStore,
    Updater, ENVELOPE_MAX, STRUCTURE_VERSION,
};

// The publisher's key signs releases; devices in the field are anchored to its public
// half and will take firmware from nobody else.
let publisher = DeviceIdentity::from_seed(&[7u8; 32]);

// The release. A manifest says who the image is for, which slot it belongs in, how big
// it is and what it hashes to; nothing about the image itself is taken on trust.
let image = b"firmware for a flow meter, version two";
let manifest = Manifest {
    structure_version: STRUCTURE_VERSION,
    sequence: 2,
    vendor_id: [0x0A; 16],
    class_id: [0x0B; 16],
    format: PayloadFormat::Raw,
    storage: 1,
    digest: image_digest(image),
    size: image.len() as u32,
    expires: 0,
};

// Signing it produces the envelope that travels with the image.
let mut buf = [0u8; ENVELOPE_MAX];
let written = manifest
    .sign(&publisher, &mut buf)
    .expect("a signed release");
let envelope = &buf[..written];
let sequence = manifest.sequence;
println!("published sequence {sequence} in a {written}-byte envelope");

// On the device. It checks the envelope against the key it was anchored to before it
// accepts a single byte of the image.
let device = Device {
    vendor_id: manifest.vendor_id,
    class_id: manifest.class_id,
    anchor: publisher.public(),
};
let opened = Envelope::decode(envelope).expect("a well-formed envelope");
match opened.verify(&device.anchor) {
    Ok(release) => println!("accepted  a release for slot {}", release.storage),
    Err(error) => println!("refused   {error}"),
}

// It left the factory running sequence 1 from slot 0, so the release goes to the spare
// slot and the image it is running stays where it is.
let mut updater = Updater::new(device, MemoryStore::new(2, 4096));
updater.provision(0, 1).expect("the shipped image");
let mut staging = updater.begin(envelope).expect("a release for this device");
for piece in image.chunks(16) {
    staging.write(piece).expect("the next piece");
}
let (received, total) = staging.progress();
println!("staged    {received} of {total} bytes");
let slot = staging.finish().expect("the image matched its digest");
println!("written   to slot {slot}, leaving the running image alone");

// The first boot into a new image is a trial. It reverts on the next boot unless the
// device confirms that it came up, which is what makes a bad release survivable.
println!("booting   {:?}", updater.on_boot().expect("a decision"));
updater.confirm().expect("it came up");
let state = updater.store().record(slot).expect("the new slot").state;
println!("confirmed slot {slot} is now {state:?}");

// The same release signed by a key this device is not anchored to gets nowhere.
let impostor = DeviceIdentity::from_seed(&[90u8; 32]);
let mut forged = [0u8; ENVELOPE_MAX];
let signed = manifest
    .sign(&impostor, &mut forged)
    .expect("a signed release");
match updater.stage(&forged[..signed], image) {
    Ok(_) => println!("a forged release was accepted, which should never happen"),
    Err(error) => println!("forged    refused: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/update.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/update.py#example -->
From [`bindings/python/guides/update.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/update.py):

```python
from pamoja.core import PamojaError
from pamoja.security import DeviceIdentity
from pamoja.update import (
    BootAction,
    Manifest,
    SlotState,
    Updater,
    image_digest,
    sign_manifest,
    verify_envelope,
)

# The publisher's key signs releases; devices in the field are anchored to its public half
# and will take firmware from nobody else.
publisher = DeviceIdentity.from_seed(bytes([7]) * 32)
vendor = bytes([0x0A]) * 16
device_class = bytes([0x0B]) * 16

# The release. A manifest says who the image is for, which slot it belongs in, how big it
# is and what it hashes to; nothing about the image itself is taken on trust.
image = b"firmware for a flow meter, version two"
manifest = Manifest(
    sequence=2,
    vendor_id=vendor,
    class_id=device_class,
    storage=1,
    digest=image_digest(image),
    size=len(image),
)
envelope = sign_manifest(manifest, publisher)
print(f"published sequence {manifest.sequence} in a {len(envelope)}-byte envelope")

# On the device. It checks the envelope against the key it was anchored to before it
# accepts a single byte of the image.
opened = verify_envelope(envelope, publisher.public_key)
print(f"accepted  a release for slot {opened.storage}")

# It left the factory running sequence 1 from slot 0, so the release goes to the spare slot
# and the image it is running stays where it is.
fleet = Updater(vendor, device_class, publisher.public_key, 2, 4096)
fleet.provision(0, 1)
fleet.begin(envelope)
for at in range(0, len(image), 16):
    fleet.write(image[at : at + 16])
print(f"staged    {fleet.progress().written} of {len(image)} bytes")
slot = fleet.finish()
print(f"written   to slot {slot}, leaving the running image alone")

# The first boot into a new image is a trial. It reverts on the next boot unless the device
# confirms that it came up, which is what makes a bad release survivable.
print(f"booting   {fleet.on_boot().action}")
fleet.confirm()
print(f"confirmed slot {slot} is now {fleet.slot_record(slot).state}")

# The same release signed by a key this device is not anchored to gets nowhere.
impostor = DeviceIdentity.from_seed(bytes([90]) * 32)
try:
    fleet.stage(sign_manifest(manifest, impostor), image)
    print("a forged release was accepted, which should never happen")
except PamojaError as error:
    print(f"forged    refused: {error}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/UpdateGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/UpdateGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/UpdateGuide.cs):

```csharp
// The publisher's key signs releases; devices in the field are anchored to its
// public half and will take firmware from nobody else.
byte[] seed = new byte[32];
Array.Fill(seed, (byte)7);
using var publisher = new DeviceIdentity(seed);
byte[] vendor = Enumerable.Repeat((byte)0x0A, 16).ToArray();
byte[] deviceClass = Enumerable.Repeat((byte)0x0B, 16).ToArray();

// The release. A manifest says who the image is for, which slot it belongs in, how
// big it is and what it hashes to; nothing about the image is taken on trust.
byte[] image = Encoding.ASCII.GetBytes("firmware for a flow meter, version two");
var manifest = new Manifest(
    Sequence: 2,
    VendorId: vendor,
    ClassId: deviceClass,
    Storage: 1,
    Digest: Update.ImageDigest(image),
    Size: (uint)image.Length);
byte[] envelope = Update.SignManifest(manifest, publisher);
Console.WriteLine(
    $"published sequence {manifest.Sequence} in a {envelope.Length}-byte envelope");

// On the device. It checks the envelope against the key it was anchored to before
// it accepts a single byte of the image.
Manifest opened = Update.VerifyEnvelope(envelope, publisher.PublicKey);
Console.WriteLine($"accepted  a release for slot {opened.Storage}");

// It left the factory running sequence 1 from slot 0, so the release goes to the
// spare slot and the image it is running stays where it is.
using var fleet = new Updater(vendor, deviceClass, publisher.PublicKey, 2, 4096);
fleet.Provision(0, 1);
fleet.Begin(envelope);
for (int at = 0; at < image.Length; at += 16)
{
    fleet.Write(image.AsSpan(at, Math.Min(16, image.Length - at)));
}

Console.WriteLine($"staged    {fleet.CurrentProgress().Written} of {image.Length} bytes");
byte slot = fleet.Finish();
Console.WriteLine($"written   to slot {slot}, leaving the running image alone");

// The first boot into a new image is a trial. It reverts on the next boot unless
// the device confirms it came up, which is what makes a bad release survivable.
Console.WriteLine($"booting   {fleet.OnBoot().Action}");
fleet.Confirm();
Console.WriteLine($"confirmed slot {slot} is now {fleet.Record(slot).State}");

// The same release signed by a key this device is not anchored to gets nowhere.
byte[] impostorSeed = new byte[32];
Array.Fill(impostorSeed, (byte)90);
using var impostor = new DeviceIdentity(impostorSeed);
try
{
    fleet.Stage(Update.SignManifest(manifest with { Sequence = 3 }, impostor), image);
    Console.WriteLine("a forged release was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"forged    refused: {error.Message}");
}
```
<!-- end -->

## Reference

<!-- table: reference update -->
- Rust: [`pamoja-update`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html)
- TypeScript: [`@pamoja/update`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html)
- Python: [`pamoja.update`](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html)
- C#: [`Pamoja.Update`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html)
<!-- end -->
