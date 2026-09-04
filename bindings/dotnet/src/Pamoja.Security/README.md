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
// The seed is provisioned into the device and never leaves it. This one is
// RFC 8032 test vector 2, so the key it derives and the signature below are
// published constants rather than values checked against themselves.
using var device = new DeviceIdentity(Convert.FromHexString(
    "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"));
byte[] message = [0x72];
byte[] published = Convert.FromHexString(
    "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da"
    + "085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00");
Expect(
    device.Sign(message).SequenceEqual(published),
    "the signature is the one the vector publishes");

// Only the 32-byte public key travels to the gateway.
byte[] gatewayKey = device.PublicKey;
Expect(
    DeviceIdentity.FingerprintOf(gatewayKey) == "3d4017c3e843895a",
    "the fingerprint labels the key the vector fixes");

// Signing is deterministic, so the same reading always yields the same 64 bytes;
// there is no randomness to get wrong on a microcontroller.
const string reading = "meter-4 1182.750 kWh";
byte[] signature = device.Sign(reading);
Expect(device.Sign(reading).SequenceEqual(signature), "signing is deterministic");
Expect(DeviceIdentity.Verify(gatewayKey, reading, signature), "the reading is authentic");

// A digit changed in transit fails, and so does a signature offered under another
// device's key.
Expect(
    !DeviceIdentity.Verify(gatewayKey, "meter-4 1082.750 kWh", signature),
    "an altered reading does not verify");
byte[] impostorSeed = new byte[DeviceIdentity.KeyLength];
Array.Fill(impostorSeed, (byte)0x5A);
using var impostor = new DeviceIdentity(impostorSeed);
Expect(
    !DeviceIdentity.Verify(impostor.PublicKey, reading, signature),
    "another device's key does not verify it either");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-security`](https://crates.io/crates/pamoja-security) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html), [docs.rs](https://docs.rs/pamoja-security) |
| TypeScript | [`@pamoja/security`](https://www.npmjs.com/package/@pamoja/security) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_security.html) |
| Python | [`pamoja-security`](https://pypi.org/project/pamoja-security/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/security.html) |
| C# | [`Pamoja.Security`](https://www.nuget.org/packages/Pamoja.Security) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.html) |

## Documentation

- [`Pamoja.Security` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Security.html), every type in this namespace.
- [The Device identity guide](https://pamoja.molex.cloud/docs/guides/security.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
