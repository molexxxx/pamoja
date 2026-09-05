"""The audit log guide example; see docs/guides/audit.md."""

# ANCHOR: example
from pamoja.audit import AuditEntry, AuditLog, verify_chain
from pamoja.security import DeviceIdentity

# The controller signs its own log with a provisioned seed and an auditor holds only the
# public half, so a log can be checked anywhere without the device present.
keeper = DeviceIdentity.from_seed(bytes([7]) * 32)
auditor = keeper.public_key

log = AuditLog(keeper)
lit = log.append(b"burner=on")
stopped = log.append(b"burner=off")
print(f"recorded  {lit.index} then {stopped.index}")

# Each record hashes its own index, the digest of the record before it, and what it
# carries, so the chain fixes the order as well as the contents.
print(f"chained   {stopped.previous == lit.digest}")
print(f"verified  {verify_chain(auditor, [lit, stopped])}")

# Editing a stored record changes the digest its signature covers.
edited = bytearray(stopped.to_bytes())
edited[-1] ^= 0xFF
tampered = AuditEntry.from_bytes(bytes(edited))
print(f"edited    caught: {not verify_chain(auditor, [lit, tampered])}")

# Dropping the first record leaves the survivor chained to a link that is no longer there,
# so a shortened log is caught as readily as an edited one.
print(f"shortened caught: {not verify_chain(auditor, [stopped])}")
# ANCHOR_END: example

assert verify_chain(auditor, [lit, stopped]) is True
assert verify_chain(auditor, [lit, tampered]) is False
assert verify_chain(auditor, [stopped]) is False
