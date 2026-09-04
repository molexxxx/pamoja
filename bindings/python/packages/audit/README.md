# pamoja-audit

A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-audit`](https://crates.io/crates/pamoja-audit) | [docs.rs](https://docs.rs/pamoja-audit), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html) |
| TypeScript | [`@pamoja/audit`](https://www.npmjs.com/package/@pamoja/audit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html) |
| Python | [`pamoja-audit`](https://pypi.org/project/pamoja-audit/) | [`pamoja.audit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html) |
| C# | [`Pamoja.Audit`](https://www.nuget.org/packages/Pamoja.Audit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.Audit.html) |

## Documentation

- [The Audit log guide](https://pamoja.molex.cloud/docs/guides/audit.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
