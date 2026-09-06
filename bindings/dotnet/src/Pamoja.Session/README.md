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
// Each device is provisioned with a 32-byte seed and publishes the key it derives.
// A real seed comes from the factory or a secure element; any 32 bytes stand in.
byte[] nodeSeed = new byte[32];
Array.Fill(nodeSeed, (byte)7);
byte[] gatewaySeed = new byte[32];
Array.Fill(gatewaySeed, (byte)9);
using var node = new AgreementKey(nodeSeed);
using var gateway = new AgreementKey(gatewaySeed);

// Neither side sends the session key. Both derive it from the shared secret, a
// salt that travels in the clear, and both public keys, with opposite roles.
//
// The salt must be fresh for every session: reusing one derives the same key from
// the same pair of devices twice. The initiator draws it and sends it in the
// clear, so the responder uses the salt it received rather than one of its own.
byte[] salt = RandomNumberGenerator.GetBytes(16);
using var uplink = new Session(node, gateway.PublicKey, salt, SessionRole.Initiator);
using var downlink = new Session(gateway, node.PublicKey, salt, SessionRole.Responder);
Console.WriteLine("both sides derived a key without sending one");

// The pump id is authenticated but not encrypted, so a router still reads it while
// any change to it fails the tag.
SealedMessage reading = uplink.Seal("flow=41.2"u8, "pump-3"u8);
bool hidden = !reading.Ciphertext.SequenceEqual("flow=41.2"u8.ToArray());
Console.WriteLine($"sealed    the reading is no longer readable: {hidden}");
byte[] opened = downlink.Open(reading, "pump-3"u8);
Console.WriteLine($"opened    {System.Text.Encoding.UTF8.GetString(opened)}");

// The anti-replay window refuses a counter it has already accepted, so a frame
// captured off the air and sent again is not delivered a second time.
try
{
    downlink.Open(reading, "pump-3"u8);
    Console.WriteLine("a replayed frame was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"replay    refused: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-session`](https://crates.io/crates/pamoja-session) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_session/index.html), [docs.rs](https://docs.rs/pamoja-session), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-session) |
| TypeScript | [`@pamoja/session`](https://www.npmjs.com/package/@pamoja/session) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_session.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-session) |
| Python | [`pamoja-session`](https://pypi.org/project/pamoja-session/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/session.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-session) |
| C# | [`Pamoja.Session`](https://www.nuget.org/packages/Pamoja.Session) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-session) |

## Documentation

- [`Pamoja.Session` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Session.html), every type in this namespace.
- [The Secured session guide](https://pamoja.molex.cloud/docs/guides/session.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
