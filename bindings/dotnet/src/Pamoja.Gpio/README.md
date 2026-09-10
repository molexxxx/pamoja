# Pamoja.Gpio

I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/gpio.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html)

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
// Most relay boards energize when their input is pulled low, and a float switch
// wired to ground closes the same way. Saying "active low" once, here, is what
// keeps the inversion out of every line below it.
const PinPolarity Relay = PinPolarity.ActiveLow;
const PinPolarity Float = PinPolarity.ActiveLow;
var pump = new Line();
var floatSwitch = new Line(PinLevel.High, PinLevel.Low);
Console.WriteLine(
    $"a pump on an active-low relay runs when its line is {Pin.LevelFor(Relay, true)}");

// The pump runs while the tank fills. The stand-in line answers open and then
// closed, so this is the real loop with nothing plugged in.
pump.Drive(Pin.LevelFor(Relay, true));
bool whileFilling = Pin.IsAsserted(Float, floatSwitch.Read());
bool onceFilled = Pin.IsAsserted(Float, floatSwitch.Read());
Console.WriteLine($"the float reads full: {whileFilling}, then {onceFilled}");

// The moment the float closes is that line going low, which is a falling edge. A
// watch armed for the rising one would sleep through the tank filling.
bool closing = Pin.Triggers(PinEdge.Falling, PinLevel.High, PinLevel.Low);
Console.WriteLine($"the float closing is a falling edge on that line: {closing}");

// Full, so the pump stops, and the levels the line was driven to are the whole
// conversation the board saw.
pump.Drive(Pin.LevelFor(Relay, false));
(PinLevel ran, PinLevel stopped) = (pump.Driven[0], pump.Driven[1]);
Console.WriteLine($"running drove the line {ran} and stopping drove it {stopped}");

// A part on a shared bus answers to an address, and the byte on the wire is not
// the address the datasheet prints: it shifts up one and the low bit says read or
// write.
byte toWrite = I2c.AddressFrame(0x76)[0];
byte toRead = I2c.AddressFrame(0x76, read: true)[0];
Console.WriteLine(
    $"a part at 0x76 is written to as 0x{toWrite:X2} and read from as 0x{toRead:X2}");

// Two ranges belong to the specification itself, so a part answering in either is
// a wiring mistake rather than a device.
bool reserved = I2c.IsReserved(I2c.ReservedFrom);
Console.WriteLine(
    $"0x{I2c.ReservedFrom:X2} is reserved by the specification: {reserved}");

// And a datasheet quotes SPI's clock polarity and phase as one mode number.
SpiClock clock = Spi.ClockFor(3);
Console.WriteLine(
    $"SPI mode 3 idles high: {clock.Cpol}, samples on the trailing edge: {clock.Cpha}");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-gpio`](https://crates.io/crates/pamoja-gpio) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html), [docs.rs](https://docs.rs/pamoja-gpio), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gpio) |
| TypeScript | [`@pamoja/gpio`](https://www.npmjs.com/package/@pamoja/gpio) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gpio) |
| Python | [`pamoja-gpio`](https://pypi.org/project/pamoja-gpio/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gpio) |
| C# | [`Pamoja.Gpio`](https://www.nuget.org/packages/Pamoja.Gpio) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gpio) |

## Documentation

- [`Pamoja.Gpio` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html), every type in this namespace.
- [The I2C, SPI, and GPIO guide](https://pamoja.molex.cloud/docs/guides/gpio.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
