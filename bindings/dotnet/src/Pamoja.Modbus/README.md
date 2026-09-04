# Pamoja.Modbus

Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
// Ask unit 0x11 for three holding registers starting at 0x006B. The last two bytes
// are the CRC-16/MODBUS, so this is the frame exactly as it goes out on the wire.
byte[] request = Modbus.ReadHoldingRegisters(0x11, 0x006B, 3);
Expect(
    request.SequenceEqual(new byte[] { 0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87 }),
    "the request is the frame the specification fixes");

// The device answers with three 16-bit registers. A reply carries its own checksum,
// so the receiver validates the frame before reading any value out of it.
byte[] body = [0x11, 0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64];
ushort checksum = Modbus.Crc16(body);
byte[] wire = [.. body, (byte)(checksum & 0xFF), (byte)(checksum >> 8)];
using ModbusFrame reply = Modbus.ParseFrame(wire);
Expect(reply.Address == 0x11, "the reply comes from the unit that was asked");
Expect(reply.Exception is null, "a served request reports no exception");
Expect(
    reply.Registers().SequenceEqual(new ushort[] { 0x022B, 0x0000, 0x0064 }),
    "the three registers read back");

// One flipped bit anywhere in the frame fails the checksum, which is the whole
// point of carrying one over a long RS485 run.
byte[] corrupt = [.. wire];
corrupt[2] ^= 0xFF;
bool rejected = false;
try
{
    using ModbusFrame _ = Modbus.ParseFrame(corrupt);
}
catch (PamojaException)
{
    rejected = true;
}
Expect(rejected, "a frame mangled on the wire is rejected");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-modbus`](https://crates.io/crates/pamoja-modbus) | [docs.rs](https://docs.rs/pamoja-modbus), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html) |
| TypeScript | [`@pamoja/modbus`](https://www.npmjs.com/package/@pamoja/modbus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html) |
| Python | [`pamoja-modbus`](https://pypi.org/project/pamoja-modbus/) | [`pamoja.modbus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html) |
| C# | [`Pamoja.Modbus`](https://www.nuget.org/packages/Pamoja.Modbus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.Modbus.html) |

## Documentation

- [The Modbus RTU guide](https://pamoja.molex.cloud/docs/guides/modbus.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
