"""The signed update guide example; see docs/guides/update.md."""

# ANCHOR: example
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
# ANCHOR_END: example

assert manifest.digest == image_digest(image)
assert opened.digest == manifest.digest
assert slot == 1
assert fleet.slot_record(1).state == SlotState.CONFIRMED
assert fleet.on_boot().action != BootAction.TRYING
