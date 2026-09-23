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
# ANCHOR_END: example

assert manifest.digest == image_digest(image)
assert opened.digest == manifest.digest
assert slot == 1
assert decision.action == BootAction.TRYING
assert fleet.slot_record(1).state == SlotState.CONFIRMED
assert trial.action == BootAction.TRYING and trial.slot == 0
assert after.action == BootAction.REVERTED
assert (after.slot, after.fallback) == (0, 1)
assert fleet.slot_record(0).state == SlotState.FAILED
assert fleet.installed_sequence == 3
