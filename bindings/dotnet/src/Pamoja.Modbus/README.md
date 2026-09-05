# Pamoja.Modbus

Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/modbus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Modbus
```

```csharp
using Pamoja.Modbus;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Codec`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs):

```csharp
// The device this gateway polls: a power meter at unit 17, whose manual says the
// three registers holding voltage, current and a fault word start at address 107.
const byte Meter = 17;
const ushort FirstRegister = 107;

// Ask it for those three registers. The frame is complete, checksum included,
// exactly as it goes out on the wire.
byte[] request = Modbus.ReadHoldingRegisters(Meter, FirstRegister, 3);
Console.WriteLine($"polling unit {Meter}, {request.Length} bytes out");

// A stand-in for the meter. On a running gateway this frame arrives over RS485;
// here the library builds what a meter reporting those values would send back.
byte[] fromTheMeter = Modbus.ReadHoldingRegistersReply(Meter, [2301, 418, 0]);

// Everything below is the gateway's own code. A reply carries its own checksum,
// so the frame is validated before any value is read out of it.
ModbusFrame reply = Modbus.ParseFrame(fromTheMeter);
ushort[] registers = reply.Registers();
Console.WriteLine($"voltage   {registers[0] / 10.0:F1} V");
Console.WriteLine($"current   {registers[1] / 100.0:F2} A");
Console.WriteLine($"faults    {registers[2]}");

// One flipped bit anywhere in the frame fails the checksum, which is the whole
// point of carrying one over a long RS485 run.
byte[] mangled = [.. fromTheMeter];
mangled[2] ^= 0xFF;
try
{
    Modbus.ParseFrame(mangled);
    Console.WriteLine("mangled frame accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"mangled frame rejected: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-modbus`](https://crates.io/crates/pamoja-modbus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html), [docs.rs](https://docs.rs/pamoja-modbus) |
| TypeScript | [`@pamoja/modbus`](https://www.npmjs.com/package/@pamoja/modbus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html) |
| Python | [`pamoja-modbus`](https://pypi.org/project/pamoja-modbus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html) |
| C# | [`Pamoja.Modbus`](https://www.nuget.org/packages/Pamoja.Modbus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html) |

## Documentation

- [`Pamoja.Modbus` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html), every type in this namespace.
- [The Modbus RTU guide](https://pamoja.molex.cloud/docs/guides/modbus.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
