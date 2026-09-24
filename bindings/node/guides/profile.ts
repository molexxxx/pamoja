// The device-profile guide example; see docs/guides/profile.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Profile, Viz } from '@pamoja/profile'

// A profile is plain data, so a fleet ships one as a file rather than as code. This
// manifest names no battery thresholds, so the documented defaults apply.
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
console.log(`profile   ${profile.name} reports on ${profile.topic}`)
console.log(
  `defaults  the file names no battery thresholds, so saver starts below ${(profile.power.saverBelow * 100).toFixed(0)}% and critical below ${(profile.power.criticalBelow * 100).toFixed(0)}%`,
)

// The schedule becomes a power plan, which says what mode a charge puts the node in and
// how long it waits between samples there, in microseconds.
const plan = profile.powerPlan()
for (const charge of [0.8, 0.3, 0.1]) {
  console.log(
    `battery   at ${(charge * 100).toFixed(0)}% it runs ${plan.mode(charge)} and samples every ${plan.intervalUs(charge) / 1_000_000} s`,
  )
}

// One controller runs for the life of the node, because it remembers whether the lamp is
// on. The lamp switches on at 31.5 C or below and off at 32.5 C or above, the setpoint
// less and plus the hysteresis, and in between it stays as it was. A reading more than
// 4 C from the setpoint raises an alert as well.
const controller = profile.controller()
let lamp = false
for (const reading of [27.5, 31.8, 32.6, 32.1, 31.4]) {
  const reaction = controller.evaluate(reading)
  const on = reaction.actuator === true
  const change = on ? (lamp ? 'lamp stays on' : 'lamp on') : lamp ? 'lamp off' : 'lamp stays off'
  const alert = reaction.alert ? `, alert ${reaction.alert.kind}` : ''
  console.log(`${`${reading} C`.padEnd(10)}${change}${alert}`)
  lamp = on
}

// Written back out, the manifest names the thresholds the file left to their defaults,
// so the next reader has nothing to infer, and it loads as the same profile.
const shared = profile.toJson()
if (shared.includes('saver_below') && Profile.fromJson(shared).toJson() === shared) {
  console.log('shared    written back out, it names saver_below and loads as the same profile')
}

// The manifest also carries how a dashboard draws the node: one element here, the
// brooder's temperature on a thermometer with the band the chicks are safe in.
const drawn = profile.withPresentation({
  elements: [
    {
      key: 'brooder_temperature',
      unit: 'celsius',
      label: 'Brooder temperature',
      viz: Viz.Thermometer,
      band: [28, 36],
    },
  ],
})
const element = drawn.presentation?.elements[0]
console.log(
  `draws     ${element?.key} in ${element?.unit} on a ${element?.viz}, safe from ${element?.band?.[0]} to ${element?.band?.[1]}`,
)
// ANCHOR_END: example

assert.equal(lamp, true)
assert.equal(profile.power.saverBelow, 0.5)

// ANCHOR: kinds
import { AlertKind } from '@pamoja/profile'

// A level warns before a tank or a well runs dry. The shipped well profile counts 0.5 m
// as dry and warns once the last fall puts dry six samples away or nearer.
const well = Profile.wellLevel().controller()
for (const depth of [5.0, 4.4, 3.8]) {
  const alert = well.evaluate(depth).alert
  if (alert?.kind === AlertKind.RunningOut) {
    console.log(`well      ${depth} m: dry in ${alert.samples} samples at this rate, RunningOut`)
  } else {
    console.log(`well      ${depth} m: no warning yet`)
  }
}

// A surge warns when a reading moves too far in one sample. The shipped flood sensor warns
// when a river rises more than 0.3 m between two readings.
const river = Profile.floodSensor().controller()
for (const gauge of [1.2, 1.35, 1.9]) {
  const alert = river.evaluate(gauge).alert
  if (alert?.kind === AlertKind.ChangingFast) {
    console.log(`river     ${gauge} m: up ${alert.rate?.toFixed(2)} m in one sample, ChangingFast`)
  } else {
    console.log(`river     ${gauge} m: no warning`)
  }
}
// ANCHOR_END: kinds

// ANCHOR: wrong
import { ControlKind } from '@pamoja/profile'

// A probe that fails reports a reading that is not a number. The controller raises it
// rather than going quiet, and the lamp holds its state; what off means for the chicks is
// the node's call.
const failed = controller.evaluate(Number.NaN)
if (failed.alert) {
  const holds = failed.actuator === true ? 'on' : 'off'
  console.log(`probe     a reading of NaN raises ${failed.alert.kind}, and the lamp holds ${holds}`)
}

// A controller built again for each reading forgets the lamp was on, so inside the
// deadband it switches the lamp off.
const first = profile.controller().evaluate(27.5).actuator
const then = profile.controller().evaluate(31.8).actuator
if (first === true && then === false) {
  console.log(
    'fresh     built again for each reading, the controller turns the lamp off at 31.8 C',
  )
}

// A manifest no node could run is refused as it loads, with the reason.
for (const edited of [
  manifest.replace('"hysteresis": 0.5', '"hysteresis": 0.0'),
  manifest.replace('"saver_secs": 600', '"saver_secs": 60'),
]) {
  try {
    Profile.fromJson(edited)
    console.log('a manifest no node could run was accepted, which should never happen')
  } catch (error) {
    console.log(`refused   ${(error as Error).message}`)
  }
}

// A misspelled optional field is not an error: it names no field, so the default stays.
// Writing the profile back out shows what the node understood.
const misspelled = manifest.replace(
  '"critical_secs": 1800 }',
  '"critical_secs": 1800, "saver_bellow": 0.3 }',
)
const understood = Profile.fromJson(misspelled)
console.log(
  `typo      saver_bellow names no field, so saver still starts below ${(understood.power.saverBelow * 100).toFixed(0)}%`,
)

// A kind the library does not ship loads with its parameters and runs as a monitor until
// the node supplies the policy, so it drives nothing and raises nothing.
const custom = Profile.fromJson(manifest.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
if (custom.control.kind === ControlKind.Custom) {
  const reaction = custom.controller().evaluate(27.5)
  if (reaction.actuator == null && reaction.alert == null) {
    const count = Object.keys(custom.control.params ?? {}).length
    console.log(
      `custom    ${custom.control.customKind} loads with ${count} parameters, and with no policy behind it drives nothing`,
    )
  }
}
// ANCHOR_END: wrong
