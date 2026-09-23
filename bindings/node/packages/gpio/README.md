# @pamoja/gpio

I2C address frames with reserved-range checks, the four SPI clock modes, active-high or active-low switches and contacts, and GPIO lines opened on a Linux board. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/gpio.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html)

## Install

```sh
npm install @pamoja/gpio
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/gpio.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gpio.ts):

```typescript
import { Contact, GpioLine, PinEdge, PinLevel, PinScript, Switch, i2c, pin, spi } from '@pamoja/gpio'

// Most relay boards energize when their input is pulled low, and a float switch wired to
// ground closes the same way. Saying "active low" once, here, is what keeps the inversion
// out of every line below it.
const pump = Switch.activeLow(new PinScript())
const float = Contact.activeLow(new PinScript([PinLevel.High, PinLevel.Low]))
const runsOn = pin.levelFor(pump.polarity, true)
console.log(`a pump on an active-low relay runs when its line is ${runsOn}`)

// The pump runs while the tank fills. The scripted line answers open and then closed, so
// this is the real loop with nothing plugged in; on a board the same two lines take a pin
// from the board's GPIO library instead.
pump.set(true)
const whileFilling = float.isAsserted()
const onceFilled = float.isAsserted()
const full = (closed: boolean) => (closed ? 'full' : 'not full')
console.log(`the float reads ${full(whileFilling)}, then ${full(onceFilled)}`)

// The moment the float closes is that line going low, which is a falling edge. A watch
// armed for the rising one would sleep through the tank filling.
const closing = pin.triggers(PinEdge.Falling, PinLevel.High, PinLevel.Low)
console.log(`the float closing is ${closing ? 'a falling edge' : 'not a falling edge'} on that line`)

// Full, so the pump stops. Releasing the switch hands the line back, and the levels it was
// driven to are the whole conversation the board saw.
pump.set(false)
const [ran, stopped] = pump.release().driven
console.log(`running drove the line ${ran} and stopping drove it ${stopped}`)

// A part on a shared bus answers to an address, and the byte on the wire is not the
// address the datasheet prints: it shifts up one and the low bit says read or write.
const hex = (byte: number) => `0x${byte.toString(16).toUpperCase().padStart(2, '0')}`
const toWrite = i2c.addressFrame(0x76)[0]!
const toRead = i2c.addressFrame(0x76, { read: true })[0]!
console.log(`a part at 0x76 is written to as ${hex(toWrite)} and read from as ${hex(toRead)}`)

// Two ranges belong to the specification itself, so a part answering in either is a wiring
// mistake rather than a device.
const reserved = i2c.isReserved(i2c.RESERVED_FROM)
console.log(`${hex(i2c.RESERVED_FROM)} is ${reserved ? 'reserved by the specification' : 'free for a device'}`)

// And a datasheet quotes SPI's clock polarity and phase as one mode number.
const clock = spi.clockFor(3)
const idle = clock.cpol ? 'high' : 'low'
const edge = clock.cpha ? 'trailing' : 'leading'
console.log(`SPI mode 3 idles ${idle} and samples on the ${edge} edge`)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-gpio`](https://crates.io/crates/pamoja-gpio) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html), [docs.rs](https://docs.rs/pamoja-gpio), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gpio) |
| TypeScript | [`@pamoja/gpio`](https://www.npmjs.com/package/@pamoja/gpio) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gpio) |
| Python | [`pamoja-gpio`](https://pypi.org/project/pamoja-gpio/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gpio) |
| C# | [`Pamoja.Gpio`](https://www.nuget.org/packages/Pamoja.Gpio) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gpio) |

## Documentation

- [`@pamoja/gpio` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html), every class, function, and type this package exports.
- [The I2C, SPI, and GPIO guide](https://pamoja.molex.cloud/docs/guides/gpio.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
