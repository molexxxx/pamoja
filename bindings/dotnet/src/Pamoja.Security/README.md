# Pamoja.Security

ed25519 device identity: sign a reading and verify it, so a gateway can prove it is authentic. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/security.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Security
```

```csharp
using Pamoja.Security;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/SecurityGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SecurityGuide.cs):

```csharp
// The seed is provisioned into the device once and never leaves it. A real one
// comes from the factory or a secure element; any 32 bytes stand in here.
byte[] seed = new byte[DeviceIdentity.KeyLength];
Array.Fill(seed, (byte)7);
using var device = new DeviceIdentity(seed);

// Only the 32-byte public key travels to the gateway. Its fingerprint is the short
// form an operator reads off a screen to tell one device from another.
byte[] gatewayKey = device.PublicKey;
Console.WriteLine($"device     {DeviceIdentity.FingerprintOf(gatewayKey)}");

// Signing is deterministic, so the same reading always produces the same 64 bytes
// and there is no randomness to get wrong on a microcontroller.
const string reading = "meter-4 1182.750 kWh";
byte[] signature = device.Sign(reading);
Console.WriteLine(DeviceIdentity.Verify(gatewayKey, reading, signature)
    ? $"accepted   {reading}"
    : "rejected   a reading the device really did sign, which should never happen");

// A digit changed in transit no longer matches what was signed.
const string edited = "meter-4 1082.750 kWh";
Console.WriteLine(DeviceIdentity.Verify(gatewayKey, edited, signature)
    ? "accepted   an edited reading, which should never happen"
    : $"rejected   {edited}");

// Nor does the same reading offered under another device's key.
byte[] impostorSeed = new byte[DeviceIdentity.KeyLength];
Array.Fill(impostorSeed, (byte)90);
using var impostor = new DeviceIdentity(impostorSeed);
Console.WriteLine(DeviceIdentity.Verify(impostor.PublicKey, reading, signature)
    ? "accepted   an impostor, which should never happen"
    : "rejected   a signature offered under another device's key");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-security`](https://crates.io/crates/pamoja-security) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html), [docs.rs](https://docs.rs/pamoja-security), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-security) |
| TypeScript | [`@pamoja/security`](https://www.npmjs.com/package/@pamoja/security) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-security) |
| Python | [`pamoja-security`](https://pypi.org/project/pamoja-security/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/security.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-security) |
| C# | [`Pamoja.Security`](https://www.nuget.org/packages/Pamoja.Security) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-security) |

## Documentation

- [`Pamoja.Security` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.html), every type in this namespace.
- [The Device identity guide](https://pamoja.molex.cloud/docs/guides/security.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
