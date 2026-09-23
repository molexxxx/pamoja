# Pamoja.Hal

The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, one I2C bus a program and its drivers share, and delays that sleep or only count. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/hal.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Hal.html)

## Install

```sh
dotnet add package Pamoja.Hal
```

```csharp
using Pamoja.Hal;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs):

```csharp
const byte Part = Bme280.AddressPrimary;
static string Fixed(float value) => value.ToString("F2", CultureInfo.InvariantCulture);
static string Shown(Bme280Measurement reading) =>
    $"{Fixed(reading.Celsius)} C, {Fixed(reading.Hectopascals)} hPa, {Fixed(reading.RelativeHumidityPercent)} %";
static string X(Bme280.Oversampling oversampling) => $"x{Bme280.OversamplingFactor(oversampling)}";

// A bus with one part on it: a BME280 that is not there. It holds a real part's
// calibration and one measurement that part took, and it answers from its registers,
// so the driver runs its whole datasheet sequence against it. On a Raspberry Pi the
// bus is I2cBus.Open("/dev/i2c-1") and nothing after this line changes.
using I2cPart simulated = Bme280.Sim.Part(Part);
using I2cBus bus = I2cBus.Simulated(simulated);
using var sensor = new Bme280(bus, Part);

// Reset, identify, calibrate, configure. The datasheet wants ctrl_hum written before
// ctrl_meas, and the part left asleep until a measurement is forced. The part keeps
// what the driver wrote, so the configuration reads back off the bus.
sensor.Init();
using I2cPart part = bus.Part<I2cPart>(Part)!;
Bme280.Oversampling humidity = Bme280.CtrlHumFromBits(part.Register(Bme280.Register.CtrlHum));
Bme280CtrlMeas ctrl = Bme280.CtrlMeasFromBits(part.Register(Bme280.Register.CtrlMeas));
bool asleep = ctrl.Mode == Bme280.Mode.Sleep;
Console.WriteLine(
    $"configured   humidity {X(humidity)}, temperature {X(ctrl.Temperature)}, " +
    $"pressure {X(ctrl.Pressure)}, asleep: {asleep.ToString().ToLowerInvariant()}");

// One forced measurement. The driver waits the datasheet's longest measurement time
// for these settings before it reads, and a simulated bus counts that wait rather
// than sleeping through it.
Bme280Measurement reading = sensor.Measure();
Console.WriteLine($"measured     {Shown(reading)}");
ulong waited = bus.WaitedMicros;
string waitedMs = (waited / 1000.0).ToString("F2", CultureInfo.InvariantCulture);
Console.WriteLine($"waited       {waitedMs} ms across {bus.Transfers} transfers");

// A part reports whatever it is asked to. Putting one in the first one's place is how
// a program meets a reading it would otherwise wait on the weather for, here a cold
// store at four degrees, and the driver carries on without noticing.
using (I2cPart chilled = Bme280.Sim.Reporting(Part, 4.0f, 1013.25f, 80.0f))
{
    bus.Attach(chilled);
}

Bme280Measurement cold = sensor.Measure();
Console.WriteLine($"cold store   {Shown(cold)}");

// Nothing answers at the part's other address, and the driver says so rather than
// returning a reading.
try
{
    using var absent = new Bme280(bus, Bme280.AddressSecondary);
    absent.Init();
    Console.WriteLine("absent       a part answered");
}
catch (PamojaException error)
{
    Console.WriteLine($"absent       {error.Message}");
}

// The other half of the bus layer. A script plays one conversation and refuses
// anything else, which proves a driver follows the datasheet rather than merely
// working: the reset, the status once the calibration has loaded, the chip id, the
// two calibration blocks, the three configuration writes in the order the part
// requires, then one forced measurement.
const Bme280.Oversampling X1 = Bme280.Oversampling.X1;
var settings = new Bme280CtrlMeas(X1, X1, Bme280.Mode.Sleep);
var forced = settings with { Mode = Bme280.Mode.Forced };
var resetState = new Bme280Config(Bme280.Standby.Ms0_5, Bme280.Filter.Off, Spi3Wire: false);
byte[] idle = [Bme280.Sim.StatusIdle];
using I2cBus script = I2cBus.Scripted(
    I2cStep.Write(Part, [Bme280.Register.Reset, Bme280.ResetWord]),
    I2cStep.WriteRead(Part, [Bme280.Register.Status], idle),
    I2cStep.WriteRead(Part, [Bme280.Register.ChipId], [Bme280.ChipId]),
    I2cStep.WriteRead(Part, [Bme280.Register.CalibTempPress], Bme280.Sim.Calibration()),
    I2cStep.WriteRead(Part, [Bme280.Register.CalibHumidity], Bme280.Sim.CalibrationHumidity()),
    I2cStep.Write(Part, [Bme280.Register.Config, Bme280.ConfigBits(resetState)]),
    I2cStep.Write(Part, [Bme280.Register.CtrlHum, Bme280.CtrlHumBits(X1)]),
    I2cStep.Write(Part, [Bme280.Register.CtrlMeas, Bme280.CtrlMeasBits(settings)]),
    I2cStep.Write(Part, [Bme280.Register.CtrlMeas, Bme280.CtrlMeasBits(forced)]),
    I2cStep.WriteRead(Part, [Bme280.Register.Status], idle),
    I2cStep.WriteRead(Part, [Bme280.Register.Data], Bme280.Sim.Burst()));
using var datasheet = new Bme280(script, Part);
Bme280Measurement checkedReading = datasheet.Measure();
Console.WriteLine(
    $"datasheet    {Fixed(checkedReading.Celsius)} C after {script.Transfers} transfers, " +
    $"{script.Remaining ?? 0} steps left");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-hal`](https://crates.io/crates/pamoja-hal) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_hal/index.html), [docs.rs](https://docs.rs/pamoja-hal), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-hal) |
| TypeScript | [`@pamoja/hal`](https://www.npmjs.com/package/@pamoja/hal) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_hal.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-hal) |
| Python | [`pamoja-hal`](https://pypi.org/project/pamoja-hal/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/hal.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-hal) |
| C# | [`Pamoja.Hal`](https://www.nuget.org/packages/Pamoja.Hal) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Hal.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-hal) |

## Documentation

- [`Pamoja.Hal` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Hal.html), every type in this namespace.
- [The Buses guide](https://pamoja.molex.cloud/docs/guides/hal.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
