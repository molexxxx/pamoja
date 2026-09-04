# @pamoja/audit

A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/audit.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/audit
```

This pulls in `@pamoja/native`, the compiled engine, and `@pamoja/security`. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/audit.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/audit.ts):

```typescript
import assert from 'node:assert/strict'

import { AuditEntry, AuditLog, verifyChain } from '@pamoja/audit'
import { DeviceIdentity } from '@pamoja/security'

// The controller signs its own log with a provisioned seed. This one is RFC 8032 test
// vector 1, so the key the records are checked against is a published constant rather
// than a value checked against itself.
const keeper = DeviceIdentity.fromSeed(
  Buffer.from('9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60', 'hex')
)
assert.equal(
  keeper.publicKey().toString('hex'),
  'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a'
)

const log = new AuditLog(keeper)
const lit = log.append(Buffer.from('burner=on'))
const stopped = log.append(Buffer.from('burner=off'))

// A record's digest is SHA-256 over its little-endian index, the digest of the record
// before it, and its payload, so the first record hashes forty zero bytes and then what
// it carries.
assert.equal(lit.index, 0)
assert.equal(
  lit.digest.toString('hex'),
  'e50c6a7a944fab6dd13ffdb760ca190e14ea00c168ba7c948745ba0af146c159'
)
assert.deepEqual(stopped.previous, lit.digest)
assert.ok(verifyChain(keeper.publicKey(), [lit, stopped]))

// Editing a stored record changes the digest its signature covers.
const edited = stopped.toBytes()
edited[edited.length - 1] ^= 0xff
const tampered = AuditEntry.fromBytes(edited)
assert.ok(!verifyChain(keeper.publicKey(), [lit, tampered]))

// Dropping the record before it leaves the survivor chained to a link that is no longer
// there, so a shortened log is caught as readily as an edited one.
assert.ok(!verifyChain(keeper.publicKey(), [stopped]))
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-audit`](https://crates.io/crates/pamoja-audit) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html), [docs.rs](https://docs.rs/pamoja-audit) |
| TypeScript | [`@pamoja/audit`](https://www.npmjs.com/package/@pamoja/audit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html) |
| Python | [`pamoja-audit`](https://pypi.org/project/pamoja-audit/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html) |
| C# | [`Pamoja.Audit`](https://www.nuget.org/packages/Pamoja.Audit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html) |

## Documentation

- [`@pamoja/audit` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html), every class, function, and type this package exports.
- [The Audit log guide](https://pamoja.molex.cloud/docs/guides/audit.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
