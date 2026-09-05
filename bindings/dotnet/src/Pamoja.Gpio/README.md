# Pamoja.Gpio

I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/gpio.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Gpio
```

```csharp
using Pamoja.Gpio;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs):

```csharp
// A BME280 answers at the 7-bit address its datasheet gives. That is not the byte
// that goes on the wire: the address shifts up one and the low bit says whether
// this transaction reads or writes, which is easiest to get wrong by hand.
const byte Bme280 = 0x76;
Console.WriteLine($"write to  0x{I2c.AddressFrame(Bme280)[0]:X2}");
Console.WriteLine($"read from 0x{I2c.AddressFrame(Bme280, read: true)[0]:X2}");

// The I2C specification keeps two ranges of addresses for itself, so a part
// answering in either is a wiring mistake rather than a device.
Console.WriteLine(
    $"0x{Bme280:X2} reserved: {I2c.IsReserved(Bme280)}, "
    + $"0x{I2c.ReservedFrom:X2} reserved: {I2c.IsReserved(I2c.ReservedFrom)}");

// A 10-bit address spends a reserved prefix over two bytes rather than one, so a
// bus driver sends a different number of bytes depending on the address it holds.
// This is the worked example UM10204 itself prints.
const ushort TenBitDevice = 0x2A5;
Console.WriteLine($"a 10-bit address takes {I2c.FrameLen(TenBitDevice, tenBit: true)} bytes");

// Datasheets quote clock polarity and phase as one mode number. Mode 3 idles the
// clock high and samples on the trailing edge.
SpiClock clock = Spi.ClockFor(3);
Console.WriteLine(
    $"spi mode 3: idles high {clock.Cpol}, samples on the trailing edge {clock.Cpha}");

// A relay board sold as active low energises when its pin is driven low. The
// polarity carries that inversion, so no call site has to remember it.
PinLevel energise = Pin.LevelFor(PinPolarity.ActiveLow, true);
Console.WriteLine($"to energise an active-low relay, drive the pin {energise}");

// Releasing it drives the line back high, an edge a falling trigger ignores.
bool rising = Pin.Triggers(PinEdge.Rising, PinLevel.Low, PinLevel.High);
bool falling = Pin.Triggers(PinEdge.Falling, PinLevel.Low, PinLevel.High);
Console.WriteLine(
    $"release seen by a rising trigger: {rising}, by a falling trigger: {falling}");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-gpio`](https://crates.io/crates/pamoja-gpio) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html), [docs.rs](https://docs.rs/pamoja-gpio) |
| TypeScript | [`@pamoja/gpio`](https://www.npmjs.com/package/@pamoja/gpio) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html) |
| Python | [`pamoja-gpio`](https://pypi.org/project/pamoja-gpio/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html) |
| C# | [`Pamoja.Gpio`](https://www.nuget.org/packages/Pamoja.Gpio) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html) |

## Documentation

- [`Pamoja.Gpio` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html), every type in this namespace.
- [The I2C, SPI, and GPIO guide](https://pamoja.molex.cloud/docs/guides/gpio.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
