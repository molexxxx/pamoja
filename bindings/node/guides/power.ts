// The power-scheduling guide example; see docs/guides/power.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { DutyCycle, PowerMode, PowerPlan } from '@pamoja/power'

// A solar node samples every minute while the charge is healthy, stretches to ten
// minutes to conserve, and to an hour once the battery is nearly flat. Durations cross
// the binding as microseconds.
const plan = new PowerPlan(60_000_000, 600_000_000, 3_600_000_000)

// The default thresholds enter saver mode below 50% charge and critical below 20%.
assert.equal(plan.mode(0.8), PowerMode.Active)
assert.equal(plan.intervalUs(0.8), 60_000_000)
assert.equal(plan.mode(0.35), PowerMode.Saver)
assert.equal(plan.intervalUs(0.35), 600_000_000)
assert.equal(plan.mode(0.12), PowerMode.Critical)
assert.equal(plan.intervalUs(0.12), 3_600_000_000)

// A panel that is delivering buys back one mode, so the same flat battery keeps
// reporting on the ten-minute saver cadence while the sun is on it.
assert.equal(plan.modeWhileCharging(0.12, true), PowerMode.Saver)

// The work is the same two seconds whichever mode the node is in; stretching the cycle
// is what saves the energy. The duty fraction is the proxy for average draw, so the
// hourly cadence costs a sixtieth of what the one-minute cadence does.
const awakeUs = 2_000_000
const healthy = new DutyCycle(awakeUs, plan.intervalUs(0.8) - awakeUs)
const flat = new DutyCycle(awakeUs, plan.intervalUs(0.12) - awakeUs)
assert.ok(Math.abs(healthy.fraction - 2 / 60) < 1e-6)
assert.ok(Math.abs(flat.fraction - 2 / 3600) < 1e-6)

// Stating the budget as a fraction instead gives the awake time directly.
const quarter = DutyCycle.fromFraction(1_000_000, 0.25)
assert.equal(quarter.activeUs, 250_000)
assert.equal(quarter.sleepUs, 750_000)
// ANCHOR_END: example
