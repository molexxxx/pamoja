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
import { AuditEntry, AuditLog, verifyChain } from '@pamoja/audit'
import { DeviceIdentity } from '@pamoja/security'

// The controller signs its own log with a provisioned seed and an auditor holds only the
// public half, so a log can be checked anywhere without the device present.
const keeper = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))
const auditor = keeper.publicKey()

const log = new AuditLog(keeper)
const lit = log.append(Buffer.from('burner=on'))
const stopped = log.append(Buffer.from('burner=off'))
console.log(`recorded  ${lit.index} then ${stopped.index}`)

// Each record hashes its own index, the digest of the record before it, and what it
// carries, so the chain fixes the order as well as the contents.
console.log(`chained   ${stopped.previous.equals(lit.digest)}`)
verifyChain(auditor, [lit, stopped])
console.log('verified  the whole log is authentic and in order')

// Editing a stored record changes the digest its signature covers.
const edited = Buffer.from(stopped.toBytes())
edited[edited.length - 1] ^= 0xff
const tampered = AuditEntry.fromBytes(edited)
try {
  verifyChain(auditor, [lit, tampered])
  console.log('an edited record verified, which should never happen')
} catch (error) {
  console.log(`edited    caught: ${(error as Error).message}`)
}

// Dropping the first record leaves the survivor chained to a link that is no longer there,
// so a shortened log is caught as readily as an edited one.
try {
  verifyChain(auditor, [stopped])
  console.log('a shortened log verified, which should never happen')
} catch (error) {
  console.log(`shortened caught: ${(error as Error).message}`)
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-audit`](https://crates.io/crates/pamoja-audit) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html), [docs.rs](https://docs.rs/pamoja-audit), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-audit) |
| TypeScript | [`@pamoja/audit`](https://www.npmjs.com/package/@pamoja/audit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-audit) |
| Python | [`pamoja-audit`](https://pypi.org/project/pamoja-audit/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-audit) |
| C# | [`Pamoja.Audit`](https://www.nuget.org/packages/Pamoja.Audit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-audit) |

## Documentation

- [`@pamoja/audit` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html), every class, function, and type this package exports.
- [The Audit log guide](https://pamoja.molex.cloud/docs/guides/audit.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
