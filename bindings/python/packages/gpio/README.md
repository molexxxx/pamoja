# pamoja-gpio

I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/gpio.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
from pamoja.gpio import Edge, Level, Polarity, i2c, pin, spi

# A BME280 answers at the 7-bit address its datasheet gives. That is not the byte that
# goes on the wire: the address shifts up one and the low bit says whether this
# transaction reads or writes, which is the step easiest to get wrong by hand.
BME280 = 0x76
print(f"write to  0x{i2c.address_frame(BME280)[0]:02X}")
print(f"read from 0x{i2c.address_frame(BME280, read=True)[0]:02X}")

# The I2C specification keeps two ranges of addresses for itself, so a part answering in
# either is a wiring mistake rather than a device.
reserved = i2c.RESERVED_FROM
print(f"0x{BME280:02X} reserved: {i2c.is_reserved(BME280)}, "
      f"0x{reserved:02X} reserved: {i2c.is_reserved(reserved)}")

# A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
# driver has to send a different number of bytes depending on the address it holds.
# This is the worked example UM10204 itself prints.
TEN_BIT_DEVICE = 0x2A5
print(f"a 10-bit address takes {i2c.frame_len(TEN_BIT_DEVICE, ten_bit=True)} bytes")

# Datasheets quote clock polarity and phase as one mode number. Mode 3 idles the clock
# high and samples on the trailing edge.
clock = spi.clock_for(3)
print(f"spi mode 3: idles high {clock.cpol}, samples on the trailing edge {clock.cpha}")

# A relay board sold as active low energises when its pin is driven low. The polarity
# carries that inversion, so no call site has to remember which way round it is.
energise = pin.level_for(Polarity.ACTIVE_LOW, True)
print(f"to energise an active-low relay, drive the pin {energise.name}")

# Releasing it drives the line back high, an edge a falling trigger ignores.
rising = pin.triggers(Edge.RISING, Level.LOW, Level.HIGH)
falling = pin.triggers(Edge.FALLING, Level.LOW, Level.HIGH)
print(f"release seen by a rising trigger: {rising}, by a falling trigger: {falling}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-gpio`](https://crates.io/crates/pamoja-gpio) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html), [docs.rs](https://docs.rs/pamoja-gpio) |
| TypeScript | [`@pamoja/gpio`](https://www.npmjs.com/package/@pamoja/gpio) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html) |
| Python | [`pamoja-gpio`](https://pypi.org/project/pamoja-gpio/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html) |
| C# | [`Pamoja.Gpio`](https://www.nuget.org/packages/Pamoja.Gpio) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html) |

## Documentation

- [`pamoja.gpio` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html), every class and function in this module.
- [The I2C, SPI, and GPIO guide](https://pamoja.molex.cloud/docs/guides/gpio.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
