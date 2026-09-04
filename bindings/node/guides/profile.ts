// The device-profile guide example; see docs/guides/profile.md.

// ANCHOR: example
import assert from 'node:assert/strict'

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
assert.equal(profile.name, 'brooder-heater')
assert.equal(profile.topic, 'poultry/brooder/temperature')
assert.equal(profile.control.kind, ControlKind.Setpoint)
assert.equal(profile.control.setpoint, 32.0)
assert.equal(profile.control.cooling, false)
assert.equal(profile.power.activeSecs, 120)
assert.equal(profile.power.saverBelow, 0.5)

// The manifest is the whole control loop. At 27.5 C the reading is below the deadband, so
// the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
const reaction = profile.controller().evaluate(27.5)
assert.equal(reaction.actuator, true)
assert.equal(reaction.alert?.kind, AlertKind.OutOfRange)
assert.equal(reaction.alert?.reading, 27.5)

// Serializing writes the defaulted fields out in full, so a profile edited on a device and
// shared back carries no value the next reader has to infer.
const shared = profile.toJson()
assert.ok(shared.includes('"saver_below"'))
assert.equal(Profile.fromJson(shared).control.setpoint, 32.0)
// ANCHOR_END: example
