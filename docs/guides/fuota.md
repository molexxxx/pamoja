# Firmware over the air

A signed update has to reach the device before it can be refused or accepted, and
over LoRaWAN that is the hard part. An image is larger than any frame, the link
drops a fair share of what it carries, and sending it once per device would take
a gateway out of service for a day. The LoRa Alliance answers with three
application layer packages that work together: a group of devices is given one
address and one key so the image goes out once, the image is cut into fragments
with more sent than there are so a device solves for what it missed, and a clock
package keeps every device agreeing on when the window opens.

None of that is trusted. The packages move bytes; what decides whether the image
runs is the same signed manifest a wired update carries, checked against the key
the device was anchored to. A group key that reaches the wrong device still gets
it nothing.

## What the example does

It signs a release, frames the manifest and the image into one block, sets up a
multicast group whose key travels wrapped under a key each device derives for
itself, cuts the block into fragments, and loses every fourth one on the way. The
device solves for the losses, checks the block against the code the session setup
carried, and only then hands what it rebuilt to the updater. Finally the same
release comes back signed by a key the device does not trust.

The block convention is pamoja's own: the signed manifest and the image behind a
six-byte header that says where each begins. TS004-2.0.0 moves a block and says
nothing about what is inside it, so something has to, and putting the manifest at
a known offset is the least a transport can get away with. Nothing in that header
is trusted; every rule that decides whether the image runs is still the
manifest's.

It proves:

- The group key the server wraps is the one the device unwraps, derived through
  `McRootKey` and `McKEKey` from a root key the device never transmits.
- The session says how many fragments a block takes and how much padding the last
  one carries, so a device knows the shape before the first fragment arrives.
- A device that missed a quarter of the fragments still finishes, because the
  coded fragments past the first pass let it solve for the ones it lost.
- The block it rebuilt is the block the server sent, checked by a code taken over
  it a piece at a time so the image is never held twice.
- The manifest is verified after all of that, against the anchored key, exactly as
  it would be over a wire, and a release signed by anybody else is refused.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example fuota" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example fuota</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- fuota" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- fuota</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/fuota.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/fuota.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- fuota" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- fuota</code></div>
</div>
<!-- end -->

## Rust

