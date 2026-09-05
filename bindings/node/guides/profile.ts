// The device-profile guide example; see docs/guides/profile.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { AlertKind, ControlKind, Profile } from '@pamoja/profile'

// A profile is plain data, so a fleet ships one as a file rather than as code. The two
// power thresholds are optional and fall back to the documented defaults.
const manifest = `{
  "name": "brooder-heater",
  "topic": "poultry/brooder/temperature",
  "control": {
    "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
    "cooling": false, "safe_band": 4.0
  },
  "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
}`

const profile = Profile.fromJson(manifest)
console.log(`${profile.name} reports on ${profile.topic}`)
console.log(`wakes every ${profile.power.activeSecs}s while the battery is healthy`)
console.log(`saver mode below ${(profile.power.saverBelow * 100).toFixed(0)}% charge`)

// The manifest is the whole control loop. At 27.5 C the reading is below the deadband, so
// the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
const cold = profile.controller().evaluate(27.5)
console.log(`at 27.5 C: lamp ${cold.actuator}, alert ${cold.alert?.kind}`)

// Back inside the deadband the lamp is left as it was, and nothing is raised.
const settled = profile.controller().evaluate(32.2)
console.log(`at 32.2 C: lamp ${settled.actuator}, alert ${settled.alert}`)

// Serializing writes the defaulted fields out in full, so a profile edited on a device and
// shared back carries no value the next reader has to infer.
const shared = profile.toJson()
console.log(`shared form names its defaults: ${shared.includes('saver_below')}`)
// ANCHOR_END: example

assert.equal(profile.control.kind, ControlKind.Setpoint)
assert.equal(profile.control.setpoint, 32.0)
assert.equal(profile.control.cooling, false)
assert.equal(profile.power.activeSecs, 120)
assert.equal(profile.power.saverBelow, 0.5)
assert.equal(cold.actuator, true)
assert.equal(cold.alert?.kind, AlertKind.OutOfRange)
assert.equal(settled.alert ?? null, null)
assert.ok(shared.includes('saver_below'))
