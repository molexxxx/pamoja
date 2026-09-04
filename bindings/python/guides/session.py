"""The secured session guide example; see docs/guides/session.md."""

# ANCHOR: example
from pamoja.core import PamojaError
from pamoja.session import AgreementKey, Role, Session

# Each device is provisioned with a 32-byte seed and publishes the key it derives. These
# are the X25519 pair RFC 7748 section 6.1 publishes, so the derivation is pinned to the
# specification rather than checked against itself.
node = AgreementKey(
    bytes.fromhex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
)
gateway = AgreementKey(
    bytes.fromhex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")
)
assert node.public_key.hex() == (
    "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"
)

# Neither side sends the session key. Both derive it from the shared secret, a salt that
# travels in the clear, and both public keys. The roles have to be opposite.
salt = bytes([0x09]) * 16
uplink = Session(node, gateway.public_key, salt, Role.INITIATOR)
downlink = Session(gateway, node.public_key, salt, Role.RESPONDER)

# The pump id is authenticated but not encrypted, so a router still reads it while any
# change to it fails the tag.
sealed = uplink.seal(b"flow=41.2", b"pump-3")
assert sealed.ciphertext != b"flow=41.2"
assert downlink.open(sealed, b"pump-3") == b"flow=41.2"

# The anti-replay window refuses a counter it has already accepted, so a frame captured
# off the air and sent again is not delivered a second time.
try:
    downlink.open(sealed, b"pump-3")
except PamojaError:
    pass
else:
    raise AssertionError("a replayed message should be refused")
# ANCHOR_END: example
