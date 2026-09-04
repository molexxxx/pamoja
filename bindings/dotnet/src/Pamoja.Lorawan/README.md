# Pamoja.Lorawan

LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
dotnet add package Pamoja.Lorawan
```

```csharp
using Pamoja.Lorawan;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Codec`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs):

```csharp
// A join accept captured off a live EU868 network, the root key it was signed
// under, and the session keys an independent implementation derived from it.
// Published at https://github.com/anthonykirby/lora-packet/issues/10
byte[] captured = Convert.FromHexString(
    "204dd85ae608b87fc4889970b7d2042c9e72959b0057aed6094b16003df12de145");
byte[] appKey = Convert.FromHexString("b6b53f4a168a7a88bdf7ea135ce9cfca");
const ushort devNonce = 0xCC85;

// The network half: the address and radio settings this network grants, encrypted
// and signed under the root key, are the frame that was captured.
var offer = new LorawanGrant(
    appNonce: 0x00E5063A,
    netId: 0x13,
    devAddr: 0x26012E43,
    dlSettings: 0x03,
    rxDelay: 0x01,
    cflist: Convert.FromHexString("184f84e85684b85e84886684586e8400"));
Expect(
    offer.Accept(appKey, devNonce).SequenceEqual(captured),
    "the join accept this network signs is the frame that was captured");

// The device half. A join accept carries no EUI, so only the root key decides
// whether it verifies.
using var node = new LorawanDevice(new byte[8], new byte[8], appKey);
using LorawanJoinAccept accepted = node.AcceptJoin(captured, devNonce);
Expect(accepted.DevAddr == 0x26012E43, "the device takes the address it was granted");

// Neither side transmits a session key; both derive it from the two nonces. What
// the device derived is read back by a session holding the published keys.
byte[] keys = Convert.FromHexString(
    "2c96f7028184bb0be8aa49275290d4fcf3a5c8f0232a38c144029c165865802c");
using var gateway = new LorawanSession(0x26012E43, keys.AsSpan(0, 16), keys.AsSpan(16));
using LorawanSession activated = accepted.Session();
byte[] uplink = activated.EncodeUplink(1, 1, "real"u8);
Expect(
    gateway.Decode(uplink, 1).Payload.AsSpan().SequenceEqual("real"u8),
    "the network reads what the device it just admitted wrote");

// A single byte changed in the air fails the MIC, so no one else can admit the
// device.
byte[] forged = [.. captured];
forged[1] ^= 0xFF;
bool refused = false;
try
{
    using LorawanJoinAccept _ = node.AcceptJoin(forged, devNonce);
}
catch (PamojaException)
{
    refused = true;
}
Expect(refused, "a join accept nobody signed does not activate a session");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-lorawan`](https://crates.io/crates/pamoja-lorawan) | [docs.rs](https://docs.rs/pamoja-lorawan), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html) |
| TypeScript | [`@pamoja/lorawan`](https://www.npmjs.com/package/@pamoja/lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html) |
| Python | [`pamoja-lorawan`](https://pypi.org/project/pamoja-lorawan/) | [`pamoja.lorawan`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html) |
| C# | [`Pamoja.Lorawan`](https://www.nuget.org/packages/Pamoja.Lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.Lorawan.html) |

## Documentation

- [The LoRaWAN guide](https://pamoja.molex.cloud/docs/guides/lorawan.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
