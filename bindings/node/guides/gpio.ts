// The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md.

// ANCHOR: example
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
// ANCHOR_END: example
