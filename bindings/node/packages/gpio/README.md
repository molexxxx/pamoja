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
import assert from 'node:assert/strict'

import { PinEdge, PinLevel, PinPolarity, i2c, pin, spi } from '@pamoja/gpio'

// A BME280 answers at 7-bit address 0x76, which is not the byte that goes on the wire:
// the address shifts up one and the read/write bit fills the low bit.
assert.deepEqual([...i2c.addressFrame(0x76)], [0xec])
assert.deepEqual([...i2c.addressFrame(0x76, { read: true })], [0xed])

// UM10204 keeps 0x00..0x07 and 0x78..0x7F for itself, so an address in either range is
// a wiring mistake rather than a device.
assert.equal(i2c.isReserved(0x76), false)
assert.equal(i2c.isReserved(0x78), true)

// A 10-bit address spends the reserved 11110 prefix over two bytes: the prefix, the top
// two address bits, and the read/write bit, then the low eight bits.
assert.equal(i2c.frameLen(0x2a5, true), 2)
assert.deepEqual([...i2c.addressFrame(0x2a5, { tenBit: true })], [0xf4, 0xa5])
assert.deepEqual([...i2c.addressFrame(0x2a5, { read: true, tenBit: true })], [0xf5, 0xa5])

// Datasheets quote clock polarity and phase as one mode number, (CPOL << 1) | CPHA, so
// mode 3 idles the clock high and samples on the trailing edge.
assert.deepEqual(spi.clockFor(3), { cpol: true, cpha: true })
assert.equal(spi.modeFor(true, false), 2)

// A relay board sold as active low energises when its pin is driven low. The polarity
// carries that inversion so no call site has to remember it.
const relay = PinPolarity.ActiveLow
const energised = pin.levelFor(relay, true)
assert.equal(energised, PinLevel.Low)
assert.equal(pin.isAsserted(relay, energised), true)

// Releasing the relay drives the line back high, an edge a falling trigger ignores.
const released = pin.invert(energised)
assert.equal(pin.triggers(PinEdge.Rising, energised, released), true)
assert.equal(pin.triggers(PinEdge.Falling, energised, released), false)
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
