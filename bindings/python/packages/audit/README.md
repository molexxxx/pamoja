# pamoja-audit

A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/audit.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html)

## Install

```sh
pip install pamoja-audit
```

```python
from pamoja import audit
```

This pulls in `pamoja-native`, the compiled engine, and `pamoja-security`. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/audit.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/audit.py):

```python
from pamoja.audit import AuditEntry, AuditLog, verify_chain
from pamoja.core import PamojaError
from pamoja.security import DeviceIdentity

# The controller signs its own log with a provisioned seed and an auditor holds only the
# public half, so a log can be checked anywhere without the device present.
seed = bytes([7]) * 32
keeper = DeviceIdentity.from_seed(seed)
auditor = keeper.public_key

log = AuditLog(keeper)
lit = log.append(b"burner=on")
stopped = log.append(b"burner=off")
print(f"recorded  burner=on as record {lit.index} and burner=off as record {stopped.index}")

# Each record hashes its own index, the digest of the record before it, and what it
# carries, so the chain fixes the order as well as the contents.
linked = "carries" if stopped.previous == lit.digest else "does not carry"
print(f"chained   record {stopped.index} {linked} the digest of record {lit.index}")
try:
    verify_chain(auditor, [lit, stopped])
    print("verified  the whole log is authentic and in order")
except PamojaError as error:
    print(f"rejected  {error}")

# Editing a stored record changes the digest its signature covers.
edited = bytearray(stopped.to_bytes())
edited[-1] ^= 0xFF
tampered = AuditEntry.from_bytes(bytes(edited))
try:
    verify_chain(auditor, [lit, tampered])
    print("an edited record verified, which should never happen")
except PamojaError as error:
    print(f"edited    caught: {error}")

# Dropping the first record, or swapping the two, leaves a record where its index says it
# cannot be, so a shortened or reordered log is caught as readily as an edited one.
try:
    verify_chain(auditor, [stopped])
    print("a shortened log verified, which should never happen")
except PamojaError as error:
    print(f"shortened caught: {error}")
try:
    verify_chain(auditor, [stopped, lit])
    print("a reordered log verified, which should never happen")
except PamojaError as error:
    print(f"reordered caught: {error}")

# A log checked against another device's key fails on the first signature.
stranger = DeviceIdentity.from_seed(bytes([8]) * 32).public_key
try:
    verify_chain(stranger, [lit, stopped])
    print("another device's key verified the log, which should never happen")
except PamojaError as error:
    print(f"stranger  caught: {error}")

# After a restart the controller loads its seed again and resumes from the last record in
# storage, so the log carries on as one chain rather than starting a second.
resumed = AuditLog.resume(DeviceIdentity.from_seed(seed), stopped)
relit = resumed.append(b"burner=on")
try:
    verify_chain(auditor, [lit, stopped, relit])
    print(f"resumed   burner=on again as record {relit.index}, and the whole log still verifies")
except PamojaError as error:
    print(f"rejected  {error}")

# What a chain cannot show is a record cut from its end, because what is left is still a
# valid chain. The auditor catches it against the last index the device reported.
reported = relit.index
cut = [lit, stopped]
verify_chain(auditor, cut)
ends = cut[-1].index
verdict = "a record is missing" if ends < reported else "nothing is missing"
print(
    f"cut       the log verifies but ends at record {ends}, "
    f"and the device reported record {reported}: {verdict}"
)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-audit`](https://crates.io/crates/pamoja-audit) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html), [docs.rs](https://docs.rs/pamoja-audit), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-audit) |
| TypeScript | [`@pamoja/audit`](https://www.npmjs.com/package/@pamoja/audit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-audit) |
| Python | [`pamoja-audit`](https://pypi.org/project/pamoja-audit/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-audit) |
| C# | [`Pamoja.Audit`](https://www.nuget.org/packages/Pamoja.Audit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-audit) |

## Documentation

- [`pamoja.audit` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html), every class and function in this module.
- [The Audit log guide](https://pamoja.molex.cloud/docs/guides/audit.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
