# @pamoja/gpio

I2C address frames with reserved-range checks, the four SPI clock modes, and active-high or active-low pins. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/gpio.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/gpio
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/gpio.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gpio.ts):

```typescript
import { PinEdge, PinLevel, PinPolarity, i2c, pin, spi } from '@pamoja/gpio'

// A BME280 answers at the 7-bit address its datasheet gives. That is not the byte that
// goes on the wire: the address shifts up one and the low bit says whether this
// transaction reads or writes, which is the step easiest to get wrong by hand.
const BME280 = 0x76
const hex = (byte: number) => `0x${byte.toString(16).toUpperCase()}`
console.log(`write to  ${hex(i2c.addressFrame(BME280)[0]!)}`)
console.log(`read from ${hex(i2c.addressFrame(BME280, { read: true })[0]!)}`)

// The I2C specification keeps two ranges of addresses for itself, so a part answering in
// either is a wiring mistake rather than a device.
console.log(
  `${hex(BME280)} reserved: ${i2c.isReserved(BME280)}, 0x78 reserved: ${i2c.isReserved(0x78)}`,
)

// A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
// driver has to send a different number of bytes depending on the address it holds.
console.log(`a 10-bit address takes ${i2c.frameLen(0x2a5, true)} bytes`)

// Datasheets quote clock polarity and phase as one mode number. Mode 3 idles the clock
// high and samples on the trailing edge.
const clock = spi.clockFor(3)
console.log(`spi mode 3: idles high ${clock.cpol}, samples on the trailing edge ${clock.cpha}`)

// A relay board sold as active low energises when its pin is driven low. The polarity
// carries that inversion, so no call site has to remember which way round it is.
const energise = pin.levelFor(PinPolarity.ActiveLow, true)
console.log(`to energise an active-low relay, drive the pin ${energise}`)

// Releasing it drives the line back high, an edge a falling trigger ignores.
const rising = pin.triggers(PinEdge.Rising, PinLevel.Low, PinLevel.High)
const falling = pin.triggers(PinEdge.Falling, PinLevel.Low, PinLevel.High)
console.log(`release seen by a rising trigger: ${rising}, by a falling trigger: ${falling}`)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-gpio`](https://crates.io/crates/pamoja-gpio) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html), [docs.rs](https://docs.rs/pamoja-gpio) |
| TypeScript | [`@pamoja/gpio`](https://www.npmjs.com/package/@pamoja/gpio) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html) |
| Python | [`pamoja-gpio`](https://pypi.org/project/pamoja-gpio/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html) |
| C# | [`Pamoja.Gpio`](https://www.nuget.org/packages/Pamoja.Gpio) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html) |

## Documentation

- [`@pamoja/gpio` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html), every class, function, and type this package exports.
- [The I2C, SPI, and GPIO guide](https://pamoja.molex.cloud/docs/guides/gpio.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
