// The power-budget guide example; see docs/guides/power.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { DutyCycle, PowerMode, PowerPlan } from '@pamoja/power'

// A solar node samples every minute while the charge is healthy, stretches to ten minutes
// to conserve, and to an hour once the battery is nearly flat. Durations cross the binding
// as microseconds.
const plan = new PowerPlan(60_000_000, 600_000_000, 3_600_000_000)

// The default thresholds enter saver mode below 50% charge and critical below 20%.
for (const charge of [0.8, 0.35, 0.12]) {
  const every = plan.intervalUs(charge) / 1_000_000
  console.log(
    `at ${(charge * 100).toFixed(0)}% charge: ${plan.mode(charge)}, sampling every ${every}s`,
  )
}

// A panel that is delivering buys back one mode. The interval for a charge knows nothing of
// the panel, so the cadence comes from the mode the panel bought.
const charging = plan.modeWhileCharging(0.12, true)
const chargingEvery = plan.intervalForUs(charging) / 1_000_000
console.log(`at 12% charge while charging: ${charging}, sampling every ${chargingEvery}s`)

// A charge worked out from a fuel gauge that did not answer is not a number. The plan takes
// it as critical, so a node that cannot tell what it has left does the least until it can.
const unknown = Number.NaN
const unknownEvery = plan.intervalUs(unknown) / 1_000_000
console.log(
  `with no reading from the gauge: ${plan.mode(unknown)}, sampling every ${unknownEvery}s`,
)

// The thresholds say how long the battery must carry the node without sun. Winter nights
// are long, so a winter plan starts saving sooner and goes critical sooner.
const winter = plan.withThresholds(0.7, 0.3)
const saver = (winter.saverBelow * 100).toFixed(0)
const critical = (winter.criticalBelow * 100).toFixed(0)
console.log(`the winter plan saves below ${saver}% and goes critical below ${critical}%`)
const cold = winter.mode(0.6)
const mild = plan.mode(0.6)
console.log(`at 60% charge: ${cold} in winter, ${mild} by default`)

// A fuel gauge wanders a point or two between readings, so a charge sitting at a threshold
// would change the cadence on every cycle. `nextMode` takes the mode the node is in: it drops
// as soon as the charge falls below a threshold, and climbs back only once the charge is the
// plan's hysteresis margin clear of it.
const wandering = [0.49, 0.51, 0.5, 0.53, 0.48, 0.52]
const walk = (governor: PowerPlan): string => {
  let mode: PowerMode = PowerMode.Active
  const modes: string[] = []
  for (const charge of wandering) {
    mode = governor.nextMode(mode, charge)
    modes.push(mode)
  }
  return modes.join(', ')
}
const flapping = walk(plan.withHysteresis(0))
console.log(`a charge wandering around 50% with no margin: ${flapping}`)
const margin = (plan.hysteresis * 100).toFixed(0)
const settled = walk(plan)
console.log(`and with the ${margin} point margin: ${settled}`)
const back = plan.nextMode(PowerMode.Saver, 0.56)
console.log(`at 56% the charge has cleared the margin: ${back}`)

// The work is the same two seconds whichever mode the node is in; stretching the cycle is
// what saves the energy. The duty fraction is the proxy for average draw, so the hourly
// cadence costs a sixtieth of what the one-minute cadence does.
const awakeUs = 2_000_000
const healthy = new DutyCycle(awakeUs, plan.intervalUs(0.8) - awakeUs)
const flat = new DutyCycle(awakeUs, plan.intervalUs(0.12) - awakeUs)
console.log(`awake ${(healthy.fraction * 100).toFixed(2)}% of the time when healthy`)
console.log(`awake ${(flat.fraction * 100).toFixed(3)}% of the time when flat`)

// A node that lives on its panel can stay awake for the share of the time the harvest pays
// for. Asleep it draws next to nothing, so that share is the harvest over what it draws
// awake, and the duty cycle turns it into time.
const minuteUs = 60_000_000
const awakeMw = 120
const cloudy = DutyCycle.fromFraction(minuteUs, 12 / awakeMw)
console.log(`a 12 mW harvest pays for ${cloudy.activeUs / 1000}ms awake in each minute`)

// The share is clamped, so a harvest above the draw keeps the node awake throughout, and a
// harvest the meter could not read keeps it asleep until one can be.
const sunny = DutyCycle.fromFraction(minuteUs, 150 / awakeMw)
const unread = DutyCycle.fromFraction(minuteUs, Number.NaN)
console.log(`a 150 mW harvest keeps it awake all ${sunny.activeUs / 1000}ms`)
console.log(`an unread harvest keeps it asleep all ${unread.sleepUs / 1000}ms`)
// ANCHOR_END: example

assert.equal(plan.mode(0.8), PowerMode.Active)
assert.equal(plan.intervalUs(0.8), 60_000_000)
assert.equal(plan.mode(0.35), PowerMode.Saver)
assert.equal(plan.mode(0.12), PowerMode.Critical)
assert.equal(plan.intervalUs(0.12), 3_600_000_000)
assert.equal(charging, PowerMode.Saver)
assert.equal(plan.intervalForUs(charging), 600_000_000)
assert.equal(plan.mode(unknown), PowerMode.Critical)
assert.equal(plan.intervalUs(unknown), 3_600_000_000)
assert.equal(cold, PowerMode.Saver)
assert.equal(mild, PowerMode.Active)
assert.equal(flapping, 'Saver, Active, Active, Active, Saver, Active')
assert.equal(settled, 'Saver, Saver, Saver, Saver, Saver, Saver')
assert.equal(back, PowerMode.Active)
assert.ok(Math.abs(healthy.fraction - 2 / 60) < 1e-6)
assert.ok(Math.abs(flat.fraction - 2 / 3600) < 1e-6)
assert.equal(cloudy.activeUs, 6_000_000)
assert.equal(cloudy.periodUs, minuteUs)
assert.equal(sunny.activeUs, minuteUs)
assert.equal(sunny.sleepUs, 0)
assert.equal(unread.activeUs, 0)
assert.equal(unread.sleepUs, minuteUs)
assert.throws(() => new DutyCycle(-1, 0), /activeUs must be a whole number of microseconds/)
assert.throws(() => new PowerPlan(1.5, 0, 0), /activeUs must be a whole number/)
