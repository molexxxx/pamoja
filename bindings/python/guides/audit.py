"""The audit log guide example; see docs/guides/audit.md."""

# ANCHOR: example
from pamoja.audit import AuditEntry, AuditLog, verify_chain
from pamoja.core import PamojaError
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
verify_chain(auditor, [lit, stopped])
print("verified  the whole log is authentic and in order")

# Editing a stored record changes the digest its signature covers.
edited = bytearray(stopped.to_bytes())
edited[-1] ^= 0xFF
tampered = AuditEntry.from_bytes(bytes(edited))
try:
    verify_chain(auditor, [lit, tampered])
    print("an edited record verified, which should never happen")
except PamojaError as error:
    print(f"edited    caught: {error}")

# Dropping the first record leaves the survivor chained to a link that is no longer there,
# so a shortened log is caught as readily as an edited one.
try:
    verify_chain(auditor, [stopped])
    print("a shortened log verified, which should never happen")
except PamojaError as error:
    print(f"shortened caught: {error}")
# ANCHOR_END: example

verify_chain(auditor, [lit, stopped])
for broken in ([lit, tampered], [stopped]):
    try:
        verify_chain(auditor, broken)
        raise AssertionError("a broken log verified")
    except PamojaError:
        pass
