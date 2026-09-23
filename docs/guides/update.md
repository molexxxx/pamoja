# Signed updates

A device in the field has to be fixable, and the thing that fixes it is the most
dangerous input it will ever accept. pamoja treats a release as a signed
statement about an image rather than as the image itself: a manifest names the
devices it is for, the slot it belongs in, its size, and the digest the image
must hash to, and the publisher signs that. The device checks the signature
against a key it was anchored to, hashes the image as it streams in, and refuses
anything whose digest does not match what was signed.

An update also has to survive being wrong. The image is written to the slot the
device is not running from, and the first boot into it is a trial: unless the
new image confirms itself, the next boot goes back to the one that worked. Slots
are never swapped, which is the arrangement MCUboot calls direct-XIP, so a power
cut in the middle of an update leaves no half-moved image to recover.

## What the example does

A flow meter leaves the factory running sequence 1 from slot 0. The example signs
sequence 2 for slot 1, checks the envelope against the key the meter is anchored
to, and streams the image in sixteen-byte pieces, the way it would arrive over a
slow link. The first boot into slot 1 is a trial, and the meter confirms it,
which erases slot 0 and frees it for the next release.

Then come the releases a device has to turn away. Sequence 2 offered again is
refused as a rollback. Sequence 3, signed properly and bound for slot 0, arrives
with its first byte damaged and fails its digest. The same manifest signed by a
key the meter does not trust fails before anything in it is read.

Last, the genuine sequence 3 stages into slot 0 and boots on trial, but never
confirms, so the next boot fails it and the meter runs slot 1 again. Offering
sequence 3 a second time is refused too: a release that failed has spent its
sequence number, and the fix goes out as sequence 4.

The damaged image is not typed out by hand. It is the release's own image with
its first byte flipped, so it is the right size and only its hash is wrong.

It proves:

- Verifying the envelope hands back the manifest, so the device learns which
  slot the release is for from the signature rather than from whoever sent it.
- Staging completes only because every byte the manifest declared arrived and
  hashed to its digest, which the device works out as the pieces come in.
- The release lands in the slot the device is not running from, so the working
  image is never overwritten.
- The first boot into a staged image is a trial, and confirming it is what
  leaves the slot confirmed.
- A release no newer than what the device holds is refused, even one the device
  has just installed.
- An image that arrives whole but damaged is refused at the end, and the slot it
  was written into never becomes bootable.
- A release signed by another key is refused before its manifest is read.
- An image that boots and never confirms is failed on the next boot, and the
  device goes back to the slot that worked.
- A failed release keeps its sequence number spent, so a captured copy of it
  cannot be offered again.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example update" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example update</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- update" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- update</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/update.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/update.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- update" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- update</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-update` serves both ends. A publisher fills in a `Manifest`,
hashes the image with `image_digest`, and signs it with a `DeviceIdentity` from
`pamoja-security` into a buffer of `ENVELOPE_MAX` bytes. On the device,
`Envelope::decode` and `verify` check an envelope on their own, and an `Updater`
applies every rule against a `SlotStore`. `begin` opens a `Staging` that takes
the image through `write` and settles it with `finish`, `resume_at` continues a
transfer a reset cut off, and `stage` does all of it for an image held whole.
`begin` and `stage` have `_at` forms that take the time, for a release that
expires, and `resume_at` always takes it. `on_boot` returns a `Boot`, `confirm`
and `revert` settle a trial, and `installed_sequence` is the number a release
has to beat. Every refusal is a `Refusal`, which turns into the core `Error` as
`Auth`, `Codec`, or `Io`. `MemoryStore` keeps slots in memory; on a device,
`SlotStore` is the seam to flash, and the records it keeps must survive a
reboot.

