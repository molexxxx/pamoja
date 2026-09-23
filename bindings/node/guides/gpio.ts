// The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
// ANCHOR_END: example

assert.equal(runsOn, PinLevel.Low)
assert.equal(whileFilling, false)
assert.equal(onceFilled, true)
assert.equal(closing, true)
assert.equal(pin.triggers(PinEdge.Rising, PinLevel.High, PinLevel.Low), false)
assert.deepEqual([ran, stopped], [PinLevel.Low, PinLevel.High])
assert.equal(pump.isAsserted, false)
assert.deepEqual([toWrite, toRead], [0xec, 0xed])
assert.equal(i2c.isReserved(0x76), false)
assert.equal(reserved, true)
assert.deepEqual([clock.cpol, clock.cpha], [true, true])
assert.equal(spi.modeFor(true, false), 2)

// ANCHOR: board
// The same pump and float on a Raspberry Pi: the relay board's input on GPIO17 and the
// float switch between GPIO27 and ground. Only the two lines change.
async function onABoard(chip: string): Promise<void> {
  // The relay energizes on a low input, so its line is taken high and the pump stays off
  // until it is asked to run. The float closes to ground against a pull-up.
  const relay = GpioLine.openOutput(chip, 17, PinLevel.High)
  const floatLine = GpioLine.openInput(chip, 27)
  const pump = Switch.activeLow(relay)
  const float = Contact.activeLow(floatLine)

  // Run the pump until the float closes, and stop it whatever happens: a pump left running
  // on a failed float is the fault this whole program exists to prevent.
  const deadline = Date.now() + 10 * 60_000
  pump.set(true)
  try {
    while (!float.isAsserted()) {
      if (Date.now() >= deadline) {
        throw new Error('the tank did not fill in ten minutes; check the float and the supply')
      }
      await new Promise((resolve) => setTimeout(resolve, 100))
    }
  } finally {
    pump.set(false)
    relay.close()
    floatLine.close()
  }
  console.log('the tank is full and the pump is off')
}
// ANCHOR_END: board

const chip = process.env['PAMOJA_GPIO_CHIP']
if (chip) {
  onABoard(chip).catch((error: Error) => {
    console.error(error.message)
    process.exitCode = 1
  })
}
