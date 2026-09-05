# Audit log

A log a device keeps about itself is worth only as much as the trouble it takes
to edit afterwards. Each record here carries its position in the log, the digest
of the record before it, and a signature over that digest, so what a device wrote
is a chain rather than a pile of lines: altering a record, reordering two, or
dropping one breaks the chain at that point and at every point after it. pamoja
does not store the records. The device appends them and writes the bytes wherever
it keeps them, an SD card or a file on a gateway, and whoever audits it later
reads them back and checks the chain against the device's public key.

## What the example does

It signs two records of what a controller did, then breaks the log twice: once by
editing a record already in storage, and once by dropping the record before it.

The digest a record hashes to is pinned in the conformance vectors every binding
checks itself against, so this page shows what an auditor does with a log rather
than restating the bytes one produces.

It proves:

- Each record carries the digest of the one before it, which is the link an
  auditor follows, so the chain fixes the order as well as the contents.
- Editing a stored record breaks verification, and so does dropping one, so a
  record removed from a log is as visible as one that was altered.

## Rust

<!-- snippet: examples/tests/guides/audit.rs#example -->
From [`examples/tests/guides/audit.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/audit.rs):

```rust
use pamoja_audit::{verify_chain, AuditLog, Entry};
use pamoja_security::DeviceIdentity;

// The controller signs its own log with a provisioned seed and an auditor holds only
// the public half, so a log can be checked anywhere without the device present.
let keeper = DeviceIdentity::from_seed(&[7u8; 32]);
let auditor = keeper.public();

let mut log = AuditLog::new(keeper);
let lit = log.append(b"burner=on");
let stopped = log.append(b"burner=off");
println!("recorded  {} then {}", lit.index(), stopped.index());

// Each record hashes its own index, the digest of the record before it, and what it
// carries, so the chain fixes the order as well as the contents.
println!("chained   {}", stopped.previous() == lit.digest());
match verify_chain(&auditor, &[lit.clone(), stopped.clone()]) {
    Ok(()) => println!("verified  the whole log is authentic and in order"),
    Err(error) => println!("rejected  {error}"),
}

// Editing a stored record changes the digest its signature covers.
let mut edited = stopped.to_bytes();
*edited.last_mut().expect("a record with a payload") ^= 0xFF;
let tampered = Entry::from_bytes(&edited).expect("a well-formed record");
match verify_chain(&auditor, &[lit, tampered]) {
    Ok(()) => println!("an edited record verified, which should never happen"),
    Err(error) => println!("edited    caught: {error}"),
}

// Dropping the first record leaves the survivor chained to a link that is no longer
// there, so a shortened log is caught as readily as an edited one.
match verify_chain(&auditor, &[stopped]) {
    Ok(()) => println!("a shortened log verified, which should never happen"),
    Err(error) => println!("shortened caught: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/audit.ts#example -->
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
console.log(`verified  ${verifyChain(auditor, [lit, stopped])}`)

// Editing a stored record changes the digest its signature covers.
const edited = Buffer.from(stopped.toBytes())
edited[edited.length - 1] ^= 0xff
const tampered = AuditEntry.fromBytes(edited)
console.log(`edited    caught: ${!verifyChain(auditor, [lit, tampered])}`)

// Dropping the first record leaves the survivor chained to a link that is no longer there,
// so a shortened log is caught as readily as an edited one.
console.log(`shortened caught: ${!verifyChain(auditor, [stopped])}`)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/audit.py#example -->
From [`bindings/python/guides/audit.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/audit.py):

```python
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
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/AuditGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/AuditGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/AuditGuide.cs):

```csharp
// The controller signs its own log with a provisioned seed and an auditor holds
// only the public half, so a log can be checked anywhere without the device.
byte[] seed = new byte[32];
Array.Fill(seed, (byte)7);
using var keeper = new DeviceIdentity(seed);
byte[] auditor = keeper.PublicKey;

using var log = new AuditLog(keeper);
using AuditEntry lit = log.Append("burner=on"u8);
using AuditEntry stopped = log.Append("burner=off"u8);
Console.WriteLine($"recorded  {lit.Index} then {stopped.Index}");

// Each record hashes its own index, the digest of the record before it, and what
// it carries, so the chain fixes the order as well as the contents.
Console.WriteLine($"chained   {stopped.Previous.SequenceEqual(lit.Digest)}");
Console.WriteLine($"verified  {Audit.VerifyChain(auditor, [lit, stopped])}");

// Editing a stored record changes the digest its signature covers.
byte[] edited = stopped.ToBytes();
edited[^1] ^= 0xFF;
using AuditEntry tampered = AuditEntry.FromBytes(edited);
Console.WriteLine($"edited    caught: {!Audit.VerifyChain(auditor, [lit, tampered])}");

// Dropping the first record leaves the survivor chained to a link that is no
// longer there, so a shortened log is caught as readily as an edited one.
Console.WriteLine($"shortened caught: {!Audit.VerifyChain(auditor, [stopped])}");
```
<!-- end -->

## Reference

<!-- table: reference audit -->
- Rust: [`pamoja-audit`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html)
- TypeScript: [`@pamoja/audit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html)
- Python: [`pamoja.audit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html)
- C#: [`Pamoja.Audit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html)
<!-- end -->
