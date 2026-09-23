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
`burner=off`, and breaks the log every way a log gets broken: a record edited in
storage, the first record left out, the two swapped, and the whole log checked
against another device's key. Then the controller restarts, picks the log up
where it left off, and writes a third record; and finally the log is cut back to
its first two records, which is the one change a chain cannot see by itself.

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
  device's key, and the second record's link is the digest of the first.
- A record edited in storage still parses and still carries the device's
  signature, but the digest recomputed from its fields no longer matches it, so
  verification fails.
- A log missing its first record, and one with its records swapped, are both
  rejected: each record's index says where it has to be.
- Another device's public key does not verify the log at all.
- A controller that restarts and resumes from its last stored record writes record
  2 onto the same chain, and the three records verify as one log.
- A log cut back to its first two records still verifies. The auditor catches the
  missing record only by comparing the last index with the one the device reported.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example audit" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example audit</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- audit" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- audit</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/audit.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/audit.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- audit" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- audit</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-audit` holds three pieces. An `AuditLog` owns the device's
`DeviceIdentity` from `pamoja-security`, and `append` signs a payload and returns
its `Entry`; `AuditLog::resume` takes the identity again and the last stored
entry after a restart. An `Entry` gives back its `index()`, `previous()` link,
`payload()`, `signature()`, and recomputed `digest()`, and crosses storage as
`to_bytes()` and `Entry::from_bytes`. `verify_chain` checks a whole log from its
first record, and a `Verifier` checks one record at a time as they arrive, which
suits a gateway reading a log off a card. Every failure is `Error::Auth` with the
reason, except a record too short to hold a header, which is `Error::Codec`.

