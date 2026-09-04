# Pamoja.Session

X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/session.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Session
```

```csharp
using Pamoja.Session;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/SessionGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SessionGuide.cs):

```csharp
// Each device is provisioned with a 32-byte seed and publishes the key it
// derives. These are the X25519 pair RFC 7748 section 6.1 publishes, so the
// derivation is pinned to the specification rather than checked against itself.
using var node = new AgreementKey(Convert.FromHexString(
    "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a"));
using var gateway = new AgreementKey(Convert.FromHexString(
    "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb"));
Expect(
    Convert.ToHexString(node.PublicKey).ToLowerInvariant()
        == "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a",
    "the public key is the one the vector publishes");

// Neither side sends the session key. Both derive it from the shared secret, a
// salt that travels in the clear, and both public keys. The roles are opposite.
// The salt must be fresh for every session: reusing one derives the same key from
// the same pair of devices twice. The initiator draws it and sends it in the clear,
// so the responder here uses the salt it received rather than one of its own.
byte[] salt = RandomNumberGenerator.GetBytes(16);
using var uplink = new Session(node, gateway.PublicKey, salt, SessionRole.Initiator);
using var downlink = new Session(gateway, node.PublicKey, salt, SessionRole.Responder);

// The pump id is authenticated but not encrypted, so a router still reads it
// while any change to it fails the tag.
SealedMessage reading = uplink.Seal("flow=41.2"u8, "pump-3"u8);
Expect(
    !reading.Ciphertext.SequenceEqual("flow=41.2"u8.ToArray()),
    "the reading does not travel in the clear");
Expect(
    downlink.Open(reading, "pump-3"u8).SequenceEqual("flow=41.2"u8.ToArray()),
    "the gateway recovers the reading");

// The anti-replay window refuses a counter it has already accepted, so a frame
// captured off the air and sent again is not delivered a second time.
bool refused = false;
try
{
    downlink.Open(reading, "pump-3"u8);
}
catch (PamojaException)
{
    refused = true;
}
Expect(refused, "the same message is refused a second time");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-session`](https://crates.io/crates/pamoja-session) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html), [docs.rs](https://docs.rs/pamoja-session) |
| TypeScript | [`@pamoja/session`](https://www.npmjs.com/package/@pamoja/session) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html) |
| Python | [`pamoja-session`](https://pypi.org/project/pamoja-session/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html) |
| C# | [`Pamoja.Session`](https://www.nuget.org/packages/Pamoja.Session) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html) |

## Documentation

- [`Pamoja.Session` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html), every type in this namespace.
- [The Secured session guide](https://pamoja.molex.cloud/docs/guides/session.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
