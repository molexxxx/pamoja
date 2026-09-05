// The helpers guide example; see docs/guides/kit.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Calibration, Median, Thermostat } from '@pamoja/kit'

// A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is full,
// so the span is 16 mA and mid-scale is 12 mA, not 10.
const level = Calibration.twoPoint(4, 0, 20, 100)
console.log(`12 mA is ${level.apply(12)}% full, 4 mA is ${level.apply(4)}%`)

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
// scale rather than an empty tank.
const broken = level.apply(0)
console.log(`a dead loop reads ${broken}%, which is not a level at all`)

// A median window drops that sample outright, where an average would blend a quarter of
// the range into every reading after it.
const filtered = new Median()
let percent = 0
for (const milliamps of [12, 12, 0, 12, 12]) {
  percent = level.apply(filtered.update(milliamps))
}
console.log(`through the dropout, the level held at ${percent}%`)

// A refill pump runs when the level falls below the deadband, which is the direction
// heating names; nothing about it is specific to temperature. The deadband stops a level
// sitting on the threshold from chattering the contactor.
const pump = Thermostat.heating(50, 10)
for (const reading of [percent, 38, 45, 62]) {
  console.log(`at ${reading}% the pump is ${pump.update(reading) ? 'on' : 'off'}`)
}
// ANCHOR_END: example

assert.equal(level.apply(12), 50)
assert.equal(level.apply(4), 0)
assert.equal(broken, -25)
assert.equal(percent, 50)

const again = Thermostat.heating(50, 10)
assert.equal(again.update(50), false)
assert.equal(again.update(38), true)
assert.equal(again.update(45), true)
assert.equal(again.update(62), false)
