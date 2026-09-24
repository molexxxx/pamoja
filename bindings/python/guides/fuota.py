"""The firmware update over the air guide example; see docs/guides/fuota.md."""

# ANCHOR: example
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
# ANCHOR_END: example

assert intact
assert slot == 1
assert carried_image == image
assert len(payload_key) == 16

# ANCHOR: losses
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
# ANCHOR_END: losses

assert caught
assert far.missing == 0
