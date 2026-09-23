# Pamoja.Sensors

Datasheet-anchored drivers for eleven parts, from every language: the BME280, BMP280, DS18B20, HDC1080, INA219, INA226, ADS1115, OPT3001, SCD4x, SHT3x, and TMP117. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sensors.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html)

## Install

```sh
dotnet add package Pamoja.Sensors
```

```csharp
using Pamoja.Sensors;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Hal`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs):

```csharp
// The address plan. Every part answers at the address its pins choose, and two parts on
// one address garble each other, so a bench of nine is planned around the two that
// cannot move: the HDC1080 and the SCD41 have one address each.
const byte Air = Sht3x.AddressA;
const byte Pressure = Bmp280.AddressSecondary;
const byte Light = Opt3001.AddressScl;
const byte Soil = Ads1115.AddressSda;
const byte Enclosure = Tmp117.AddressAdd0Vplus;
byte panel = Ina219.Address(Ina219.AddressPin.Ground, Ina219.AddressPin.Supply);
byte battery = Ina226.Address(Ina226.AddressPin.Supply, Ina226.AddressPin.Supply);

// The bench with nothing plugged in: each part answers the way its datasheet says, with
// the reading it is given here, a warm and humid afternoon. The bus keeps a copy of each
// part, so the program lets go of its own. On a Raspberry Pi the bus is
// I2cBus.Open("/dev/i2c-1") and nothing after this statement changes.
SimulatedPart[] parts =
[
    Sht3x.Sim.Reporting(Air, 24.1f, 62.0f),
    Bmp280.Sim.Reporting(Pressure, 24.1f, 1003.2f),
    Scd4x.Sim.Reporting(1_180, 24.1f, 62.0f),
    Opt3001.Sim.Reporting(Light, 4_200f),
    Ads1115.Sim.Reporting(Soil, Ads1115.Pga.Fsr4_096, 2.35f),
    Tmp117.Sim.Reporting(Enclosure, 31.25f),
    Hdc1080.Sim.Reporting(31.25f, 38.0f),
    Ina219.Sim.Reporting(panel, 100, 3_200_000, 18_400, 1_250_000),
    Ina226.Sim.Reporting(battery, 2, 20_000_000, 12_800_000, -350_000),
];
using I2cBus bus = I2cBus.Simulated(parts);
foreach (SimulatedPart part in parts)
{
    part.Dispose();
}

// One driver per part. Each holds its own share of the bus, and each runs its
// datasheet's whole conversation on the first measurement: reset, identify, configure,
// convert, read.
using var airSensor = new Sht3x(bus, Air);
var airNow = airSensor.Measure();
Console.WriteLine(Invariant($"air          {airNow.Celsius:F2} C, {airNow.RelativeHumidity:F2} %"));

using var barometer = new Bmp280(bus, Pressure);
Bmp280Reading weather = barometer.Measure();
Console.WriteLine(Invariant($"pressure     {weather.Hectopascals:F1} hPa"));

using var co2Sensor = new Scd4x(bus);
var co2 = co2Sensor.Measure();
Console.WriteLine($"co2          {co2.Co2Ppm} ppm");

using var lightSensor = new Opt3001(bus, Light);
Opt3001Reading sun = lightSensor.Measure();
Console.WriteLine(Invariant($"light        {sun.Lux:F0} lux"));

// A capacitive probe's voltage falls as the soil wets and nears 3 V in dry soil, past the
// ADS1115's default range of 2.048 V, so the driver is given the 4.096 V range.
using var converter = new Ads1115(bus, Soil, Ads1115.Mux.Ain0Gnd, Ads1115.Pga.Fsr4_096);
Ads1115Sample probe = converter.Sample();
Console.WriteLine(Invariant($"soil         {probe.Volts:F3} V"));

using var boxThermometer = new Tmp117(bus, Enclosure);
using var boxHygrometer = new Hdc1080(bus);
Tmp117Reading box = boxThermometer.Measure();
var boxAir = boxHygrometer.Measure();
Console.WriteLine(Invariant($"enclosure    {box.Celsius:F2} C, {boxAir.RelativeHumidity:F1} %"));

// Two current monitors, each calibrated for its own shunt. The fans and the pump draw
// more than the panel gives, so the battery makes up the rest and its current reads
// negative: current through a shunt is signed by its direction.
using var panelMonitor = new Ina219(bus, panel, shuntMilliohms: 100, maxMicroamps: 3_200_000);
Ina219Reading charge = panelMonitor.Measure();
Console.WriteLine(Invariant(
    $"panel        {charge.BusMillivolts / 1e3:F2} V, {charge.CurrentMicroamps / 1e6:F2} A, {charge.PowerMicrowatts / 1e6:F2} W"));
using var batteryMonitor = new Ina226(bus, battery, shuntMilliohms: 2, maxMicroamps: 20_000_000);
Ina226Reading drain = batteryMonitor.Measure();
Console.WriteLine(Invariant(
    $"battery      {drain.BusVolts:F2} V, {drain.CurrentAmps:F2} A, {drain.PowerWatts:F2} W"));

// Every driver waited as its datasheet asks, the OPT3001's 800 ms integration and the
// SCD41's command times most of all. The simulated bus counted the waits and slept none.
Console.WriteLine(Invariant($"waited       {bus.WaitedMicros / 1e6:F2} s across {bus.Transfers} transfers"));

// The same probe read at the default range. Past 2.048 V the converter pins at its top
// code, and the sample says so rather than passing the edge of the range off as a reading.
using (WordPart reprobed = Ads1115.Sim.Reporting(Soil, Ads1115.Pga.Fsr2_048, 2.35f))
{
    bus.Attach(reprobed);
}

using var defaultGain = new Ads1115(bus, Soil, Ads1115.Mux.Ain0Gnd);
Ads1115Sample pinned = defaultGain.Sample();
Console.WriteLine(Invariant(
    $"default gain {pinned.Volts:F3} V, clipped: {pinned.Clipped.ToString().ToLowerInvariant()}"));

// A driver aimed at the wrong address meets whatever answers there. The TMP117 reads its
// device id before anything else, so pointed at the soil probe's converter it refuses
// rather than reporting that part's registers as a temperature.
try
{
    using var misplaced = new Tmp117(bus, Soil);
    misplaced.Init();
    Console.WriteLine("wrong part   accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"wrong part   {error.Message}");
}

// A DS18B20 on a lead into a pot, read the way a Linux board reads one: the kernel's
// 1-Wire driver serves each probe as a file under /sys/bus/w1/devices. The program writes
// that file itself, with the text the kernel prints, so it runs anywhere.
string devices = Path.Combine(Path.GetTempPath(), $"pamoja-bench-{Environment.ProcessId}");
string directory = Path.Combine(devices, $"{Ds18b20.FamilyCode:x2}-000005e2fdc3");
Directory.CreateDirectory(directory);
byte[] scratchpad = Ds18b20.BuildScratchpad(19.5f, 12, 30, 5);
File.WriteAllText(Path.Combine(directory, "w1_slave"), Ds18b20.W1SlaveText(scratchpad));
var found = new List<Ds18b20Reading>();
foreach (Ds18b20Thermometer thermometer in Ds18b20Thermometer.Discover(devices))
{
    using (thermometer)
    {
        Ds18b20Reading reading = thermometer.Read();
        Console.WriteLine(Invariant(
            $"soil probe   {reading.Celsius:F4} C, alarms at {reading.AlarmLow} and {reading.AlarmHigh} C"));
        found.Add(reading);
    }
}

Directory.Delete(devices, recursive: true);
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
