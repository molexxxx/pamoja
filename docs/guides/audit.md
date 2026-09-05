# Audit log

A log a device keeps about itself is worth only as much as the trouble it takes
to edit afterwards. Each record here carries its position in the log, the digest
of the record before it, and the payload. A record's own digest covers those
three and the device signs it, so what a device wrote is a chain rather than a
pile of lines: altering a record, reordering two, or dropping one breaks the
chain at that point and at every point after it. pamoja does not store the
records. The device appends them and writes the bytes wherever it keeps them, an
SD card or a file on a gateway, and whoever audits it later reads them back and
checks the chain against the device's public key.

## What the example does

It signs two records of what a burner controller did, `burner=on` then
`burner=off`, then breaks the log twice: once by editing a record already
written to storage, and once by leaving out the first record altogether.

The edited record is not a constant typed out by hand. It is the record's own
stored bytes with the last byte flipped, parsed back into a record the auditor
accepts as well formed. That byte lands in the payload, because the index, the
link and the signature come before it in the encoding.

A record does not carry its own digest. The auditor recomputes it from the
index, the link and the payload, and checks the signature against that. The
digests themselves are pinned in the conformance vectors every binding checks
itself against, so this page follows what an auditor does with a log rather than
restating the bytes one produces.

It proves:

- The two records verify in order against nothing but the public half of the
  device's key.
- The second record's link is the digest of the first, so the chain fixes the
  order as well as the contents.
- A record edited in storage still parses and still carries the device's
  signature, but the digest recomputed from its fields no longer matches it, so
  verification fails.
- A log missing its first record is rejected as well: the survivor's index and
  its link both say a record came before it.

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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/audit.py#example -->
From [`bindings/python/guides/audit.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/audit.py):

```python
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
Audit.VerifyChain(auditor, [lit, stopped]);
Console.WriteLine("verified  the whole log is authentic and in order");

// Editing a stored record changes the digest its signature covers.
byte[] edited = stopped.ToBytes();
edited[^1] ^= 0xFF;
using AuditEntry tampered = AuditEntry.FromBytes(edited);
try
{
    Audit.VerifyChain(auditor, [lit, tampered]);
    Console.WriteLine("an edited record verified, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"edited    caught: {error.Message}");
}

// Dropping the first record leaves the survivor chained to a link that is no
// longer there, so a shortened log is caught as readily as an edited one.
try
{
    Audit.VerifyChain(auditor, [stopped]);
    Console.WriteLine("a shortened log verified, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"shortened caught: {error.Message}");
}
```
<!-- end -->

## Reference

<!-- table: reference audit -->
- Rust: [`pamoja-audit`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html)
- TypeScript: [`@pamoja/audit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html)
- Python: [`pamoja.audit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html)
- C#: [`Pamoja.Audit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html)
<!-- end -->
