"""The device identity guide example; see docs/guides/security.md."""

# ANCHOR: example
from pamoja.security import DeviceIdentity, fingerprint, verify

# The seed is provisioned into the device and never leaves it. This one is RFC 8032 test
# vector 2, so the key it derives and the signature below are published constants rather
# than values checked against themselves.
device = DeviceIdentity.from_seed(
    bytes.fromhex("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")
)
assert device.sign(bytes([0x72])) == bytes.fromhex(
    "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da"
    "085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"
)

# Only the 32-byte public key travels to the gateway.
gateway_key = device.public_key
assert fingerprint(gateway_key) == "3d4017c3e843895a"

# Signing is deterministic, so the same reading always yields the same 64 bytes; there is
# no randomness to get wrong on a microcontroller.
reading = "meter-4 1182.750 kWh"
signature = device.sign(reading)
assert device.sign(reading) == signature
assert verify(gateway_key, reading, signature) is True

# A digit changed in transit fails, and so does a signature offered under another device's
# key.
assert verify(gateway_key, "meter-4 1082.750 kWh", signature) is False
impostor = DeviceIdentity.from_seed(bytes([0x5A]) * 32)
assert verify(impostor.public_key, reading, signature) is False
# ANCHOR_END: example
