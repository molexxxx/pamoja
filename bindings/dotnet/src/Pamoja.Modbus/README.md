# Pamoja.Modbus

Modbus RTU for RS485 field devices: a client that polls them over a serial port with the line's timing, simulated devices that answer as real ones do, and the frames with their CRC-16/MODBUS. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/modbus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html)

## Install

```sh
dotnet add package Pamoja.Modbus
```

```csharp
using Pamoja.Modbus;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Codec` and `Pamoja.Hal`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs):

```csharp
static string Word(bool on) => on ? "on" : "off";

// The line: 19200 baud, even parity, one stop bit, the default the Modbus
// specification sets. Eleven bits a character, and 3.5 of them of silence mark where
// a frame ends.
var settings = new SerialSettings(19_200, Parity.Even);
ulong gap = ModbusClient.FrameGapNanos(settings) / 1_000;
Console.WriteLine($"line         {settings}, {settings.BitsPerCharacter} bits a character, t3.5 is {gap} us");

// Each device's manual gives its unit address and where its values live. The meter
// keeps its measurements in input registers from 0: volts in tenths, amps in
// hundredths, then a fault word. The relay module has four relays as coils 0 to 3, and
// the tank's low-level float switch as discrete input 0, on while the water is below it.
const byte Meter = 17;
const byte Pump = 18;
using var meter = new ModbusServer(Meter);
meter.SetInputRegisters(0, [2301, 418, 0]);
using var relays = new ModbusServer(Pump);
relays.SetCoils(0, new bool[4]);
relays.SetDiscreteInputs(0, [true]);
using var line = new ModbusLine().Attach(meter).Attach(relays);

// The devices sit on a simulated line. On a gateway the port is
// SerialPort.Open("/dev/ttyUSB0", settings), and nothing after this statement changes.
using SerialPort port = line.Port(settings);
using var client = new ModbusClient(port);

// Poll the meter with function 0x04 for three input registers, and scale each one as
// its manual says.
ushort[] registers = client.ReadInputRegisters(Meter, 0, 3);
Console.WriteLine(Invariant(
    $"meter        {registers[0] / 10.0:F1} V, {registers[1] / 100.0:F2} A, faults {registers[2]}"));

// What that poll cost the line: the request, the reply, and the silence before the
// request.
long bytesOut = port.Written;
long bytesBack = port.Received;
ulong lineTime = settings.TransferMicros((int)bytesOut) + settings.TransferMicros((int)bytesBack) + gap;
Console.WriteLine(Invariant(
    $"poll         {bytesOut} bytes out, {bytesBack} back, {lineTime / 1_000.0:F2} ms of line time"));

// Read the float switch, and start the pump on relay 0 when the tank is low.
bool low = client.ReadDiscreteInputs(Pump, 0, 1)[0];
Console.WriteLine($"tank         low-level switch {Word(low)}");
if (low)
{
    client.WriteSingleCoil(Pump, 0, true);
}

bool[] states = client.ReadCoils(Pump, 0, 4);
Console.WriteLine($"relays       {string.Join(' ', states.Select(Word))}");

// A broadcast, to unit 0, reaches every device on the line and none answers: here
// every relay off at once. The client waits out the turnaround so each device has
// carried it out before the next request.
client.WriteMultipleCoils(ModbusClient.Broadcast, 0, new bool[4]);
Console.WriteLine($"broadcast    every relay off, no reply, {client.Turnaround.TotalMilliseconds} ms turnaround");
bool[] after = client.ReadCoils(Pump, 0, 4);
Console.WriteLine($"relays       {string.Join(' ', after.Select(Word))}");

// The meter keeps its measurements in input registers. Asking for them as holding
// registers, function 0x03, is the usual mistake with a new device, and the meter
// refuses it with an exception instead of answering.
ModbusClientException? refused = null;
try
{
    client.ReadHoldingRegisters(Meter, 0, 3);
}
catch (ModbusClientException error)
{
    refused = error;
    Console.WriteLine($"refused      {error.Message}");
}

// A unit that is not on the line never answers. The client gives up after its response
// timeout, one second unless told otherwise, which a simulated line counts instead of
// sleeping through.
ulong before = port.WaitedMicros;
ModbusClientException? silent = null;
try
{
    client.ReadInputRegisters(19, 0, 1);
}
catch (ModbusClientException error)
{
    silent = error;
    Console.WriteLine($"silent       {error.Message}, {(port.WaitedMicros - before) / 1_000} ms counted and not slept");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-modbus`](https://crates.io/crates/pamoja-modbus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html), [docs.rs](https://docs.rs/pamoja-modbus), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-modbus) |
| TypeScript | [`@pamoja/modbus`](https://www.npmjs.com/package/@pamoja/modbus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-modbus) |
| Python | [`pamoja-modbus`](https://pypi.org/project/pamoja-modbus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-modbus) |
| C# | [`Pamoja.Modbus`](https://www.nuget.org/packages/Pamoja.Modbus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-modbus) |

## Documentation

- [`Pamoja.Modbus` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html), every type in this namespace.
- [The Modbus RTU guide](https://pamoja.molex.cloud/docs/guides/modbus.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
