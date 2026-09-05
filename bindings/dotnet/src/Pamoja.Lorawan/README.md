# Pamoja.Lorawan

LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/lorawan.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
// The root key is provisioned into the device at the factory and known to the
// network server. It is the only secret either side starts with; any 16 bytes
// stand in here.
byte[] appKey = new byte[16];
Array.Fill(appKey, (byte)7);

// The device asks to join with a nonce it has not used before, which is what stops
// an old accept being replayed at it.
const ushort DevNonce = 1;
using var node = new LorawanDevice(new byte[8], new byte[8], appKey);

// The network grants the join. It draws its own nonce, names the network the
// device is joining, and assigns the address it will answer to from then on.
const uint DevAddr = 0x26012E43;
var offer = new LorawanGrant(appNonce: 2, netId: 19, devAddr: DevAddr);
byte[] accept = offer.Accept(appKey, DevNonce);
Console.WriteLine($"granted   address 0x{DevAddr:X8} in a {accept.Length}-byte accept");

// The device verifies it against the root key. A join accept carries no device
// identifier, so only that key decides whether it is for this device.
using LorawanJoinAccept joined = node.AcceptJoin(accept, DevNonce);
Console.WriteLine($"joined    the device took address 0x{joined.DevAddr:X8}");

// Neither side transmits a session key. Both derive the same pair from the root
// key and the two nonces, so the network reads what the device sends without ever
// having been told how.
using LorawanSession network = offer.Session(appKey, DevNonce);
using LorawanSession activated = joined.Session();
byte[] uplink = activated.EncodeUplink(1, 1, "level=high"u8);
LorawanRxData received = network.Decode(uplink, 1);
Console.WriteLine(
    $"uplink    the network read {System.Text.Encoding.UTF8.GetString(received.Payload)}");

// A single byte changed in the air fails that check, so no one else can admit the
// device or put words in its mouth.
byte[] forged = [.. accept];
forged[1] ^= 0xFF;
try
{
    node.AcceptJoin(forged, DevNonce).Dispose();
    Console.WriteLine("a forged accept was taken, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"forged    accept refused: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-lorawan`](https://crates.io/crates/pamoja-lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html), [docs.rs](https://docs.rs/pamoja-lorawan) |
| TypeScript | [`@pamoja/lorawan`](https://www.npmjs.com/package/@pamoja/lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lorawan.html) |
| Python | [`pamoja-lorawan`](https://pypi.org/project/pamoja-lorawan/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/lorawan.html) |
| C# | [`Pamoja.Lorawan`](https://www.nuget.org/packages/Pamoja.Lorawan) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html) |

## Documentation

- [`Pamoja.Lorawan` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lorawan.html), every type in this namespace.
- [The LoRaWAN guide](https://pamoja.molex.cloud/docs/guides/lorawan.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
