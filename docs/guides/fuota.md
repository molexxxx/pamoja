# Firmware over the air

A signed update has to reach the device before it can be refused or accepted, and
over LoRaWAN that is the hard part. An image is larger than any frame, the link
drops a fair share of what it carries, and sending it once per device would take
a gateway out of service for a day. The LoRa Alliance answers with application
layer packages that work together: a group of devices is given one address and one
key so the image goes out once, the image is cut into fragments with more sent than
there are so a device solves for what it missed, a clock package keeps every device
agreeing on when the window opens, and a firmware package says what each device
holds and when it reboots.

None of that is trusted. The packages move bytes; what decides whether the image
runs is the same signed manifest a wired update carries, checked against the key
the device was anchored to. A group key that reaches the wrong device still gets
it nothing.

## What the example does

It signs a release, frames the manifest and the image into one block, sets up a
multicast group whose key travels wrapped under a key each device derives for
itself, cuts the block into fragments, and loses every fourth one on the way. The
device solves for the losses, checks the block against the code the server took over
it with this device's own key, and only then hands what it rebuilt to the updater.
The same release then comes back signed by a key the device does not trust.

The second part runs the same session over a worse link. A device further out hears
every other fragment of the first pass and is still missing some when it ends, so
the server keeps sending coded fragments until it has what it needs. Then a fragment
is changed on the way, and the block's code catches what the fragments alone cannot.

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
- A device that heard only half of the first pass knows how many it still lacks,
  and finishes on the coded fragments that follow, a few more than it lacks, since
  not every coded fragment tells it something new.
- The block it rebuilt is the block the server sent, checked by a code taken over
  it a piece at a time so the image is never held twice.
- A fragment changed on the way still completes the block, and the block's code,
  taken with the device's own key, is what refuses it. A member of the same group,
  who holds the group key, cannot forge it either.
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

In Rust, the packages are modules of `pamoja-lorawan`: `packages::multicast` derives
a device's keys and wraps or unwraps a group key, `packages::fragment` has
`Fragmenter` for the server, `Defragmenter` for the device, and `BlockMicKey` for
the block's code, and `packages::clock` and `packages::firmware` hold the other two
packages, with each package's commands to build and read. A `Defragmenter` works in
storage the caller sets aside once and never allocates. `pamoja-update` frames and
splits the block with `block::frame` and `block::split`, and its `Updater` stages what
arrived. A refusal is an `Err`: a `LorawanError` or `FragError` from the transport,
and a `Refusal` from the updater, whose message names the rule that refused.

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
use pamoja_update::block::{frame, split, DESCRIPTOR};
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
let block = &block[..block_len];
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
let keyed = if unwrapped == group_key {
    "keyed by the key the server wrapped and the device unwrapped"
} else {
    "with a key the device could not unwrap"
};
println!("group     {group_addr:#010x}, {keyed}");
let _payload_key = mc_app_s_key(&group_key, group_addr);

// The server cuts the block into fragments and sends more than there are, so a device
// that misses some can still finish. Each coded fragment is the exclusive-or of a
// pseudo-random half of the originals.
let sender = Fragmenter::new(block, 32)?;
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

// What the device built is checked against the code the server took over the block with
// this device's own key, and sent in the session setup. It is taken a piece at a time, so
// the image is never held twice.
let signer = BlockMicKey::new(&data_block_int_key(&device_root_key));
let mut expected = signer.start(1, 0, DESCRIPTOR, block_len as u32);
expected.update(block);
let expected = expected.finish();
let mut built = signer.start(1, 0, DESCRIPTOR, block_len as u32);
built.update(&receiver.block()[..block_len]);
let intact = built.finish() == expected;
let verdict = if intact {
    "carries the code the server took over it"
} else {
    "does not carry the code the server took over it"
};
println!("checked   the block the device built {verdict}");

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

The same session over a worse link, continuing from above:

