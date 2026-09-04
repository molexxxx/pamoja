# pamoja-gpio

I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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

# A BME280 answers at 7-bit address 0x76, which is not the byte that goes on the wire:
# the address shifts up one and the read/write bit fills the low bit.
assert i2c.address_frame(0x76) == bytes([0xEC])
assert i2c.address_frame(0x76, read=True) == bytes([0xED])

# UM10204 keeps 0x00..0x07 and 0x78..0x7F for itself, so an address in either range is
# a wiring mistake rather than a device.
assert not i2c.is_reserved(0x76)
assert i2c.is_reserved(0x78)

# A 10-bit address spends the reserved 11110 prefix over two bytes: the prefix, the top
# two address bits, and the read/write bit, then the low eight bits.
assert i2c.frame_len(0x2A5, ten_bit=True) == 2
assert i2c.address_frame(0x2A5, ten_bit=True) == bytes([0xF4, 0xA5])
assert i2c.address_frame(0x2A5, read=True, ten_bit=True) == bytes([0xF5, 0xA5])

# Datasheets quote clock polarity and phase as one mode number, (CPOL << 1) | CPHA, so
# mode 3 idles the clock high and samples on the trailing edge.
clock = spi.clock_for(3)
assert clock.cpol and clock.cpha
assert spi.mode_for(True, False) == 2

# A relay board sold as active low energises when its pin is driven low. The polarity
# carries that inversion so no call site has to remember it.
assert pin.level_for(Polarity.ACTIVE_LOW, True) is Level.LOW
assert pin.is_asserted(Polarity.ACTIVE_LOW, Level.LOW)

# Releasing the relay drives the line back high, an edge a falling trigger ignores.
assert pin.triggers(Edge.RISING, Level.LOW, Level.HIGH)
assert not pin.triggers(Edge.FALLING, Level.LOW, Level.HIGH)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-gpio`](https://crates.io/crates/pamoja-gpio) | [docs.rs](https://docs.rs/pamoja-gpio), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html) |
| TypeScript | [`@pamoja/gpio`](https://www.npmjs.com/package/@pamoja/gpio) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html) |
| Python | [`pamoja-gpio`](https://pypi.org/project/pamoja-gpio/) | [`pamoja.gpio`](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html) |
| C# | [`Pamoja.Gpio`](https://www.nuget.org/packages/Pamoja.Gpio) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.I2c.html) |

## Documentation

- [The I2C, SPI, and GPIO guide](https://pamoja.molex.cloud/docs/guides/gpio.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
