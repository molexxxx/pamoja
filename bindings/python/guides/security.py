"""The device identity guide example; see docs/guides/security.md."""

# ANCHOR: example
from pamoja.security import DeviceIdentity, fingerprint, verify

# The seed is provisioned into the device once and never leaves it. A real one comes from
# the factory or a secure element; any 32 bytes stand in here.
device = DeviceIdentity.from_seed(bytes([7]) * 32)

# Only the 32-byte public key travels to the gateway. Its fingerprint is the short form an
# operator reads off a screen to tell one device from another.
gateway_key = device.public_key
print(f"device     {fingerprint(gateway_key)}")

# Signing is deterministic, so the same reading always produces the same 64 bytes and there
# is no randomness to get wrong on a microcontroller.
reading = "meter-4 1182.750 kWh"
signature = device.sign(reading)
if verify(gateway_key, reading, signature):
    print(f"accepted   {reading}")
else:
    print("rejected   a reading the device really did sign, which should never happen")

# A digit changed in transit no longer matches what was signed.
edited = "meter-4 1082.750 kWh"
if verify(gateway_key, edited, signature):
    print("accepted   an edited reading, which should never happen")
else:
    print(f"rejected   {edited}")

# Nor does the same reading offered under another device's key.
impostor = DeviceIdentity.from_seed(bytes([90]) * 32)
if verify(impostor.public_key, reading, signature):
    print("accepted   an impostor, which should never happen")
else:
    print("rejected   a signature offered under another device's key")
# ANCHOR_END: example

assert device.sign(reading) == signature
assert verify(gateway_key, reading, signature) is True
assert verify(gateway_key, edited, signature) is False
assert verify(impostor.public_key, reading, signature) is False
