// The helper-math guide example; see docs/guides/kit.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { Calibration, Median, Thermostat } from '@pamoja/kit'

// A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is
// full, so the span is 16 mA and mid-scale is 12 mA, not 10.
const level = Calibration.twoPoint(4, 0, 20, 100)
assert.equal(level.apply(12), 50)
assert.equal(level.apply(4), 0)

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
// scale rather than an empty tank. A median window drops that sample outright, where an
// average would blend a quarter of the range into every reading after it.
assert.equal(level.apply(0), -25)
const filtered = new Median()
let percent = 0
for (const milliamps of [12, 12, 0, 12, 12]) {
  percent = level.apply(filtered.update(milliamps))
  assert.equal(percent, 50)
}

// A refill pump runs when the level falls below the deadband, which is the direction
// heating names; nothing about it is specific to temperature. The deadband stops a level
// sitting on the threshold from chattering the contactor.
const pump = Thermostat.heating(50, 10)
assert.equal(pump.update(percent), false)
assert.equal(pump.update(38), true)
assert.equal(pump.update(45), true)
assert.equal(pump.update(62), false)
// ANCHOR_END: example
