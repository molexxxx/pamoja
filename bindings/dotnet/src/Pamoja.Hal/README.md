# Pamoja.Hal

The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, and one I2C bus a program and its drivers share. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
var bus = new ScriptedDevice(
[
    new Step(0xE0, Write: 0xB6),
    new Step(0xF3, Reply: [0x00]),
    new Step(0xD0, Reply: [Bme280.ChipId]),
    new Step(0x88, Reply: CalibrationA),
    new Step(0xE1, Reply: CalibrationB),
    new Step(0xF5, Write: 0x00),
    new Step(0xF2, Write: 0x01),
    new Step(0xF4, Write: 0x24),
    new Step(0xF4, Write: 0x25),
    new Step(0xF3, Reply: [0x00]),
    new Step(0xF7, Reply: Burst),
]);

// The datasheet's start-up: the soft reset word, its 2 ms start-up time, and
// the status register, whose low bit clears once the calibration image loaded.
bus.Write([0xE0, 0xB6]);
await Task.Delay(2);
byte[] status = new byte[1];
bus.WriteRead([0xF3], status);

// The chip id says it is a BME280, and the two calibration blocks are read once.
byte[] id = new byte[1];
bus.WriteRead([0xD0], id);
byte[] tempPress = new byte[26];
byte[] humidity = new byte[7];
bus.WriteRead([0x88], tempPress);
bus.WriteRead([0xE1], humidity);
using var calibration = new Bme280Calibration(tempPress, humidity);
Console.WriteLine($"calibration  read once: {(id[0] == Bme280.ChipId).ToString().ToLowerInvariant()}");

// config, ctrl_hum, then ctrl_meas, in that order because ctrl_hum only takes
// effect after the ctrl_meas write: every measurement at oversampling x1, asleep.
bus.Write([0xF5, 0x00]);
bus.Write([0xF2, 0x01]);
bus.Write([0xF4, 0x24]);

// One forced measurement: the mode bits, the datasheet's 9.3 ms maximum for
// these settings, the status read that confirms the part is idle, the burst.
bus.Write([0xF4, 0x25]);
await Task.Delay(10);
bus.WriteRead([0xF3], status);
byte[] burst = new byte[8];
bus.WriteRead([0xF7], burst);
Bme280Measurement measurement = calibration.Compensate(burst);
string celsius = measurement.Celsius.ToString("F2", CultureInfo.InvariantCulture);
string hectopascals = measurement.Hectopascals.ToString("F2", CultureInfo.InvariantCulture);
string humidityPercent = measurement.RelativeHumidityPercent.ToString("F2", CultureInfo.InvariantCulture);
Console.WriteLine($"measured     {celsius} C, {hectopascals} hPa, {humidityPercent} %");

// The script is spent: every transfer the datasheet lists was made, and no other.
Console.WriteLine($"bus          {bus.Transfers} transfers, unexpected: {(!bus.Done).ToString().ToLowerInvariant()}");
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
