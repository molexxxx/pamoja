# Pamoja.Audit

A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
dotnet add package Pamoja.Audit
```

```csharp
using Pamoja.Audit;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Codec` and `Pamoja.Security`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/AuditGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/AuditGuide.cs):

```csharp
// The controller signs its own log with a provisioned seed. This one is RFC 8032
// test vector 1, so the key the records are checked against is a published
// constant rather than a value checked against itself.
using var keeper = new DeviceIdentity(Convert.FromHexString(
    "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
Expect(
    Convert.ToHexString(keeper.PublicKey).ToLowerInvariant()
        == "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
    "the key a chain is checked against is the one the vector publishes");

using var log = new AuditLog(keeper);
using AuditEntry lit = log.Append("burner=on"u8);
using AuditEntry stopped = log.Append("burner=off"u8);

// A record's digest is SHA-256 over its little-endian index, the digest of the
// record before it, and its payload, so the first record hashes forty zero bytes
// and then what it carries.
Expect(lit.Index == 0, "the first record sits at index zero");
Expect(
    Convert.ToHexString(lit.Digest).ToLowerInvariant()
        == "e50c6a7a944fab6dd13ffdb760ca190e14ea00c168ba7c948745ba0af146c159",
    "the digest is the one the chain construction fixes");
Expect(
    stopped.Previous.SequenceEqual(lit.Digest),
    "each record carries the hash of the one before it");
Expect(Audit.VerifyChain(keeper.PublicKey, [lit, stopped]), "an untouched chain verifies");

// Editing a stored record changes the digest its signature covers.
byte[] edited = stopped.ToBytes();
edited[^1] ^= 0xFF;
using AuditEntry tampered = AuditEntry.FromBytes(edited);
Expect(
    !Audit.VerifyChain(keeper.PublicKey, [lit, tampered]),
    "and an edited record does not");

// Dropping the record before it leaves the survivor chained to a link that is no
// longer there, so a shortened log is caught as readily as an edited one.
Expect(
    !Audit.VerifyChain(keeper.PublicKey, [stopped]),
    "a log with its first record removed does not verify either");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-audit`](https://crates.io/crates/pamoja-audit) | [docs.rs](https://docs.rs/pamoja-audit), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html) |
| TypeScript | [`@pamoja/audit`](https://www.npmjs.com/package/@pamoja/audit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html) |
| Python | [`pamoja-audit`](https://pypi.org/project/pamoja-audit/) | [`pamoja.audit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html) |
| C# | [`Pamoja.Audit`](https://www.nuget.org/packages/Pamoja.Audit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.Audit.html) |

## Documentation

- [The Audit log guide](https://pamoja.molex.cloud/docs/guides/audit.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
