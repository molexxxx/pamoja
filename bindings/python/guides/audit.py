"""The audit log guide example; see docs/guides/audit.md."""

# ANCHOR: example
from pamoja.audit import AuditEntry, AuditLog, verify_chain
from pamoja.security import DeviceIdentity

# The controller signs its own log with a provisioned seed. This one is RFC 8032 test
# vector 1, so the key the records are checked against is a published constant rather
# than a value checked against itself.
keeper = DeviceIdentity.from_seed(
    bytes.fromhex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
)
assert keeper.public_key.hex() == (
    "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
)

log = AuditLog(keeper)
lit = log.append(b"burner=on")
stopped = log.append(b"burner=off")

# A record's digest is SHA-256 over its little-endian index, the digest of the record
# before it, and its payload, so the first record hashes forty zero bytes and then what
# it carries.
assert lit.index == 0
assert lit.digest.hex() == (
    "e50c6a7a944fab6dd13ffdb760ca190e14ea00c168ba7c948745ba0af146c159"
)
assert stopped.previous == lit.digest
assert verify_chain(keeper.public_key, [lit, stopped]) is True

# Editing a stored record changes the digest its signature covers.
edited = bytearray(stopped.to_bytes())
edited[-1] ^= 0xFF
tampered = AuditEntry.from_bytes(bytes(edited))
assert verify_chain(keeper.public_key, [lit, tampered]) is False

# Dropping the record before it leaves the survivor chained to a link that is no longer
# there, so a shortened log is caught as readily as an edited one.
assert verify_chain(keeper.public_key, [stopped]) is False
# ANCHOR_END: example
