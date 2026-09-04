"""The signed-update guide example; see docs/guides/update.md."""

# ANCHOR: example
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
# ANCHOR_END: example
