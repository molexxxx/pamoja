# Pamoja.Sensors

Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sensors.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Sensors
```

```csharp
using Pamoja.Sensors;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs):

```csharp
// Stand-ins for the two parts. On a running node the thermometer's nine bytes
// come off the 1-Wire bus and the monitor's registers off I2C; here the library
// builds what each would send, so the program runs with nothing plugged in.
byte[] thermometer = Ds18b20.BuildScratchpad(25.0625f, 12, 75, -10);

// The monitor is set up for 1 mA per count across a 2 milliohm shunt, the load
// its datasheet's worked design example describes: 11.98 V, 10 A, and 119.8 W.
const uint CurrentLsb = 1_000;
ushort bus = Ina219.BusRegister(11_980);
short current = Ina219.CurrentRegister(10_000_000, CurrentLsb);
ushort power = Ina219.PowerRegister(119_800_000, CurrentLsb);

// Everything below is the node's own code. The thermometer checksums every read,
// so a reading is verified before it is believed.
Ds18b20Reading reading = Ds18b20.ParseScratchpad(thermometer);
Console.WriteLine($"temperature  {reading.Celsius:F4} C");
Console.WriteLine($"resolution   {reading.ResolutionBits} bits");
Console.WriteLine($"alarms       {reading.AlarmHigh} / {reading.AlarmLow} C");

// The monitor computes nothing until it has been told what shunt it is across.
Console.WriteLine($"calibration  0x{Ina219.Calibration(CurrentLsb, 2):X4}");
Console.WriteLine($"bus          {Ina219.BusMillivolts(bus)} mV");
Console.WriteLine($"current      {Ina219.CurrentMicroamps(current, CurrentLsb) / 1_000} mA");
Console.WriteLine($"power        {Ina219.PowerMicrowatts(power, CurrentLsb) / 1_000} mW");

// A bit flipped on a long 1-Wire run fails the checksum, so the node repeats the
// read instead of logging a temperature a couple of degrees off.
byte[] corrupted = [.. thermometer];
corrupted[0] ^= 1;
try
{
    Ds18b20.ParseScratchpad(corrupted);
    Console.WriteLine("corrupt read accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"corrupt read rejected: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sensors`](https://crates.io/crates/pamoja-sensors) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html), [docs.rs](https://docs.rs/pamoja-sensors), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sensors) |
| TypeScript | [`@pamoja/sensors`](https://www.npmjs.com/package/@pamoja/sensors) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sensors) |
| Python | [`pamoja-sensors`](https://pypi.org/project/pamoja-sensors/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sensors) |
| C# | [`Pamoja.Sensors`](https://www.nuget.org/packages/Pamoja.Sensors) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sensors) |

## Documentation

- [`Pamoja.Sensors` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html), every type in this namespace.
- [The Sensor drivers guide](https://pamoja.molex.cloud/docs/guides/sensors.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
