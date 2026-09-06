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
// The nodes on this bus, by the address each answers to, and the two parameter
// groups in play. J1939 publishes both, so naming them makes the traffic readable.
const byte Engine = 0;
const byte Gateway = 1;
const byte Gearbox = 33;
const uint EngineController1 = 61_444; // carries engine speed
const uint Request = 59_904; // asks another node for a parameter group

// Where engine speed sits inside that group, and the scale the standard fixes for
// it. Naming both is what stops a sender and a receiver disagreeing about either.
const int EngineSpeedAt = 3;
const double RpmPerBit = 0.125;

// J1939 keeps its addressing inside the CAN identifier: a priority, the parameter
// group, and the address of whatever sent it. A broadcast has no destination, so
// it is its own constructor rather than a magic address a caller has to know.
uint speedId = Can.BroadcastJ1939(J1939Priority.Control, EngineController1, Engine);
J1939Message speed = Can.DecodeJ1939(speedId)!;
Console.WriteLine($"broadcast pgn {speed.Pgn} at priority {speed.Priority}");

// A parameter group below the PDU1 limit is addressed rather than broadcast, so
// those eight identifier bits carry a destination instead of extending the group.
uint requestId = Can.ComposeJ1939((byte)J1939Priority.Normal, Request, Gateway, Gearbox);
Console.WriteLine($"request   pgn {Request} addressed to node {Gearbox}");

// Reading one back off the bus is the same thing in reverse, so a receiver never
// unpacks 29 bits by hand.
J1939Message heard = Can.DecodeJ1939(requestId)!;
Console.WriteLine($"heard     from node {heard.Source} for node {heard.Destination}");

// The payload. Every signal starts marked not available, and this controller
// reports only engine speed, so that is the only one it writes.
Signals reported = Signals.New();
reported.SetU16(EngineSpeedAt, (ushort)(1000 / RpmPerBit));
CanFrame eec1 = Can.Frame(speedId, reported.ToArray(), extended: true);

// The receiving node reads the same offset back, so neither end slices the payload.
double rpm = Signals.From(eec1.Data).U16(EngineSpeedAt)!.Value * RpmPerBit;
Console.WriteLine($"engine    {rpm} rpm, carried in {eec1.Dlc} bytes");

// Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
// classic frame still refuses a ninth byte.
CanFrame wide = Can.FdFrame(speedId, new byte[32], extended: true);
Console.WriteLine($"32 bytes carries length code {wide.Dlc}");
try
{
    Can.Frame(speedId, new byte[9], extended: true);
    Console.WriteLine("a classic frame took nine bytes, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"classic   refused nine bytes: {error.Message}");
}

// J1939 never rides an 11-bit identifier, so a standard frame is not one of its
// messages however its bits happen to line up.
Console.WriteLine($"an 11-bit identifier is J1939: {Can.DecodeJ1939(291, false) is not null}");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-can`](https://crates.io/crates/pamoja-can) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html), [docs.rs](https://docs.rs/pamoja-can), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-can) |
| TypeScript | [`@pamoja/can`](https://www.npmjs.com/package/@pamoja/can) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-can) |
| Python | [`pamoja-can`](https://pypi.org/project/pamoja-can/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-can) |
| C# | [`Pamoja.Can`](https://www.nuget.org/packages/Pamoja.Can) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-can) |

## Documentation

- [`Pamoja.Can` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html), every type in this namespace.
- [The CAN and J1939 guide](https://pamoja.molex.cloud/docs/guides/can.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
