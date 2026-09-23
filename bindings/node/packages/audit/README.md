# @pamoja/audit

A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/audit.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html)

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
const seed = Buffer.alloc(32, 7)
const keeper = DeviceIdentity.fromSeed(seed)
const auditor = keeper.publicKey()

const log = new AuditLog(keeper)
const lit = log.append(Buffer.from('burner=on'))
const stopped = log.append(Buffer.from('burner=off'))
console.log(`recorded  burner=on as record ${lit.index} and burner=off as record ${stopped.index}`)

// Each record hashes its own index, the digest of the record before it, and what it
// carries, so the chain fixes the order as well as the contents.
const linked = stopped.previous.equals(lit.digest) ? 'carries' : 'does not carry'
console.log(`chained   record ${stopped.index} ${linked} the digest of record ${lit.index}`)
try {
  verifyChain(auditor, [lit, stopped])
  console.log('verified  the whole log is authentic and in order')
} catch (error) {
  console.log(`rejected  ${(error as Error).message}`)
}

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

// Dropping the first record, or swapping the two, leaves a record where its index says it
// cannot be, so a shortened or reordered log is caught as readily as an edited one.
try {
  verifyChain(auditor, [stopped])
  console.log('a shortened log verified, which should never happen')
} catch (error) {
  console.log(`shortened caught: ${(error as Error).message}`)
}
try {
  verifyChain(auditor, [stopped, lit])
  console.log('a reordered log verified, which should never happen')
} catch (error) {
  console.log(`reordered caught: ${(error as Error).message}`)
}

// A log checked against another device's key fails on the first signature.
const stranger = DeviceIdentity.fromSeed(Buffer.alloc(32, 8)).publicKey()
try {
  verifyChain(stranger, [lit, stopped])
  console.log("another device's key verified the log, which should never happen")
} catch (error) {
  console.log(`stranger  caught: ${(error as Error).message}`)
}

// After a restart the controller loads its seed again and resumes from the last record in
// storage, so the log carries on as one chain rather than starting a second.
const resumed = AuditLog.resume(DeviceIdentity.fromSeed(seed), stopped)
const relit = resumed.append(Buffer.from('burner=on'))
try {
  verifyChain(auditor, [lit, stopped, relit])
  console.log(`resumed   burner=on again as record ${relit.index}, and the whole log still verifies`)
} catch (error) {
  console.log(`rejected  ${(error as Error).message}`)
}

// What a chain cannot show is a record cut from its end, because what is left is still a
// valid chain. The auditor catches it against the last index the device reported.
const reported = relit.index
const cut = [lit, stopped]
verifyChain(auditor, cut)
const ends = cut[cut.length - 1].index
const verdict = ends < reported ? 'a record is missing' : 'nothing is missing'
console.log(
  `cut       the log verifies but ends at record ${ends}, and the device reported record ${reported}: ${verdict}`,
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

- [`@pamoja/audit` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html), every class, function, and type this package exports.
- [The Audit log guide](https://pamoja.molex.cloud/docs/guides/audit.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