<!-- snippet: examples/guides/update.rs#example -->
From [`examples/guides/update.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/update.rs):

```rust
use pamoja_security::DeviceIdentity;
use pamoja_update::{
    image_digest, Boot, Device, Envelope, Manifest, MemoryStore, PayloadFormat, SlotState,
    SlotStore, Updater, ENVELOPE_MAX, STRUCTURE_VERSION,
};

// The publisher's key signs releases; devices in the field are anchored to its public
// half and will take firmware from nobody else.
let publisher = DeviceIdentity::from_seed(&[7u8; 32]);

// Who the release is for. Both identifiers are sixteen bytes a vendor assigns itself,
// and a device takes firmware only for the pair it was built as.
const VENDOR: [u8; 16] = [10; 16];
const FLOW_METER: [u8; 16] = [11; 16];

// The release. A manifest says who the image is for, which slot it belongs in, how big
// it is and what it hashes to; nothing about the image itself is taken on trust.
let image = b"firmware for a flow meter, version two";
let manifest = Manifest {
    structure_version: STRUCTURE_VERSION,
    sequence: 2,
    vendor_id: VENDOR,
    class_id: FLOW_METER,
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
let said = |decision: Boot| match decision {
    Boot::Trying(slot) => format!("slot {slot} on trial"),
    Boot::Confirmed(slot) => format!("slot {slot}, already confirmed"),
    Boot::Reverted { failed, fallback } => {
        format!("slot {failed} never confirmed, so the device runs slot {fallback} again")
    }
};
let decision = updater.on_boot().expect("a decision");
println!("booting   {}", said(decision));
updater.confirm().expect("it came up");
let state = updater.store().record(slot).expect("the new slot").state;
println!("confirmed slot {slot} is now {state:?}");

// The same release offered again would take the device nowhere new, so it is refused
// as a rollback, and so would any older one.
match updater.stage(envelope, image) {
    Ok(_) => println!("an old release was accepted, which should never happen"),
    Err(error) => println!("old       refused: {error}"),
}

// The next release goes to the slot the device is not running, slot 0 now. An image
// damaged on the way still arrives in full, but it does not hash to what was signed.
let upgrade = b"firmware for a flow meter, version three";
let third = Manifest {
    sequence: 3,
    storage: 0,
    digest: image_digest(upgrade),
    size: upgrade.len() as u32,
    ..manifest
};
let mut buf = [0u8; ENVELOPE_MAX];
let written = third.sign(&publisher, &mut buf).expect("a signed release");
let release = &buf[..written];
let mut damaged = upgrade.to_vec();
damaged[0] ^= 0xFF;
match updater.stage(release, &damaged) {
    Ok(_) => println!("a damaged image was accepted, which should never happen"),
    Err(error) => println!("corrupt   refused: {error}"),
}

// The same release signed by a key this device is not anchored to gets nowhere.
let impostor = DeviceIdentity::from_seed(&[90u8; 32]);
let mut forged = [0u8; ENVELOPE_MAX];
let signed = third
    .sign(&impostor, &mut forged)
    .expect("a signed release");
match updater.stage(&forged[..signed], upgrade) {
    Ok(_) => println!("a forged release was accepted, which should never happen"),
    Err(error) => println!("forged    refused: {error}"),
}

// The genuine release stages and boots on trial, but never confirms: the next boot
// fails it and goes back to the image that worked.
updater
    .stage(release, upgrade)
    .expect("the genuine release");
let trial = updater.on_boot().expect("a decision");
println!(
    "booting   {}, running sequence {}",
    said(trial),
    third.sequence
);
let after = updater.on_boot().expect("a decision");
println!("reverted  {}", said(after));

// A release that failed cannot be offered again, or a captured image could be
// replayed; the fix goes out as sequence 4.
match updater.stage(release, upgrade) {
    Ok(_) => println!("a failed release was accepted again, which should never happen"),
    Err(error) => println!("again     refused: {error}"),
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/update` exports `signManifest(manifest, identity)`,
which fills in the structure version, the payload format, and a zero expiry when
a manifest leaves them out, along with `imageDigest` and
`verifyEnvelope(envelope, publicKey)`. An `Updater` is built from the vendor and
class identifiers, the anchor's public key, and the slot count and capacity, and
it keeps its slots in memory. `begin`, `write`, and `finish` take an image in
pieces, carrying the hash from one call to the next, and `stage` takes one held
whole; both accept the time in seconds as a last argument. `onBoot()` returns a
`Boot` of `action`, `slot`, and `fallback`, and `BootAction` and `SlotState` are
runtime objects to compare against. A refusal throws an `Error` whose message is
the reason.

<!-- snippet: bindings/node/guides/update.ts#example -->
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
<!-- end -->

## Python

In Python, `pamoja.update` has the same pieces in snake case: `sign_manifest`,
`image_digest`, `verify_envelope`, and a `Manifest` whose `expires`, `format`,
and `structure_version` default to 0, `FORMAT_RAW`, and `STRUCTURE_VERSION`. An
`Updater` takes the identifiers, the anchor's public key, and the slot count and
capacity. `begin`, `write`, and `finish` stream an image and `stage` takes one
whole, each with a `now=` keyword. `on_boot()` returns a `BootDecision` of
`action`, `slot`, and `fallback`, and `installed_sequence` is a property.
`BootAction` and `SlotState` are string enums, so a decision's `action` compares
equal to `BootAction.TRYING`. A refusal raises `PamojaError` with the reason.

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
def said(decision):
    if decision.action == BootAction.TRYING:
        return f"slot {decision.slot} on trial"
    if decision.action == BootAction.CONFIRMED:
        return f"slot {decision.slot}, already confirmed"
    return f"slot {decision.slot} never confirmed, so the device runs slot {decision.fallback} again"


decision = fleet.on_boot()
print(f"booting   {said(decision)}")
fleet.confirm()
print(f"confirmed slot {slot} is now {fleet.slot_record(slot).state}")

# The same release offered again would take the device nowhere new, so it is refused as a
# rollback, and so would any older one.
try:
    fleet.stage(envelope, image)
    print("an old release was accepted, which should never happen")
except PamojaError as error:
    print(f"old       refused: {error}")

# The next release goes to the slot the device is not running, slot 0 now. An image damaged
# on the way still arrives in full, but it does not hash to what was signed.
upgrade = b"firmware for a flow meter, version three"
third = Manifest(
    sequence=3,
    vendor_id=vendor,
    class_id=device_class,
    storage=0,
    digest=image_digest(upgrade),
    size=len(upgrade),
)
release = sign_manifest(third, publisher)
damaged = bytes([upgrade[0] ^ 0xFF]) + upgrade[1:]
try:
    fleet.stage(release, damaged)
    print("a damaged image was accepted, which should never happen")
except PamojaError as error:
    print(f"corrupt   refused: {error}")

# The same release signed by a key this device is not anchored to gets nowhere.
impostor = DeviceIdentity.from_seed(bytes([90]) * 32)
try:
    fleet.stage(sign_manifest(third, impostor), upgrade)
    print("a forged release was accepted, which should never happen")
except PamojaError as error:
    print(f"forged    refused: {error}")

# The genuine release stages and boots on trial, but never confirms: the next boot fails it
# and goes back to the image that worked.
fleet.stage(release, upgrade)
trial = fleet.on_boot()
print(f"booting   {said(trial)}, running sequence {third.sequence}")
after = fleet.on_boot()
print(f"reverted  {said(after)}")

# A release that failed cannot be offered again, or a captured image could be replayed; the
# fix goes out as sequence 4.
try:
    fleet.stage(release, upgrade)
    print("a failed release was accepted again, which should never happen")
except PamojaError as error:
    print(f"again     refused: {error}")
```
<!-- end -->

## C#

In C#, `Pamoja.Update` puts the publisher's calls on the static `Update` class:
`SignManifest(manifest, identity)`, `ImageDigest`, and `VerifyEnvelope`.
`Manifest` is a record whose `Expires`, `Format`, and `StructureVersion` have
defaults, so `with` builds the next release from the last one. An `Updater`
takes the identifiers, the anchor's public key, and the slot count and capacity,
and is `IDisposable`. `Begin`, `Write`, and `Finish` stream an image and `Stage`
takes one whole, each with an optional `now`. `OnBoot()` returns a
`BootDecision` record struct of `Action`, `Slot`, and `Fallback`, `Record(slot)`
a `SlotRecord`, and `CurrentProgress()` a `Progress`. A refusal throws
`PamojaException` with the reason as its message.

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
static string Said(BootDecision decision) => decision.Action switch
{
    BootAction.Trying => $"slot {decision.Slot} on trial",
    BootAction.Confirmed => $"slot {decision.Slot}, already confirmed",
    _ => $"slot {decision.Slot} never confirmed, so the device runs slot {decision.Fallback} again",
};
BootDecision decision = fleet.OnBoot();
Console.WriteLine($"booting   {Said(decision)}");
fleet.Confirm();
Console.WriteLine($"confirmed slot {slot} is now {fleet.Record(slot).State}");

// The same release offered again would take the device nowhere new, so it is
// refused as a rollback, and so would any older one.
try
{
    fleet.Stage(envelope, image);
    Console.WriteLine("an old release was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"old       refused: {error.Message}");
}

// The next release goes to the slot the device is not running, slot 0 now. An
// image damaged on the way still arrives in full, but it does not hash to what was
// signed.
byte[] upgrade = Encoding.ASCII.GetBytes("firmware for a flow meter, version three");
Manifest third = manifest with
{
    Sequence = 3,
    Storage = 0,
    Digest = Update.ImageDigest(upgrade),
    Size = (uint)upgrade.Length,
};
byte[] release = Update.SignManifest(third, publisher);
byte[] damaged = (byte[])upgrade.Clone();
damaged[0] ^= 0xFF;
try
{
    fleet.Stage(release, damaged);
    Console.WriteLine("a damaged image was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"corrupt   refused: {error.Message}");
}

// The same release signed by a key this device is not anchored to gets nowhere.
byte[] impostorSeed = new byte[32];
Array.Fill(impostorSeed, (byte)90);
using var impostor = new DeviceIdentity(impostorSeed);
try
{
    fleet.Stage(Update.SignManifest(third, impostor), upgrade);
    Console.WriteLine("a forged release was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"forged    refused: {error.Message}");
}

// The genuine release stages and boots on trial, but never confirms: the next boot
// fails it and goes back to the image that worked.
fleet.Stage(release, upgrade);
BootDecision trial = fleet.OnBoot();
Console.WriteLine($"booting   {Said(trial)}, running sequence {third.Sequence}");
BootDecision after = fleet.OnBoot();
Console.WriteLine($"reverted  {Said(after)}");

// A release that failed cannot be offered again, or a captured image could be
// replayed; the fix goes out as sequence 4.
try
{
    fleet.Stage(release, upgrade);
    Console.WriteLine("a failed release was accepted again, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"again     refused: {error.Message}");
}
```
<!-- end -->

## Values at a glance

**What a manifest holds,** in the order it is encoded:

| Field | Type | What it is |
| --- | --- | --- |
| structure version | 1 in this build | the manifest format's revision; a newer one is refused |
| sequence | unsigned 64-bit | rises with every release, and has to beat every slot the device holds, failed ones included |
| vendor id | 16 bytes | who built the image |
| class id | 16 bytes | which kind of device it is for |
| format | 1, raw | how the payload is encoded; raw is the image byte for byte |
| storage | slot number | which slot the image belongs in |
| digest | 32 bytes | the SHA-256 of the image |
| size | unsigned 32-bit | the image's length in bytes |
| expires | seconds since the Unix epoch | when the release stops being offered, or 0 for never |

The manifest is a deterministic CBOR map, and the envelope carries it as a byte
string next to a 64-byte Ed25519 signature over exactly those bytes. A manifest
body fits in 128 bytes and an envelope in 224 (`MANIFEST_MAX` and
`ENVELOPE_MAX`); the one in the example is 156.

**What a slot goes through:**

| State | Means | Leaves it when |
| --- | --- | --- |
| Empty | nothing here, or nothing worth keeping | a release is begun for it: Receiving |
| Receiving | part of an image, unverified and never bootable | the image finishes whole and matching: Staged |
| Staged | a verified image not yet booted | the next boot tries it: Pending |
| Pending | booted, not yet reported healthy | it confirms: Confirmed; a revert or the next boot: Failed |
| Confirmed | booted and healthy; what the device falls back to | a newer image confirms, which erases it: Empty |
| Failed | booted and never confirmed; never tried again | a newer release is begun for it: Receiving |

A slot a transfer leaves in Receiving, cut off or refused at the end, does not
count toward the installed sequence, so the same release can be offered again. A
Failed slot does count.

**What a boot decides,** and what to run:

| Decision | When | Run |
| --- | --- | --- |
| Trying | a slot is Staged, and is now Pending | the staged slot |
| Confirmed | nothing is staged or pending | the confirmed slot |
| Reverted | a slot was Pending, so its trial never confirmed, and it is now Failed | the fallback, the confirmed slot |

In the bindings a decision is an action, a slot, and a fallback. For Reverted,
the slot is the one that failed and the fallback is the one to run; for the
others both name the same slot.

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| hash an image | `image_digest(image)` |
| sign a release | `manifest.sign(&identity, &mut buf)` into `[0u8; ENVELOPE_MAX]`, giving its length |
| check an envelope | `Envelope::decode(bytes)?.verify(&public)`, giving the `Manifest` |
| set up a device | `Updater::new(Device { vendor_id, class_id, anchor }, store)` |
| record the factory image | `provision(slot, sequence)` |
| take an image in pieces | `begin(envelope)` or `begin_at(envelope, now)`, then `write(chunk)`, `progress()`, `finish()` |
| take a piece per call | `staging.detach()`, then `resume_from(envelope, now, transfer)` for the next |
| carry on after a reset | `resume_at(envelope, now)` |
| take an image held whole | `stage(envelope, image)` or `stage_at(envelope, image, now)` |
| decide what to boot | `on_boot()`, giving a `Boot` |
| settle a trial | `confirm()` or `revert()` |
| read a slot | `store().record(slot)`, giving a `SlotRecord` |
| the number to beat | `installed_sequence()` |
| appoint a release key | `Delegation { epoch, release_key, expires }.sign(&anchor, &mut buf)` |
| accept a release key | `adopt(signed, now)`, or `with_delegation(signed, now)` on the way up |

### TypeScript

| To | Call |
| --- | --- |
| hash an image | `imageDigest(image)` |
| sign a release | `signManifest(manifest, identity)`, giving the envelope |
| check an envelope | `verifyEnvelope(envelope, publicKey)`, giving the manifest |
| set up a device | `new Updater(vendorId, classId, anchorPublicKey, slotCount, slotCapacity)` |
| record the factory image | `provision(slot, sequence)` |
| take an image in pieces | `begin(envelope, now?)`, then `write(chunk)`, `progress()`, `finish()` |
| take an image held whole | `stage(envelope, image, now?)` |
| decide what to boot | `onBoot()`, giving `{ action, slot, fallback }` |
| settle a trial | `confirm()` or `revert()` |
| read a slot | `slotRecord(slot)` |
| the number to beat | `installedSequence` |
| appoint a release key | `signDelegation({ epoch, releaseKey, expires }, anchor)` |
| accept a release key | `adopt(signed, now?)`, and `delegation` for the one in force |

### Python

| To | Call |
| --- | --- |
| hash an image | `image_digest(image)` |
| sign a release | `sign_manifest(manifest, identity)`, giving the envelope |
| check an envelope | `verify_envelope(envelope, public_key)`, giving the manifest |
| set up a device | `Updater(vendor_id, class_id, anchor_public_key, slot_count, slot_capacity)` |
| record the factory image | `provision(slot, sequence)` |
| take an image in pieces | `begin(envelope, now=None)`, then `write(chunk)`, `progress()`, `finish()` |
| take an image held whole | `stage(envelope, image, now=None)` |
| decide what to boot | `on_boot()`, giving a `BootDecision` |
| settle a trial | `confirm()` or `revert()` |
| read a slot | `slot_record(slot)` |
| the number to beat | `installed_sequence` |
| appoint a release key | `sign_delegation(Delegation(epoch, release_key), anchor)` |
| accept a release key | `adopt(signed, now=None)`, and `delegation` for the one in force |

### C#

| To | Call |
| --- | --- |
| hash an image | `Update.ImageDigest(image)` |
| sign a release | `Update.SignManifest(manifest, identity)`, giving the envelope |
| check an envelope | `Update.VerifyEnvelope(envelope, publicKey)`, giving the `Manifest` |
| set up a device | `new Updater(vendorId, classId, anchorPublicKey, slotCount, slotCapacity)` |
| record the factory image | `Provision(slot, sequence)` |
| take an image in pieces | `Begin(envelope, now)`, then `Write(chunk)`, `CurrentProgress()`, `Finish()` |
| take an image held whole | `Stage(envelope, image, now)` |
| decide what to boot | `OnBoot()`, giving a `BootDecision` |
| settle a trial | `Confirm()` or `Revert()` |
| read a slot | `Record(slot)`, giving a `SlotRecord` |
| the number to beat | `InstalledSequence` |
| appoint a release key | `Update.SignDelegation(new Delegation(epoch, releaseKey), anchor)` |
| accept a release key | `Adopt(signed, now)`, and `CurrentDelegation` for the one in force |

<!-- languages end -->

## When it goes wrong

A release is checked in a fixed order: the envelope's signature before anything
in the manifest is read, then the vendor and class, the expiry, the sequence,
whether the image fits its slot, and which slot it names.
The first rule it breaks is the one it is refused for, with the same message in
every language:

| What happened | The message |
| --- | --- |
| signed by a key that is neither the anchor nor the release key it delegated to | `the manifest signature is not from the trusted key` |
| for another vendor or device class | `the manifest is for a different vendor or device class` |
| its expiry has passed | `the manifest has expired` |
| it expires, and the call passed no time | `the manifest expires and this device has no clock` |
| its sequence does not beat every slot the device holds | `the sequence number would roll the device back` |
| the image is larger than the slot | `the image does not fit the target slot` |
| it names the slot holding the confirmed image | `the slot is not in a state that allows this` |
| more bytes arrive than it declares, or fewer by the finish | `the image is not the size the manifest declares` |
| the image does not hash to the signed digest | `the image does not match the manifest digest` |
| the envelope or the manifest is not the CBOR shape a device reads | `the manifest is malformed` |
| a newer structure version, or a payload format this build cannot apply | `the manifest structure version is not supported` |
| a slot number the device does not have | `no such slot on this device` |
| a confirm or revert with nothing on trial, or a second provision | `the slot is not in a state that allows this` |
| a boot or a revert with no confirmed image to fall back to | `there is no confirmed image to revert to` |

In Rust the message is the `Refusal` itself. Turned into the core `Error`, a
refused signature, digest, device, sequence, or expiry is an authentication
error, a malformed or unsupported manifest a codec error, and the rest are I/O
errors, each with the reason after its prefix.

The mistakes that cost an afternoon:

- **Every release that expires is refused.** A device with no clock cannot tell
  whether a release is still offered, so `begin` and `stage` without a time refuse
  one that expires. Pass the time, or publish without an expiry.
- **The fix is refused as a rollback.** A release that boots and never confirms
  spends its sequence number, so the next attempt needs a higher one, even if the
  image is the same.
- **One image for the whole fleet.** A manifest names the slot its image goes in,
  and a device refuses one that names the slot it is running from. With two
  slots, a release is two manifests, one per slot, and each device takes the one
  naming its spare; a direct-XIP image is built for the address it runs from
  anyway.
- **Confirming too early.** `confirm` erases the image the device would fall back
  to. Confirm once the new image has done what proves it works, such as reaching
  the server, not as soon as it starts.
- **Never confirming.** An image that runs fine but never calls `confirm` is
  failed on the next reboot, and the device quietly goes back to the old one.
- **A refused image is refused again when resumed.** An image that fails its
  digest leaves its slot holding the bad bytes, and resuming picks up after the
  last of them, so `finish` refuses again. Begin the release again, which clears
  the slot, rather than resuming it.
- **Slot records that do not survive a reboot.** The rules run on what the
  `SlotStore` says, including the sequence numbers rollback protection compares
  against. A store that loses its records forgets which releases it has seen.
  `MemoryStore`, and every binding's updater, keep slots in memory, which suits a
  gateway working out what a device will accept, or a test, but not firmware.

## Where next

<!-- table: next update -->
- [Device identity](security.md): ed25519 device identity.
- [Audit log](audit.md): A tamper-evident, hash-chained log.
- [LoRaWAN](lorawan.md): LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join.
- Beside it: [Firmware over the air](fuota.md).
- Also in Trust and operation: [Secured session](session.md), [Power](power.md), [Telemetry](telemetry.md).
<!-- end -->

## Reference

<!-- table: reference update -->
- Rust: [`pamoja-update`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-update)
- TypeScript: [`@pamoja/update`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-update)
- Python: [`pamoja.update`](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-update)
- C#: [`Pamoja.Update`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-update)
<!-- end -->
