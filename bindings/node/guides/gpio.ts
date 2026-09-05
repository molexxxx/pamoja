// The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
  `${hex(BME280)} reserved: ${i2c.isReserved(BME280)}, ` +
    `${hex(i2c.RESERVED_FROM)} reserved: ${i2c.isReserved(i2c.RESERVED_FROM)}`,
)

// A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
// driver has to send a different number of bytes depending on the address it holds.
// This is the worked example UM10204 itself prints.
const TEN_BIT_DEVICE = 0x2a5
console.log(`a 10-bit address takes ${i2c.frameLen(TEN_BIT_DEVICE, true)} bytes`)

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
// ANCHOR_END: example

assert.deepEqual([...i2c.addressFrame(BME280, { read: true })], [0xed])
assert.equal(i2c.isReserved(BME280), false)
assert.equal(i2c.isReserved(i2c.RESERVED_FROM), true)
assert.equal(i2c.frameLen(TEN_BIT_DEVICE, true), 2)
assert.ok(clock.cpol && clock.cpha)
assert.equal(spi.modeFor(true, false), 2)
assert.equal(energise, PinLevel.Low)
assert.ok(pin.isAsserted(PinPolarity.ActiveLow, PinLevel.Low))
assert.ok(rising)
assert.ok(!falling)
