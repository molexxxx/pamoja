// The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md.

import assert from 'node:assert/strict'

// ANCHOR: parts
import { PinEdge, PinLevel, PinPolarity, i2c, pin, spi } from '@pamoja/gpio'

// The board's own library drives the line: `onoff` or `rpio` on a Raspberry Pi, a vendor
// SDK on a microcontroller. This stands in for one so the example runs with nothing
// plugged in, and it is the only part a real node replaces.
class Line {
  driven: PinLevel[] = []

  constructor(private readings: PinLevel[] = []) {}

  drive(level: PinLevel): void {
    this.driven.push(level)
  }

  read(): PinLevel {
    return this.readings.shift()!
  }
}
// ANCHOR_END: parts

// ANCHOR: example
// Most relay boards energize when their input is pulled low, and a float switch wired to
// ground closes the same way. Saying "active low" once, here, is what keeps the inversion
// out of every line below it.
const RELAY = PinPolarity.ActiveLow
const FLOAT = PinPolarity.ActiveLow
const pump = new Line()
const float = new Line([PinLevel.High, PinLevel.Low])
console.log(`a pump on an active-low relay runs when its line is ${pin.levelFor(RELAY, true)}`)

// The pump runs while the tank fills. The stand-in line answers open and then closed, so
// this is the real loop with nothing plugged in.
pump.drive(pin.levelFor(RELAY, true))
const whileFilling = pin.isAsserted(FLOAT, float.read())
const onceFilled = pin.isAsserted(FLOAT, float.read())
console.log(`the float reads full: ${whileFilling}, then ${onceFilled}`)

// The moment the float closes is that line going low, which is a falling edge. A watch
// armed for the rising one would sleep through the tank filling.
const closing = pin.triggers(PinEdge.Falling, PinLevel.High, PinLevel.Low)
console.log(`the float closing is a falling edge on that line: ${closing}`)

// Full, so the pump stops, and the levels the line was driven to are the whole
// conversation the board saw.
pump.drive(pin.levelFor(RELAY, false))
const [ran, stopped] = pump.driven
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
console.log(`${hex(i2c.RESERVED_FROM)} is reserved by the specification: ${reserved}`)

// And a datasheet quotes SPI's clock polarity and phase as one mode number.
const clock = spi.clockFor(3)
console.log(`SPI mode 3 idles high: ${clock.cpol}, samples on the trailing edge: ${clock.cpha}`)
// ANCHOR_END: example

assert.equal(pin.levelFor(RELAY, true), PinLevel.Low)
assert.equal(whileFilling, false)
assert.equal(onceFilled, true)
assert.equal(closing, true)
assert.equal(pin.triggers(PinEdge.Rising, PinLevel.High, PinLevel.Low), false)
assert.deepEqual([ran, stopped], [PinLevel.Low, PinLevel.High])
assert.deepEqual([toWrite, toRead], [0xec, 0xed])
assert.equal(i2c.isReserved(0x76), false)
assert.equal(reserved, true)
assert.deepEqual([clock.cpol, clock.cpha], [true, true])
assert.equal(spi.modeFor(true, false), 2)
