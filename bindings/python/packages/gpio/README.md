# pamoja-gpio

I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/gpio.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html)

## Install

```sh
pip install pamoja-gpio
```

```python
from pamoja import gpio
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/gpio.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gpio.py):

```python
# Most relay boards energize when their input is pulled low, and a float switch wired to
# ground closes the same way. Saying "active low" once, here, is what keeps the inversion
# out of every line below it.
RELAY = Polarity.ACTIVE_LOW
FLOAT = Polarity.ACTIVE_LOW
pump = Line()
float_switch = Line([Level.HIGH, Level.LOW])
print(f"a pump on an active-low relay runs when its line is {pin.level_for(RELAY, True).value}")

# The pump runs while the tank fills. The stand-in line answers open and then closed, so
# this is the real loop with nothing plugged in.
pump.drive(pin.level_for(RELAY, True))
while_filling = pin.is_asserted(FLOAT, float_switch.read())
once_filled = pin.is_asserted(FLOAT, float_switch.read())
print(f"the float reads full: {while_filling}, then {once_filled}")

# The moment the float closes is that line going low, which is a falling edge. A watch
# armed for the rising one would sleep through the tank filling.
closing = pin.triggers(Edge.FALLING, Level.HIGH, Level.LOW)
print(f"the float closing is a falling edge on that line: {closing}")

# Full, so the pump stops, and the levels the line was driven to are the whole
# conversation the board saw.
pump.drive(pin.level_for(RELAY, False))
ran, stopped = pump.driven
print(f"running drove the line {ran.value} and stopping drove it {stopped.value}")

# A part on a shared bus answers to an address, and the byte on the wire is not the
# address the datasheet prints: it shifts up one and the low bit says read or write.
to_write = i2c.address_frame(0x76)[0]
to_read = i2c.address_frame(0x76, read=True)[0]
print(f"a part at 0x76 is written to as 0x{to_write:02X} and read from as 0x{to_read:02X}")

# Two ranges belong to the specification itself, so a part answering in either is a wiring
# mistake rather than a device.
reserved = i2c.is_reserved(i2c.RESERVED_FROM)
print(f"0x{i2c.RESERVED_FROM:02X} is reserved by the specification: {reserved}")

# And a datasheet quotes SPI's clock polarity and phase as one mode number.
clock = spi.clock_for(3)
print(f"SPI mode 3 idles high: {clock.cpol}, samples on the trailing edge: {clock.cpha}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-gpio`](https://crates.io/crates/pamoja-gpio) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html), [docs.rs](https://docs.rs/pamoja-gpio), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gpio) |
| TypeScript | [`@pamoja/gpio`](https://www.npmjs.com/package/@pamoja/gpio) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gpio) |
| Python | [`pamoja-gpio`](https://pypi.org/project/pamoja-gpio/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gpio) |
| C# | [`Pamoja.Gpio`](https://www.nuget.org/packages/Pamoja.Gpio) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gpio) |

## Documentation

- [`pamoja.gpio` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html), every class and function in this module.
- [The I2C, SPI, and GPIO guide](https://pamoja.molex.cloud/docs/guides/gpio.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
