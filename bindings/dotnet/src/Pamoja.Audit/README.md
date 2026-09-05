# Pamoja.Audit

A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/audit.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-audit`](https://crates.io/crates/pamoja-audit) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html), [docs.rs](https://docs.rs/pamoja-audit) |
| TypeScript | [`@pamoja/audit`](https://www.npmjs.com/package/@pamoja/audit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_audit.html) |
| Python | [`pamoja-audit`](https://pypi.org/project/pamoja-audit/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/audit.html) |
| C# | [`Pamoja.Audit`](https://www.nuget.org/packages/Pamoja.Audit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html) |

## Documentation

- [`Pamoja.Audit` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Audit.html), every type in this namespace.
- [The Audit log guide](https://pamoja.molex.cloud/docs/guides/audit.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