<!-- snippet: examples/guides/fuota.rs#losses -->
From [`examples/guides/fuota.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/fuota.rs):

```rust
// A device further out hears every other fragment of the session's first pass, eight
// uncoded and four coded. It solves for what it can and says how many it still lacks,
// so the server keeps sending coded fragments until it has none left to ask for.
let mut far_store = [0u8; 512];
let mut far_matrix = [0u8; 256];
let mut far = Defragmenter::new(
    sender.nb_frag(),
    sender.frag_size(),
    &mut far_store,
    &mut far_matrix,
)?;
let first_pass = sender.nb_frag() + sender.nb_frag() / 2;
for n in (1..=first_pass).step_by(2) {
    sender.fragment(n, &mut piece)?;
    far.fragment(n, &piece)?;
}
println!(
    "short     heard {} of {first_pass} fragments, and {} are still missing",
    far.received(),
    far.missing()
);
let mut more = 0;
for n in first_pass + 1.. {
    sender.fragment(n, &mut piece)?;
    more += 1;
    if far.fragment(n, &piece)? {
        break;
    }
}
println!("more      {more} more coded fragments finish the block");

// A fragment changed on the way, by a fault or by another member of the group, who holds
// the same group key, still completes the block. The code the server took with this
// device's own key is what catches it, so the updater never sees the block.
let mut odd_store = [0u8; 512];
let mut odd_matrix = [0u8; 256];
let mut odd = Defragmenter::new(
    sender.nb_frag(),
    sender.frag_size(),
    &mut odd_store,
    &mut odd_matrix,
)?;
for n in 1..=sender.nb_frag() {
    sender.fragment(n, &mut piece)?;
    if n == 3 {
        piece[0] ^= 0x01;
    }
    odd.fragment(n, &piece)?;
}
let mut taken = signer.start(1, 0, DESCRIPTOR, block_len as u32);
taken.update(&odd.block()[..block_len]);
let caught = taken.finish() != expected;
if odd.done() && caught {
    println!("tampered  the block completes, its code does not match, and nothing is staged");
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/lorawan` groups each package in a namespace: `multicast`
for the keys, `fragment` for `session`, `fragment`, the `Defragmenter` class and
`BlockMic`, and `clock` and `firmware` for the other two, with `packageEncode` and
`packageParse` reading and writing any package's commands by name. `@pamoja/update`
frames and splits the block, and `Updater` stages it. Blocks and keys are `Buffer`s,
and a refusal throws an `Error` carrying the rule that refused.

<!-- snippet: bindings/node/guides/fuota.ts#example -->
From [`bindings/node/guides/fuota.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/fuota.ts):

```typescript
import { DeviceIdentity } from '@pamoja/security'
import { fragment, multicast } from '@pamoja/lorawan'
import {
  BLOCK_DESCRIPTOR,
  frameBlock,
  imageDigest,
  signManifest,
  splitBlock,
  Updater,
} from '@pamoja/update'

// The publisher signs releases; the devices in the field are anchored to its public half
// and will take firmware from nobody else, however it reaches them.
const publisher = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))
const vendor = Buffer.alloc(16, 10)
const flowMeter = Buffer.alloc(16, 11)

const image = Buffer.from('firmware for a flow meter, version two, long enough to need fragmenting')
const manifest = {
  sequence: 2,
  vendorId: vendor,
  classId: flowMeter,
  storage: 1,
  digest: imageDigest(image),
  size: image.length,
}
const envelope = signManifest(manifest, publisher)

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
const keyed = unwrapped.equals(groupKey)
  ? 'keyed by the key the server wrapped and the device unwrapped'
  : 'with a key the device could not unwrap'
console.log(`group     0x${groupAddr.toString(16).padStart(8, '0')}, ${keyed}`)
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

// What the device built is checked against the code the server took over the block with this
// device's own key, and sent in the session setup. It is taken a piece at a time, so the
// image is never held twice.
const blockKey = fragment.dataBlockIntKey(deviceRootKey)
const descriptor = Buffer.from(BLOCK_DESCRIPTOR)
const taking = new fragment.BlockMic(blockKey, 1, 0, descriptor, block.length)
taking.update(block)
const expected = taking.finish()
const built = new fragment.BlockMic(blockKey, 1, 0, descriptor, block.length)
built.update(receiver.block.subarray(0, block.length))
const intact = built.finish().equals(expected)
const verdict = intact
  ? 'carries the code the server took over it'
  : 'does not carry the code the server took over it'
console.log(`checked   the block the device built ${verdict}`)

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
try {
  updater.stage(signManifest(manifest, impostor), image)
  console.log('a forged release was accepted, which should never happen')
} catch (error) {
  console.log(`forged    refused: ${(error as Error).message}`)
}
```
<!-- end -->

The same session over a worse link, continuing from above:

<!-- snippet: bindings/node/guides/fuota.ts#losses -->
From [`bindings/node/guides/fuota.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/fuota.ts):

```typescript
// A device further out hears every other fragment of the session's first pass, eight
// uncoded and four coded. It solves for what it can and says how many it still lacks, so
// the server keeps sending coded fragments until it has none left to ask for.
const far = new fragment.Defragmenter(nbFrag, fragSize, 8)
const firstPass = nbFrag + nbFrag / 2
for (let n = 1; n <= firstPass; n += 2) {
  far.fragment(n, fragment.fragment(block, fragSize, n))
}
console.log(`short     heard ${far.received} of ${firstPass} fragments, and ${far.missing} are still missing`)
let more = 0
for (let n = firstPass + 1; ; n++) {
  more += 1
  if (far.fragment(n, fragment.fragment(block, fragSize, n))) {
    break
  }
}
console.log(`more      ${more} more coded fragments finish the block`)

// A fragment changed on the way, by a fault or by another member of the group, who holds the
// same group key, still completes the block. The code the server took with this device's own
// key is what catches it, so the updater never sees the block.
const odd = new fragment.Defragmenter(nbFrag, fragSize, 8)
for (let n = 1; n <= nbFrag; n++) {
  const piece = fragment.fragment(block, fragSize, n)
  if (n === 3) {
    piece[0] ^= 0x01
  }
  odd.fragment(n, piece)
}
const taken = new fragment.BlockMic(blockKey, 1, 0, descriptor, block.length)
taken.update(odd.block.subarray(0, block.length))
const caught = !taken.finish().equals(expected)
if (odd.done && caught) {
  console.log('tampered  the block completes, its code does not match, and nothing is staged')
}
```
<!-- end -->

## Python

In Python, `pamoja.lorawan` has the same namespaces as modules: `multicast`,
`fragment`, `clock`, and `firmware`, with `package_encode` and `package_parse` for any
package's commands. `pamoja.update` frames and splits the block and stages it with
`Updater`. Keys and blocks are `bytes`; `BLOCK_DESCRIPTOR` is text, encoded where a
code takes it. A refusal raises `PamojaError` with the rule that refused.

<!-- snippet: bindings/python/guides/fuota.py#example -->
From [`bindings/python/guides/fuota.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/fuota.py):

```python
from pamoja.core import PamojaError
from pamoja.lorawan import fragment, multicast
from pamoja.security import DeviceIdentity
from pamoja.update import (
    BLOCK_DESCRIPTOR,
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
manifest = Manifest(
    sequence=2,
    vendor_id=vendor,
    class_id=flow_meter,
    storage=1,
    digest=image_digest(image),
    size=len(image),
)
envelope = sign_manifest(manifest, publisher)

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
keyed = (
    "keyed by the key the server wrapped and the device unwrapped"
    if unwrapped == group_key
    else "with a key the device could not unwrap"
)
print(f"group     {group_addr:#010x}, {keyed}")
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

# What the device built is checked against the code the server took over the block with this
# device's own key, and sent in the session setup. It is taken a piece at a time, so the
# image is never held twice.
block_key = fragment.data_block_int_key(device_root_key)
descriptor = BLOCK_DESCRIPTOR.encode()
taking = fragment.BlockMic(block_key, 1, 0, descriptor, len(block))
taking.update(block)
expected = taking.finish()
built = fragment.BlockMic(block_key, 1, 0, descriptor, len(block))
built.update(receiver.block[: len(block)])
intact = built.finish() == expected
verdict = (
    "carries the code the server took over it"
    if intact
    else "does not carry the code the server took over it"
)
print(f"checked   the block the device built {verdict}")

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
try:
    updater.stage(sign_manifest(manifest, impostor), image)
    print("a forged release was accepted, which should never happen")
except PamojaError as error:
    print(f"forged    refused: {error}")
```
<!-- end -->

The same session over a worse link, continuing from above:

<!-- snippet: bindings/python/guides/fuota.py#losses -->
From [`bindings/python/guides/fuota.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/fuota.py):

```python
# A device further out hears every other fragment of the session's first pass, eight uncoded
# and four coded. It solves for what it can and says how many it still lacks, so the server
# keeps sending coded fragments until it has none left to ask for.
far = fragment.Defragmenter(session.nb_frag, frag_size, 8)
first_pass = session.nb_frag + session.nb_frag // 2
for n in range(1, first_pass + 1, 2):
    far.fragment(n, fragment.fragment(block, frag_size, n))
print(f"short     heard {far.received} of {first_pass} fragments, and {far.missing} are still missing")
more = 0
n = first_pass
while True:
    n += 1
    more += 1
    if far.fragment(n, fragment.fragment(block, frag_size, n)):
        break
print(f"more      {more} more coded fragments finish the block")

# A fragment changed on the way, by a fault or by another member of the group, who holds the
# same group key, still completes the block. The code the server took with this device's own
# key is what catches it, so the updater never sees the block.
odd = fragment.Defragmenter(session.nb_frag, frag_size, 8)
for n in range(1, session.nb_frag + 1):
    piece = bytearray(fragment.fragment(block, frag_size, n))
    if n == 3:
        piece[0] ^= 0x01
    odd.fragment(n, bytes(piece))
taken = fragment.BlockMic(block_key, 1, 0, descriptor, len(block))
taken.update(odd.block[: len(block)])
caught = taken.finish() != expected
if odd.done and caught:
    print("tampered  the block completes, its code does not match, and nothing is staged")
```
<!-- end -->

## C#

In C#, `LorawanPackages` in `Pamoja.Lorawan` holds the key derivations, the session
shape, and the fragments, beside the `LorawanDefragmenter`, `LorawanBlockMic`,
`LorawanClockSync`, and `LorawanFirmware` classes, which own native state and are
disposed with `using`. `Pamoja.Update` frames and splits the block, and `Updater`
stages it. A refusal throws `PamojaException` with the rule that refused, and an
argument of the wrong length throws `ArgumentException` first.

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
string keyed = unwrapped.SequenceEqual(groupKey)
    ? "keyed by the key the server wrapped and the device unwrapped"
    : "with a key the device could not unwrap";
Console.WriteLine($"group     0x{groupAddr:x8}, {keyed}");
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

// What the device built is checked against the code the server took over the block
// with this device's own key, and sent in the session setup. It is taken a piece at a
// time, so the image is never held twice.
byte[] blockKey = LorawanPackages.DataBlockIntKey(deviceRootKey);
byte[] descriptor = Encoding.ASCII.GetBytes(Update.BlockDescriptor);
byte[] expected;
using (var taking = new LorawanBlockMic(blockKey, 1, 0, descriptor, (uint)block.Length))
{
    taking.Update(block);
    expected = taking.Finish();
}

byte[] built = receiver.Block().AsSpan(0, block.Length).ToArray();
using var check = new LorawanBlockMic(blockKey, 1, 0, descriptor, (uint)block.Length);
check.Update(built);
bool intact = check.Finish().SequenceEqual(expected);
string verdict = intact
    ? "carries the code the server took over it"
    : "does not carry the code the server took over it";
Console.WriteLine($"checked   the block the device built {verdict}");

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

The same session over a worse link, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/FuotaGuide.cs#losses -->
From [`bindings/dotnet/samples/Pamoja.Guides/FuotaGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/FuotaGuide.cs):

```csharp
// A device further out hears every other fragment of the session's first pass, eight
// uncoded and four coded. It solves for what it can and says how many it still lacks,
// so the server keeps sending coded fragments until it has none left to ask for.
using var far = new LorawanDefragmenter(session.NbFrag, fragSize, 8);
ushort firstPass = (ushort)(session.NbFrag + (session.NbFrag / 2));
for (ushort n = 1; n <= firstPass; n += 2)
{
    far.Fragment(n, LorawanPackages.FragFragment(block, fragSize, n));
}

Console.WriteLine(
    $"short     heard {far.Received} of {firstPass} fragments, and {far.Missing} are still missing");
int more = 0;
for (ushort n = (ushort)(firstPass + 1); ; n++)
{
    more++;
    if (far.Fragment(n, LorawanPackages.FragFragment(block, fragSize, n)))
    {
        break;
    }
}

Console.WriteLine($"more      {more} more coded fragments finish the block");

// A fragment changed on the way, by a fault or by another member of the group, who
// holds the same group key, still completes the block. The code the server took with
// this device's own key is what catches it, so the updater never sees the block.
using var odd = new LorawanDefragmenter(session.NbFrag, fragSize, 8);
for (ushort n = 1; n <= session.NbFrag; n++)
{
    byte[] piece = LorawanPackages.FragFragment(block, fragSize, n);
    if (n == 3)
    {
        piece[0] ^= 0x01;
    }

    odd.Fragment(n, piece);
}

using var taken = new LorawanBlockMic(blockKey, 1, 0, descriptor, (uint)block.Length);
taken.Update(odd.Block().AsSpan(0, block.Length));
bool caught = !taken.Finish().SequenceEqual(expected);
if (odd.Done && caught)
{
    Console.WriteLine("tampered  the block completes, its code does not match, and nothing is staged");
}
```
<!-- end -->

## Values at a glance

**The four packages,** each on a port of its own:

| Package | Specification | Port | What it does |
| --- | --- | --- | --- |
| Remote multicast setup | TS005-2.0.0 | 200 | gives a group of devices one address and one key, and says when the group listens |
| Fragmented data block transport | TS004-2.0.0 | 201 | cuts a block into fragments, and codes more so a device solves for what it missed |
| Clock synchronization | TS003-2.0.0 | 202 | tells a device the time, so it opens the group's window when the server does |
| Firmware management | TS006-1.0.0 | 203 | reports what a device runs and holds, and sets when it reboots |

**The keys,** all from the device's root key, which never leaves it:

| Key | Taken from | Used for |
| --- | --- | --- |
| McRootKey | the device's GenAppKey on LoRaWAN 1.0.x, or its AppKey on 1.1 | nothing but the next key |
| McKEKey | McRootKey | unwrapping each group key a setup carries |
| McKey | the setup command, unwrapped | the group's own two keys |
| McAppSKey, McNwkSKey | McKey and the group's address | reading and verifying the group's frames |
| DataBlockIntKey | the device's root key | the code over a whole block |

**A fragmentation session:**

| Number | Can be | In the example |
| --- | --- | --- |
| Fragments a block takes | 1 to 16383, the fourteen-bit index | 8 |
| Bytes a fragment carries | what the group's data rate carries, less the command's three | 32 |
| Padding the last one carries | less than a fragment | 23 |
| Losses a device can solve for | as many as it set aside room for | 8 |
| Sessions a device holds at once | 4 | 1 |
| Groups a device holds at once | 4 | 1 |

**The block's code** is taken with DataBlockIntKey over the session counter, the
session index, the four-byte descriptor, and the block's length, then the block
itself without its padding. The server sends it in the session setup, and the device
takes the same code over what it rebuilt.

**The block pamoja frames:**

| Bytes | Hold |
| --- | --- |
| 4 | `PJU1`, the descriptor the session setup names |
| 2 | the length of the signed manifest |
| as many as that | the signed manifest |
| the rest | the image |

**What the updater refuses,** in its own words:

| Refusal | Means |
| --- | --- |
| the manifest signature is not from the trusted key | signed by any key but the anchored one, or one it delegated to |
| the image does not match the manifest digest | the image is not the one the release names |
| the image is not the size the manifest declares | an image cut short or padded |
| the manifest is for a different vendor or device class | a release meant for other devices |
| the sequence number would roll the device back | a sequence no newer than the one installed |
| the manifest has expired | a release past its expiry |
| the manifest expires and this device has no clock | an expiry the device cannot check |
| the image does not fit the target slot | a slot smaller than the image |

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| key a group | `mc_root_key(&root)`, `mc_ke_key(&mc_root)`, `wrap_mc_key(&ke, &group)`, `mc_key(&ke, &wrapped)`, `mc_app_s_key(&group, addr)` |
| cut a block | `Fragmenter::new(block, frag_size)`, `nb_frag()`, `padding()`, `fragment(n, &mut out)` |
| rebuild it | `Defragmenter::new(nb_frag, frag_size, &mut store, &mut matrix)`, `fragment(n, &data)`, `received()`, `missing()`, `block()` |
| check it | `BlockMicKey::new(&data_block_int_key(&root))`, `start(cnt, index, DESCRIPTOR, len)`, `update(&block)`, `finish()` |
| carry a release | `block::frame(&envelope, image, &mut out)`, `block::split(&block)`, `Updater::stage(envelope, image)` |

### TypeScript

| To | Call |
| --- | --- |
| key a group | `multicast.rootKey(root)`, `multicast.keKey(mcRoot)`, `multicast.wrapKey(ke, group)`, `multicast.key(ke, wrapped)`, `multicast.appSKey(group, addr)` |
| cut a block | `fragment.session(length, fragSize)`, `fragment.fragment(block, fragSize, n)` |
| rebuild it | `new fragment.Defragmenter(nbFrag, fragSize, maxLost)`, `fragment(n, data)`, `received`, `missing`, `block` |
| check it | `new fragment.BlockMic(fragment.dataBlockIntKey(root), cnt, index, descriptor, length)`, `update(block)`, `finish()` |
| carry a release | `frameBlock(envelope, image)`, `splitBlock(block)`, `updater.stage(envelope, image)` |

### Python

| To | Call |
| --- | --- |
| key a group | `multicast.root_key(root)`, `multicast.ke_key(mc_root)`, `multicast.wrap_key(ke, group)`, `multicast.key(ke, wrapped)`, `multicast.app_s_key(group, addr)` |
| cut a block | `fragment.session(length, frag_size)`, `fragment.fragment(block, frag_size, n)` |
| rebuild it | `fragment.Defragmenter(nb_frag, frag_size, max_lost)`, `fragment(n, data)`, `received`, `missing`, `block` |
| check it | `fragment.BlockMic(fragment.data_block_int_key(root), cnt, index, descriptor, length)`, `update(block)`, `finish()` |
| carry a release | `frame_block(envelope, image)`, `split_block(block)`, `updater.stage(envelope, image)` |

### C#

| To | Call |
| --- | --- |
| key a group | `LorawanPackages.McRootKey(root)`, `McKeKey(mcRoot)`, `WrapMcKey(ke, group)`, `McKey(ke, wrapped)`, `McAppSKey(group, addr)` |
| cut a block | `LorawanPackages.FragSession(length, fragSize)`, `FragFragment(block, fragSize, n)` |
| rebuild it | `new LorawanDefragmenter(nbFrag, fragSize, maxLost)`, `Fragment(n, data)`, `Received`, `Missing`, `Block()` |
| check it | `new LorawanBlockMic(LorawanPackages.DataBlockIntKey(root), cnt, index, descriptor, length)`, `Update(block)`, `Finish()` |
| carry a release | `Update.FrameBlock(envelope, image)`, `Update.SplitBlock(block)`, `updater.Stage(envelope, image)` |

<!-- languages end -->

## When it goes wrong

The transport refuses a fragment that does not fit the session, and the updater
refuses a release with the rule it broke. The mistakes that cost an afternoon:

- **A device never finishes.** It missed more fragments than the coded ones sent so
  far let it solve for. Ask it how many it still lacks and keep sending coded
  fragments; a device that set aside room for too few losses has to be given a
  bigger margin before the session, not more fragments during it.
- **The group's frames never decrypt.** A group key unwraps without complaint under
  any key, so a device holding the wrong root key ends up with a different group
  key and every group frame fails. Check the root key the device was provisioned
  with, and whether it follows the 1.0.x or the 1.1 derivation.
- **The block's code fails on a device that heard everything.** Either a fragment
  changed on the way, or the code was taken over different terms: the session
  counter, the session index, the descriptor, and the length must match the setup,
  and the length leaves the padding out.
- **Fragments are dropped at the gateway.** A fragment and its three bytes of command
  must fit the downlink payload at the data rate the group listens at. Size the
  fragments for that rate, not for the fastest one.
- **The window opens and nobody is listening.** The group listens at a GPS time the
  class C session names, so a device whose clock was never synchronized opens its
  window at the wrong moment. Run the clock package first.
- **The release is refused after a perfect transfer.** The transport did its job;
  the manifest did not pass. The refusal names the rule: another signer, another
  device class, a sequence no newer than the running one, or an image that does not
  match its digest.

## Where next

<!-- table: next fuota -->
- [LoRaWAN](lorawan.md): LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join.
- [LoRaWAN gateways](gateway.md): What a LoRaWAN gateway speaks.
- [Secured session](session.md): X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack.
- [Signed updates](update.md): Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own.
- Beside it: [Radios and antennas](../radio.md).
- Also in Radio and reach: [LoRa airtime and range](lora.md), [LoRa radios](radios.md), [Mesh frames](mesh.md), [Routing](routing.md).
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
