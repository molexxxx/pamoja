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

It signs two records with the key RFC 8032 publishes for its first test vector
and checks the first record's digest against the value SHA-256 fixes for it, then
breaks the log twice: once by editing a record already in storage, and once by
dropping the record before it.

It proves:

- The chain is checked against a published key, so the signatures are anchored to
  the specification rather than round-tripped against themselves.
- A record's digest covers its little-endian index, the previous digest, and the
  payload, in that order, so an implementation that encodes any of the three
  differently fails here rather than staying wrong and self-consistent.
- Each record carries the digest of the one before it, which is the link an
  auditor follows.
- Editing a stored record breaks verification, and so does dropping one, so a
  record removed from a log is as visible as one that was altered.

## Rust

<!-- snippet: examples/tests/guides/audit.rs#example -->
From [`examples/tests/guides/audit.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/audit.rs):

```rust
use pamoja_audit::{verify_chain, AuditLog, Entry};
use pamoja_security::DeviceIdentity;

// The controller signs its own log with a provisioned seed. This one is RFC 8032 test
// vector 1, so the key the records are checked against is a published constant.
let keeper = DeviceIdentity::from_seed(&[
    0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
    0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
    0x7f, 0x60,
]);
let public = keeper.public();
assert_eq!(
    public.to_bytes(),
    [
        0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
        0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
        0xf7, 0x07, 0x51, 0x1a,
    ]
);

let mut log = AuditLog::new(keeper);
let lit = log.append(b"burner=on");
let stopped = log.append(b"burner=off");

// A record's digest is SHA-256 over its little-endian index, the digest of the record
// before it, and its payload, so the first record hashes forty zero bytes and then
// what it carries.
assert_eq!(lit.index(), 0);
assert_eq!(
    lit.digest(),
    [
        0xe5, 0x0c, 0x6a, 0x7a, 0x94, 0x4f, 0xab, 0x6d, 0xd1, 0x3f, 0xfd, 0xb7, 0x60, 0xca,
        0x19, 0x0e, 0x14, 0xea, 0x00, 0xc1, 0x68, 0xba, 0x7c, 0x94, 0x87, 0x45, 0xba, 0x0a,
        0xf1, 0x46, 0xc1, 0x59,
    ]
);
assert_eq!(stopped.previous(), lit.digest());
assert!(verify_chain(&public, &[lit.clone(), stopped.clone()]).is_ok());

// Editing a stored record changes the digest its signature covers.
let mut edited = stopped.to_bytes();
*edited.last_mut().expect("a record with a payload") ^= 0xFF;
let tampered = Entry::from_bytes(&edited).expect("a well-formed record");
assert!(verify_chain(&public, &[lit, tampered]).is_err());

// Dropping the record before it leaves the survivor chained to a link that is no
// longer there, so a shortened log is caught as readily as an edited one.
assert!(verify_chain(&public, &[stopped]).is_err());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/audit.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/audit.py#example -->
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
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/AuditGuide.cs#example -->
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
<!-- end -->

## Reference

<!-- table: reference audit -->
- Rust: [`pamoja-audit`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html)
- TypeScript: [`@pamoja/audit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html)
- Python: [`pamoja.audit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html)
- C#: [`Pamoja.Audit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html)
<!-- end -->
