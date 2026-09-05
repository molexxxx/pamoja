"""The secured session guide example; see docs/guides/session.md."""

# ANCHOR: example
import os

from pamoja.core import PamojaError
from pamoja.session import AgreementKey, Role, Session

# Each device is provisioned with a 32-byte seed and publishes the key it derives. A real
# seed comes from the factory or a secure element; any 32 bytes stand in here.
node = AgreementKey(bytes([7]) * 32)
gateway = AgreementKey(bytes([9]) * 32)

# Neither side sends the session key. Both derive it from the shared secret, a salt that
# travels in the clear, and both public keys, with opposite roles.
#
# The salt must be fresh for every session: reusing one derives the same key from the same
# pair of devices twice. The initiator draws it and sends it in the clear, so the responder
# uses the salt it received rather than one of its own.
salt = os.urandom(16)
uplink = Session(node, gateway.public_key, salt, Role.INITIATOR)
downlink = Session(gateway, node.public_key, salt, Role.RESPONDER)
print("both sides derived a key without sending one")

# The pump id is authenticated but not encrypted, so a router still reads it while any
# change to it fails the tag.
sealed = uplink.seal(b"flow=41.2", b"pump-3")
print(f"sealed    the reading is no longer readable: {sealed.ciphertext != b'flow=41.2'}")
print(f"opened    {downlink.open(sealed, b'pump-3').decode()}")

# The anti-replay window refuses a counter it has already accepted, so a frame captured
# off the air and sent again is not delivered a second time.
try:
    downlink.open(sealed, b"pump-3")
    print("a replayed frame was accepted, which should never happen")
except PamojaError as error:
    print(f"replay    refused: {error}")
# ANCHOR_END: example

assert sealed.ciphertext != b"flow=41.2"