<!-- snippet: examples/guides/audit.rs#example -->
From [`examples/guides/audit.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/audit.rs):

```rust
use pamoja_audit::{verify_chain, AuditLog, Entry};
use pamoja_security::DeviceIdentity;

// The controller signs its own log with a provisioned seed and an auditor holds only
// the public half, so a log can be checked anywhere without the device present.
let seed = [7u8; 32];
let keeper = DeviceIdentity::from_seed(&seed);
let auditor = keeper.public();

let mut log = AuditLog::new(keeper);
let lit = log.append(b"burner=on");
let stopped = log.append(b"burner=off");
println!(
    "recorded  burner=on as record {} and burner=off as record {}",
    lit.index(),
    stopped.index()
);

// Each record hashes its own index, the digest of the record before it, and what it
// carries, so the chain fixes the order as well as the contents.
let linked = if stopped.previous() == lit.digest() {
    "carries"
} else {
    "does not carry"
};
println!(
    "chained   record {} {linked} the digest of record {}",
    stopped.index(),
    lit.index()
);
match verify_chain(&auditor, &[lit.clone(), stopped.clone()]) {
    Ok(()) => println!("verified  the whole log is authentic and in order"),
    Err(error) => println!("rejected  {error}"),
}

// Editing a stored record changes the digest its signature covers.
let mut edited = stopped.to_bytes();
*edited.last_mut().expect("a record with a payload") ^= 0xFF;
let tampered = Entry::from_bytes(&edited).expect("a well-formed record");
match verify_chain(&auditor, &[lit.clone(), tampered]) {
    Ok(()) => println!("an edited record verified, which should never happen"),
    Err(error) => println!("edited    caught: {error}"),
}

// Dropping the first record, or swapping the two, leaves a record where its index says
// it cannot be, so a shortened or reordered log is caught as readily as an edited one.
let shortened = [stopped.clone()];
match verify_chain(&auditor, &shortened) {
    Ok(()) => println!("a shortened log verified, which should never happen"),
    Err(error) => println!("shortened caught: {error}"),
}
match verify_chain(&auditor, &[stopped.clone(), lit.clone()]) {
    Ok(()) => println!("a reordered log verified, which should never happen"),
    Err(error) => println!("reordered caught: {error}"),
}

// A log checked against another device's key fails on the first signature.
let stranger = DeviceIdentity::from_seed(&[8u8; 32]).public();
match verify_chain(&stranger, &[lit.clone(), stopped.clone()]) {
    Ok(()) => println!("another device's key verified the log, which should never happen"),
    Err(error) => println!("stranger  caught: {error}"),
}

// After a restart the controller loads its seed again and resumes from the last record
// in storage, so the log carries on as one chain rather than starting a second.
let mut resumed = AuditLog::resume(DeviceIdentity::from_seed(&seed), &stopped);
let relit = resumed.append(b"burner=on");
match verify_chain(&auditor, &[lit.clone(), stopped.clone(), relit.clone()]) {
    Ok(()) => println!(
        "resumed   burner=on again as record {}, and the whole log still verifies",
        relit.index()
    ),
    Err(error) => println!("rejected  {error}"),
}

// What a chain cannot show is a record cut from its end, because what is left is still
// a valid chain. The auditor catches it against the last index the device reported.
let reported = relit.index();
let cut = [lit, stopped];
if verify_chain(&auditor, &cut).is_ok() {
    let ends = cut[cut.len() - 1].index();
    let verdict = if ends < reported {
        "a record is missing"
    } else {
        "nothing is missing"
    };
    println!(
        "cut       the log verifies but ends at record {ends}, and the device reported \
         record {reported}: {verdict}"
    );
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/audit` exports `AuditLog`, built from a `DeviceIdentity`
from `@pamoja/security`, with `append(payload)` returning an `AuditEntry` and
`AuditLog.resume(identity, last)` for a restart. An entry's `index`, `previous`,
`digest`, `payload`, and `signature` are properties, the byte ones `Buffer`s, and
it crosses storage as `toBytes()` and `AuditEntry.fromBytes`. `verifyChain` takes
the public key's bytes and the entries, and throws with the reason when the chain
does not hold. An `AuditVerifier` checks one record at a time, and its `check`
answers `true` or `false` rather than throwing.

<!-- snippet: bindings/node/guides/audit.ts#example -->
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
<!-- end -->

## Python

In Python, `pamoja.audit` exports `AuditLog(identity)`, with `append(payload)`
returning an `AuditEntry` and `AuditLog.resume(identity, last)` for a restart. An
entry's `index`, `previous`, `digest`, `payload`, and `signature` are properties,
and it crosses storage as `to_bytes()` and `AuditEntry.from_bytes`.
`verify_chain(public_key, entries)` raises `PamojaError` with the reason. An
`AuditVerifier` checks one record at a time, and its `check` returns `True` or
`False`.

<!-- snippet: bindings/python/guides/audit.py#example -->
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
<!-- end -->

## C#

In C#, `Pamoja.Audit` holds `AuditLog`, built from a `DeviceIdentity`, with
`Append` returning an `AuditEntry` and `AuditLog.Resume(identity, last)` for a
restart. An entry's `Index`, `Previous`, `Digest`, `Payload`, and `Signature` are
properties, and it crosses storage as `ToBytes()` and `AuditEntry.FromBytes`.
`Audit.VerifyChain(publicKey, entries)` throws `PamojaException` with the reason,
and an `AuditVerifier`'s `Check` returns `true` or `false`. A log, an entry, and a
verifier each hold a native handle and belong in a `using`.

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
Console.WriteLine($"recorded  burner=on as record {lit.Index} and burner=off as record {stopped.Index}");

// Each record hashes its own index, the digest of the record before it, and what
// it carries, so the chain fixes the order as well as the contents.
string linked = stopped.Previous.SequenceEqual(lit.Digest) ? "carries" : "does not carry";
Console.WriteLine($"chained   record {stopped.Index} {linked} the digest of record {lit.Index}");
try
{
    Audit.VerifyChain(auditor, [lit, stopped]);
    Console.WriteLine("verified  the whole log is authentic and in order");
}
catch (PamojaException error)
{
    Console.WriteLine($"rejected  {error.Message}");
}

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

// Dropping the first record, or swapping the two, leaves a record where its index
// says it cannot be, so a shortened or reordered log is caught as readily as an
// edited one.
try
{
    Audit.VerifyChain(auditor, [stopped]);
    Console.WriteLine("a shortened log verified, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"shortened caught: {error.Message}");
}

try
{
    Audit.VerifyChain(auditor, [stopped, lit]);
    Console.WriteLine("a reordered log verified, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"reordered caught: {error.Message}");
}

// A log checked against another device's key fails on the first signature.
byte[] strangerSeed = new byte[32];
Array.Fill(strangerSeed, (byte)8);
using var strangerDevice = new DeviceIdentity(strangerSeed);
byte[] stranger = strangerDevice.PublicKey;
try
{
    Audit.VerifyChain(stranger, [lit, stopped]);
    Console.WriteLine("another device's key verified the log, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"stranger  caught: {error.Message}");
}

// After a restart the controller loads its seed again and resumes from the last
// record in storage, so the log carries on as one chain rather than starting a
// second.
using var restarted = new DeviceIdentity(seed);
using var resumed = AuditLog.Resume(restarted, stopped);
using AuditEntry relit = resumed.Append("burner=on"u8);
try
{
    Audit.VerifyChain(auditor, [lit, stopped, relit]);
    Console.WriteLine($"resumed   burner=on again as record {relit.Index}, and the whole log still verifies");
}
catch (PamojaException error)
{
    Console.WriteLine($"rejected  {error.Message}");
}

// What a chain cannot show is a record cut from its end, because what is left is
// still a valid chain. The auditor catches it against the last index the device
// reported.
ulong reported = relit.Index;
AuditEntry[] cut = [lit, stopped];
Audit.VerifyChain(auditor, cut);
ulong ends = cut[^1].Index;
string verdict = ends < reported ? "a record is missing" : "nothing is missing";
Console.WriteLine(
    $"cut       the log verifies but ends at record {ends}, and the device reported record {reported}: {verdict}");
```
<!-- end -->

## Values at a glance

**What a record holds,** in the order `to_bytes` writes it:

| Field | Size | What it is |
| --- | --- | --- |
| index | 8 bytes, little-endian | the record's position in the log, from 0 |
| previous | 32 bytes | the digest of the record before, or 32 zero bytes for the first |
| signature | 64 bytes | the device's Ed25519 signature over this record's digest |
| payload | the rest | whatever the device recorded |

The digest a record's signature covers, and the next record links to, is SHA-256
over the index, the previous digest, and the payload. A record shorter than the
104 bytes before its payload does not parse.

**What each check catches:**

| The log | What happens |
| --- | --- |
| intact, from its first record | verifies |
| a record edited | the recomputed digest no longer matches the signature: signature verification failed |
| a record removed from the start or the middle | a record's index is not the next one expected: out of sequence |
| two records swapped | out of sequence |
| a record whose link does not match the one before | the chain is broken |
| checked against another device's key | signature verification failed |
| records removed from the end | verifies, because what is left is a valid chain; compare the last index with the device's report |

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| start a log | `AuditLog::new(identity)` |
| carry on after a restart | `AuditLog::resume(identity, &last)` |
| record something | `log.append(payload)` gives an `Entry` |
| store and load a record | `entry.to_bytes()`, `Entry::from_bytes(&bytes)` |
| read a record | `index()`, `previous()`, `payload()`, `signature()`, `digest()` |
| check a whole log | `verify_chain(&public, &entries)` |
| check records as they arrive | `Verifier::new(public)`, then `check(&entry)` |

### TypeScript

| To | Call |
| --- | --- |
| start a log | `new AuditLog(identity)` |
| carry on after a restart | `AuditLog.resume(identity, last)` |
| record something | `log.append(payload)` gives an `AuditEntry` |
| store and load a record | `entry.toBytes()`, `AuditEntry.fromBytes(bytes)` |
| read a record | `index`, `previous`, `payload`, `signature`, `digest` |
| check a whole log | `verifyChain(publicKey, entries)`, which throws |
| check records as they arrive | `new AuditVerifier(publicKey)`, then `check(entry)` gives `true` or `false` |

### Python

| To | Call |
| --- | --- |
| start a log | `AuditLog(identity)` |
| carry on after a restart | `AuditLog.resume(identity, last)` |
| record something | `log.append(payload)` gives an `AuditEntry` |
| store and load a record | `entry.to_bytes()`, `AuditEntry.from_bytes(data)` |
| read a record | `index`, `previous`, `payload`, `signature`, `digest` |
| check a whole log | `verify_chain(public_key, entries)`, which raises `PamojaError` |
| check records as they arrive | `AuditVerifier(public_key)`, then `check(entry)` gives `True` or `False` |

### C#

| To | Call |
| --- | --- |
| start a log | `new AuditLog(identity)` |
| carry on after a restart | `AuditLog.Resume(identity, last)` |
| record something | `log.Append(payload)` gives an `AuditEntry` |
| store and load a record | `entry.ToBytes()`, `AuditEntry.FromBytes(bytes)` |
| read a record | `Index`, `Previous`, `Payload`, `Signature`, `Digest` |
| check a whole log | `Audit.VerifyChain(publicKey, entries)`, which throws `PamojaException` |
| check records as they arrive | `new AuditVerifier(publicKey)`, then `Check(entry)` gives `true` or `false` |

<!-- languages end -->

## When it goes wrong

What verification says, the same in every language:

| What happened | The message |
| --- | --- |
| a record was edited, or the log is checked against the wrong key | `authentication error: signature verification failed` |
| a record is missing before the last, or two are out of order | `authentication error: audit entry is out of sequence` |
| a record's link does not match the record before it | `authentication error: audit chain is broken` |
| the stored bytes are too short to be a record | `codec error: audit entry is shorter than its header` |

The mistakes that cost an afternoon:

- **A log cut short passes.** Verification proves nothing is missing before the last
  record it was given, not that the last record is the last one written. Have the device
  report its latest index, in a heartbeat or a telemetry snapshot, and compare.
- **A restart starts a second log.** A device that builds a new log after a reboot writes
  record 0 again, which breaks the chain for anyone reading the whole card. Resume from the
  last record in storage.
- **A log from the middle will not verify.** Verification starts at record 0 and the zero
  link, so a log whose early records were pruned fails at its first record. Keep a log's
  first records along with the rest.
- **The verifier says false and nothing else.** A binding's `check` reports whether a
  record is good; to know why one is not, run the records through `verify_chain`, which
  names the fault.
- **The device's key is in the log it signs.** An auditor that takes the public key from
  the device it is auditing proves only that the log agrees with itself. Record the key
  when the device is provisioned, and check logs against that.

## Where next

<!-- table: next audit -->
- [Device identity](security.md): ed25519 device identity.
- [Signed updates](update.md): Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own.
- [Store and forward](sync.md): Offline-first queues in memory or on disk, bounded or not, the on-disk one surviving power loss, and the drain that forwards them in order when a link returns.
- Also in Trust and operation: [Secured session](session.md), [Power](power.md), [Telemetry](telemetry.md).
<!-- end -->

## Reference

<!-- table: reference audit -->
- Rust: [`pamoja-audit`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-audit)
- TypeScript: [`@pamoja/audit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-audit)
- Python: [`pamoja.audit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-audit)
- C#: [`Pamoja.Audit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-audit)
<!-- end -->