<!-- snippet: examples/guides/fuota.rs#example -->
From [`examples/guides/fuota.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/fuota.rs):

```rust
use pamoja_lorawan::packages::fragment::{
    data_block_int_key, BlockMicKey, Defragmenter, Fragmenter,
};
use pamoja_lorawan::packages::multicast::{
    mc_app_s_key, mc_ke_key, mc_key, mc_root_key, wrap_mc_key,
};
use pamoja_security::DeviceIdentity;
use pamoja_update::block::{frame, split};
use pamoja_update::{
    image_digest, Device, Manifest, MemoryStore, PayloadFormat, SlotState, SlotStore, Updater,
    ENVELOPE_MAX, STRUCTURE_VERSION,
};

// The publisher signs releases; the devices in the field are anchored to its public half
// and will take firmware from nobody else, however it reaches them.
let publisher = DeviceIdentity::from_seed(&[7u8; 32]);
const VENDOR: [u8; 16] = [10; 16];
const FLOW_METER: [u8; 16] = [11; 16];

let image = b"firmware for a flow meter, version two, long enough to need fragmenting";
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
let mut envelope = [0u8; ENVELOPE_MAX];
let signed = manifest.sign(&publisher, &mut envelope)?;

// One block carries the release: the signed manifest and the image behind a header that
// says where each begins. The transport moves bytes and vouches for none of them.
let mut block = [0u8; 512];
let block_len = frame(&envelope[..signed], image, &mut block)?;
println!(
    "release   {block_len} bytes: a signed manifest and {} of image",
    image.len()
);

// The group every device in the field belongs to. Its key travels wrapped under a key
// each device derives from its own root key, so the broadcast key is never in the clear.
let device_root_key = [0x2B; 16];
let group_key = [0x77; 16];
let group_addr = 0x2601_0042;
let wrapped = wrap_mc_key(&mc_ke_key(&mc_root_key(&device_root_key)), &group_key);
let unwrapped = mc_key(&mc_ke_key(&mc_root_key(&device_root_key)), &wrapped);
println!(
    "group     {:#010X} keyed by a wrapped key the device unwraps: {}",
    group_addr,
    unwrapped == group_key
);
let _payload_key = mc_app_s_key(&group_key, group_addr);

// The server cuts the block into fragments and sends more than there are, so a device
// that misses some can still finish. Each coded fragment is the exclusive-or of a
// pseudo-random half of the originals.
let sender = Fragmenter::new(&block[..block_len], 32)?;
println!(
    "session   {} fragments of {} bytes, {} of padding",
    sender.nb_frag(),
    sender.frag_size(),
    sender.padding()
);

// The device puts it back together in storage it set aside once: the block itself, and
// room to solve for a handful of losses.
let mut store = [0u8; 512];
let mut matrix = [0u8; 256];
let mut receiver = Defragmenter::new(
    sender.nb_frag(),
    sender.frag_size(),
    &mut store,
    &mut matrix,
)?;

// The link drops every fourth fragment. The session keeps going until the block is whole.
let mut piece = [0u8; 32];
let (mut sent, mut coded) = (0, 0);
for n in 1..=sender.nb_frag() * 2 {
    if n % 4 == 0 {
        continue;
    }
    sender.fragment(n, &mut piece)?;
    sent += 1;
    coded += usize::from(n > sender.nb_frag());
    if receiver.fragment(n, &piece)? {
        break;
    }
}
println!("received  {sent} fragments, {coded} of them coded, and the block is whole");

// What the device built is checked against the code the session setup carried, taken
// over the block a piece at a time so the image is never held twice.
let signer = BlockMicKey::new(&data_block_int_key(&device_root_key));
let mut expected = signer.start(1, 0, *b"PJU1", block_len as u32);
expected.update(&block[..block_len]);
let mut built = signer.start(1, 0, *b"PJU1", block_len as u32);
built.update(&receiver.block()[..block_len]);
let intact = built.finish() == expected.finish();
println!("checked   the block the device built is the one the server sent: {intact}");

// Only now does the update itself get a say. The header says where the manifest ends;
// everything after that is the manifest's decision, exactly as for a wired update.
let (carried_envelope, carried_image) = split(&receiver.block()[..block_len])?;
let mut updater = Updater::new(
    Device {
        vendor_id: VENDOR,
        class_id: FLOW_METER,
        anchor: publisher.public(),
    },
    MemoryStore::new(2, 4096),
);
updater.provision(0, 1).expect("the shipped image");
let slot = updater.stage(carried_envelope, carried_image)?;
println!("staged    into slot {slot}, leaving the running image alone");

// A release broadcast to everyone is still refused by anyone it is not for. This one is
// signed by another key.
let impostor = DeviceIdentity::from_seed(&[90u8; 32]);
let mut forged = [0u8; ENVELOPE_MAX];
let forged_len = manifest.sign(&impostor, &mut forged)?;
match updater.stage(&forged[..forged_len], image) {
    Ok(_) => println!("a forged release was accepted, which should never happen"),
    Err(error) => println!("forged    refused: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/fuota.ts#example -->
From [`bindings/node/guides/fuota.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/fuota.ts):

```typescript
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
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/fuota.py#example -->
From [`bindings/python/guides/fuota.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/fuota.py):

```python
from pamoja.core import PamojaError
from pamoja.lorawan import fragment, multicast
from pamoja.security import DeviceIdentity
from pamoja.update import (
    Manifest,
    Updater,
    frame_block,
    image_digest,
    sign_manifest,
    split_block,
)

# The publisher signs releases; the devices in the field are anchored to its public half
# and will take firmware from nobody else, however it reaches them.
publisher = DeviceIdentity.from_seed(bytes([7]) * 32)
vendor = bytes([0x0A]) * 16
flow_meter = bytes([0x0B]) * 16

image = b"firmware for a flow meter, version two, long enough to need fragmenting"
envelope = sign_manifest(
    Manifest(
        sequence=2,
        vendor_id=vendor,
        class_id=flow_meter,
        storage=1,
        digest=image_digest(image),
        size=len(image),
    ),
    publisher,
)

# One block carries the release: the signed manifest and the image behind a header that
# says where each begins. The transport moves bytes and vouches for none of them.
block = frame_block(envelope, image)
print(f"release   {len(block)} bytes: a signed manifest and {len(image)} of image")

# The group every device in the field belongs to. Its key travels wrapped under a key each
# device derives from its own root key, so the broadcast key is never in the clear.
device_root_key = bytes([0x2B]) * 16
group_key = bytes([0x77]) * 16
group_addr = 0x26010042
ke_key = multicast.ke_key(multicast.root_key(device_root_key))
wrapped = multicast.wrap_key(ke_key, group_key)
unwrapped = multicast.key(ke_key, wrapped)
print(
    f"group     0x{group_addr:08X} keyed by a wrapped key the device unwraps: "
    f"{unwrapped == group_key}"
)
payload_key = multicast.app_s_key(group_key, group_addr)

# The server cuts the block into fragments and sends more than there are, so a device that
# misses some can still finish. Each coded fragment is the exclusive-or of a pseudo-random
# half of the originals.
frag_size = 32
session = fragment.session(len(block), frag_size)
print(
    f"session   {session.nb_frag} fragments of {frag_size} bytes, "
    f"{session.padding} of padding"
)

# The device puts it back together in storage it set aside once: the block itself, and room
# to solve for a handful of losses.
receiver = fragment.Defragmenter(session.nb_frag, frag_size, 8)

# The link drops every fourth fragment. The session keeps going until the block is whole.
sent = 0
coded = 0
for n in range(1, session.nb_frag * 2 + 1):
    if n % 4 == 0:
        continue
    sent += 1
    if n > session.nb_frag:
        coded += 1
    if receiver.fragment(n, fragment.fragment(block, frag_size, n)):
        break
print(f"received  {sent} fragments, {coded} of them coded, and the block is whole")

# What the device built is checked against the code the session setup carried, taken over
# the block a piece at a time so the image is never held twice.
block_key = fragment.data_block_int_key(device_root_key)
descriptor = b"PJU1"
expected = fragment.BlockMic(block_key, 1, 0, descriptor, len(block))
expected.update(block)
built = fragment.BlockMic(block_key, 1, 0, descriptor, len(block))
built.update(receiver.block[: len(block)])
intact = built.finish() == expected.finish()
print(f"checked   the block the device built is the one the server sent: {intact}")

# Only now does the update itself get a say. The header says where the manifest ends;
# everything after that is the manifest's decision, exactly as for a wired update.
carried_envelope, carried_image = split_block(receiver.block[: len(block)])
updater = Updater(vendor, flow_meter, publisher.public_key, 2, 4096)
updater.provision(0, 1)
slot = updater.stage(carried_envelope, carried_image)
print(f"staged    into slot {slot}, leaving the running image alone")

# A release broadcast to everyone is still refused by anyone it is not for. This one is
# signed by another key.
impostor = DeviceIdentity.from_seed(bytes([90]) * 32)
forged = sign_manifest(
    Manifest(
        sequence=2,
        vendor_id=vendor,
        class_id=flow_meter,
        storage=1,
        digest=image_digest(image),
        size=len(image),
    ),
    impostor,
)
try:
    updater.stage(forged, image)
    print("a forged release was accepted, which should never happen")
except PamojaError as error:
    print(f"forged    refused: {error}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/FuotaGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/FuotaGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/FuotaGuide.cs):

```csharp
// The publisher signs releases; the devices in the field are anchored to its public
// half and will take firmware from nobody else, however it reaches them.
byte[] seed = new byte[32];
Array.Fill(seed, (byte)7);
using var publisher = new DeviceIdentity(seed);
byte[] vendor = Enumerable.Repeat((byte)0x0A, 16).ToArray();
byte[] flowMeter = Enumerable.Repeat((byte)0x0B, 16).ToArray();

byte[] image = Encoding.ASCII.GetBytes(
    "firmware for a flow meter, version two, long enough to need fragmenting");
var manifest = new Manifest(
    Sequence: 2,
    VendorId: vendor,
    ClassId: flowMeter,
    Storage: 1,
    Digest: Update.ImageDigest(image),
    Size: (uint)image.Length);
byte[] envelope = Update.SignManifest(manifest, publisher);

// One block carries the release: the signed manifest and the image behind a header
// that says where each begins. The transport moves bytes and vouches for none of them.
byte[] block = Update.FrameBlock(envelope, image);
Console.WriteLine(
    $"release   {block.Length} bytes: a signed manifest and {image.Length} of image");

// The group every device in the field belongs to. Its key travels wrapped under a key
// each device derives from its own root key, so the broadcast key is never in the clear.
byte[] deviceRootKey = Enumerable.Repeat((byte)0x2B, 16).ToArray();
byte[] groupKey = Enumerable.Repeat((byte)0x77, 16).ToArray();
const uint groupAddr = 0x2601_0042;
byte[] keKey = LorawanPackages.McKeKey(LorawanPackages.McRootKey(deviceRootKey));
byte[] wrapped = LorawanPackages.WrapMcKey(keKey, groupKey);
byte[] unwrapped = LorawanPackages.McKey(keKey, wrapped);
Console.WriteLine(
    $"group     0x{groupAddr:X8} keyed by a wrapped key the device unwraps: " +
    $"{unwrapped.SequenceEqual(groupKey)}");
byte[] payloadKey = LorawanPackages.McAppSKey(groupKey, groupAddr);

// The server cuts the block into fragments and sends more than there are, so a device
// that misses some can still finish. Each coded fragment is the exclusive-or of a
// pseudo-random half of the originals.
const byte fragSize = 32;
LorawanFragSession session = LorawanPackages.FragSession(block.Length, fragSize);
Console.WriteLine(
    $"session   {session.NbFrag} fragments of {fragSize} bytes, " +
    $"{session.Padding} of padding");

// The device puts it back together in storage it set aside once: the block itself,
// and room to solve for a handful of losses.
using var receiver = new LorawanDefragmenter(session.NbFrag, fragSize, 8);

// The link drops every fourth fragment. The session keeps going until the block is whole.
int sent = 0;
int coded = 0;
for (ushort n = 1; n <= session.NbFrag * 2; n++)
{
    if (n % 4 == 0)
    {
        continue;
    }

    sent++;
    if (n > session.NbFrag)
    {
        coded++;
    }

    if (receiver.Fragment(n, LorawanPackages.FragFragment(block, fragSize, n)))
    {
        break;
    }
}

Console.WriteLine(
    $"received  {sent} fragments, {coded} of them coded, and the block is whole");

// What the device built is checked against the code the session setup carried, taken
// over the block a piece at a time so the image is never held twice.
byte[] blockKey = LorawanPackages.DataBlockIntKey(deviceRootKey);
byte[] descriptor = Encoding.ASCII.GetBytes(Update.BlockDescriptor);
using var expected = new LorawanBlockMic(blockKey, 1, 0, descriptor, (uint)block.Length);
expected.Update(block);
byte[] built = receiver.Block().AsSpan(0, block.Length).ToArray();
using var taken = new LorawanBlockMic(blockKey, 1, 0, descriptor, (uint)block.Length);
taken.Update(built);
bool intact = taken.Finish().SequenceEqual(expected.Finish());
Console.WriteLine($"checked   the block the device built is the one the server sent: {intact}");

// Only now does the update itself get a say. The header says where the manifest ends;
// everything after that is the manifest's decision, exactly as for a wired update.
(byte[] carriedEnvelope, byte[] carriedImage) = Update.SplitBlock(built);
using var updater = new Updater(vendor, flowMeter, publisher.PublicKey, 2, 4096);
updater.Provision(0, 1);
byte slot = updater.Stage(carriedEnvelope, carriedImage);
Console.WriteLine($"staged    into slot {slot}, leaving the running image alone");

// A release broadcast to everyone is still refused by anyone it is not for. This one
// is signed by another key.
byte[] impostorSeed = new byte[32];
Array.Fill(impostorSeed, (byte)90);
using var impostor = new DeviceIdentity(impostorSeed);
try
{
    updater.Stage(Update.SignManifest(manifest, impostor), image);
    Console.WriteLine("a forged release was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"forged    refused: {error.Message}");
}
```
<!-- end -->

## Reference

The packages are part of the LoRaWAN capability, and the manifest and the slots
are part of signed updates:

<!-- table: reference lorawan -->
- Rust: [`pamoja-lorawan`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-lorawan)
- TypeScript: [`@pamoja/lorawan`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-lorawan)
- Python: [`pamoja.lorawan`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-lorawan)
- C#: [`Pamoja.Lorawan`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-lorawan)
<!-- end -->

<!-- table: reference update -->
- Rust: [`pamoja-update`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-update)
- TypeScript: [`@pamoja/update`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_update.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-update)
- Python: [`pamoja.update`](https://pamoja.molex.cloud/docs/reference/python/pamoja/update.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-update)
- C#: [`Pamoja.Update`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Update.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-update)
<!-- end -->
