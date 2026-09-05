# Pamoja.Can

CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/can.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Can
```

```csharp
using Pamoja.Can;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs):

```csharp
// J1939 keeps its addressing inside the CAN identifier: a priority, a parameter
// group that says what the message is, and the address of whatever sent it.
// Building one from those fields saves a caller packing 29 bits by hand.
const byte Engine = 0x00;
const uint Eec1 = 61_444; // electronic engine controller 1, which carries speed
uint broadcast = Can.ComposeJ1939(3, Eec1, Engine);
J1939Message engine = Can.DecodeJ1939(broadcast)!;
Console.WriteLine($"broadcast priority {engine.Priority} pgn {engine.Pgn}");
Console.WriteLine($"addressed to one node: {!engine.Broadcast}");

// A parameter group below the PDU1 limit is addressed rather than broadcast, so
// those eight identifier bits carry a destination instead of extending the group.
const uint Request = 59_904;
const byte Gateway = 0x01;
const byte Transmission = 0x21;
J1939Message request = Can.DecodeJ1939(
    Can.ComposeJ1939(6, Request, Gateway, Transmission))!;
Console.WriteLine($"request   pgn {request.Pgn} to node 0x{request.Destination:X2}");
Console.WriteLine($"heard     from 0x{request.Source:X2}");

// J1939 never rides an 11-bit identifier, so a standard frame is not one.
Console.WriteLine(
    $"an 11-bit identifier is J1939: {Can.DecodeJ1939(0x123, extended: false) is not null}");

// The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of
// that parameter group at 0.125 rpm per bit, and every signal this controller is
// not reporting is filled with the not-available byte the standard reserves.
byte[] payload = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
BitConverter.TryWriteBytes(payload.AsSpan(3), (ushort)(1000 / 0.125));
CanFrame eec1 = Can.Frame(broadcast, payload, extended: true);
double speed = BitConverter.ToUInt16(eec1.Data, 3) * 0.125;
Console.WriteLine($"engine    {speed} rpm in {eec1.Dlc} bytes");

// Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
// classic frame still refuses a ninth byte.
Console.WriteLine(
    $"32 bytes carries length code {Can.FdFrame(broadcast, new byte[32], true).Dlc}");
try
{
    Can.Frame(broadcast, new byte[9], extended: true);
    Console.WriteLine("a classic frame took nine bytes, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"classic   refused nine bytes: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-can`](https://crates.io/crates/pamoja-can) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html), [docs.rs](https://docs.rs/pamoja-can) |
| TypeScript | [`@pamoja/can`](https://www.npmjs.com/package/@pamoja/can) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html) |
| Python | [`pamoja-can`](https://pypi.org/project/pamoja-can/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html) |
| C# | [`Pamoja.Can`](https://www.nuget.org/packages/Pamoja.Can) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html) |

## Documentation

- [`Pamoja.Can` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html), every type in this namespace.
- [The CAN and J1939 guide](https://pamoja.molex.cloud/docs/guides/can.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
