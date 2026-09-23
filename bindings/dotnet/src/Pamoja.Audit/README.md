# Pamoja.Audit

A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/audit.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html)

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-audit`](https://crates.io/crates/pamoja-audit) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html), [docs.rs](https://docs.rs/pamoja-audit), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-audit) |
| TypeScript | [`@pamoja/audit`](https://www.npmjs.com/package/@pamoja/audit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-audit) |
| Python | [`pamoja-audit`](https://pypi.org/project/pamoja-audit/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-audit) |
| C# | [`Pamoja.Audit`](https://www.nuget.org/packages/Pamoja.Audit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-audit) |

## Documentation

- [`Pamoja.Audit` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html), every type in this namespace.
- [The Audit log guide](https://pamoja.molex.cloud/docs/guides/audit.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
