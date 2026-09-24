// The device-profile guide example; see docs/guides/profile.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { readFileSync } from 'node:fs'
import { LoopbackBroker } from '@pamoja/loopback'
import { Node, Profile } from '@pamoja/profile'

// A profile is a file. This one ships in the catalog under profiles/: it holds a brooder at
// 32 C by switching a heat lamp, says what it reads, and says how a dashboard draws it.
const text = readFileSync('profiles/brooder-heater.json', 'utf8')
const profile = Profile.fromJson(text)

async function example(): Promise<boolean> {
  const reads = profile.reads!
  console.log(`profile   ${profile.name} reads ${reads.quantity} in ${reads.unit} and reports on ${profile.topic}`)

  // A node is the profile and the parts that make it run: a sensor, an output, and a link.
  // A morning of readings stands in for the probe, and a dashboard listens on the same
  // broker.
  const broker = new LoopbackBroker()
  const link = broker.link()
  const dashboard = broker.link()
  await link.connect()
  await dashboard.connect()
  await dashboard.subscribe(profile.topic)
  const morning = [27.5, 31.8, 32.6, 32.1, 31.4]
  let lamp = false
  const node = new Node({
    profile,
    read: () => morning.shift()!,
    drive: (on) => {
      lamp = on
    },
    link,
  })

  // Each tick reads, decides, switches the lamp, and publishes the reading. The lamp comes on
  // at 31.5 C or below and goes off at 32.5 C or above, and in between it stays as it was; a
  // reading more than 4 C from 32 raises an alert as well.
  let was = false
  for (let at = 0; at < 5; at += 1) {
    const { reading, reaction } = await node.tick()
    const on = reaction.actuator === true
    const change = on ? (was ? 'lamp stays on' : 'lamp on') : was ? 'lamp off' : 'lamp stays off'
    const alert = reaction.alert ? `, alert ${reaction.alert.kind}` : ''
    console.log(`${`${reading} C`.padEnd(10)}${change}${alert}`)
    was = on
  }

  // The dashboard heard every reading the node published.
  const heard: number[] = []
  for (let at = 0; at < 5; at += 1) {
    heard.push((await dashboard.recv())!.number!)
  }
  console.log(`heard     ${heard.join(', ')} on ${profile.topic}`)

  // Between ticks the node waits as long as its battery allows: often on a healthy charge,
  // sparingly on a low one. run() does this until stopped, waiting each interval.
  for (const charge of [0.8, 0.3, 0.1]) {
    const { mode, waitMs } = node.schedule(charge)
    console.log(`battery   at ${(charge * 100).toFixed(0)}% it runs ${mode} and waits ${waitMs / 1000} s`)
  }

  // The same file says how a dashboard draws the node.
  const element = profile.presentation!.elements[0]
  const [low, high] = element.band!
  console.log(`draws     ${element.key} in ${element.unit} on a ${element.viz}, safe from ${low} to ${high}`)
  return lamp
}
// ANCHOR_END: example

// ANCHOR: kinds
import { AlertKind } from '@pamoja/profile'

function kinds(): void {
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
}
// ANCHOR_END: kinds

// ANCHOR: custom
import { type Policy, PolicyRegistry } from '@pamoja/profile'

// A policy of the program's own: the lamp on below the setpoint, and a condition of its own
// when the chicks are chilled.
function brooderGuard(params: Record<string, number | boolean | string>): Policy {
  const setpoint = Number(params.setpoint ?? 32)
  const chilledBelow = setpoint - Number(params.safe_band ?? 4)
  return {
    evaluate: (reading) => ({
      actuator: reading < setpoint,
      alert: reading < chilledBelow ? { kind: AlertKind.Custom, code: 'Chilled', value: reading } : undefined,
    }),
  }
}

async function custom(): Promise<void> {
  // A manifest may name a control kind the library never shipped, with its parameters
  // beside it. The program registers the code that decides it under that name, and the node
  // runs whichever kind the file names.
  const guarded = Profile.fromJson(text.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
  const registry = new PolicyRegistry().register('brooder_guard', brooderGuard)
  console.log(`custom    ${guarded.control.customKind} is decided by the program's own code, registered under its name`)
  const link = new LoopbackBroker().link()
  await link.connect()
  let lamp = false
  const node = new Node({
    profile: guarded,
    policy: registry,
    read: () => 27.5,
    drive: (on) => {
      lamp = on
    },
    link,
  })
  const { reading, reaction } = await node.tick()
  const alert = reaction.alert?.code ?? reaction.alert?.kind ?? 'none'
  console.log(`${`${reading} C`.padEnd(10)}lamp ${lamp ? 'on' : 'off'}, alert ${alert}`)
}
// ANCHOR_END: custom

// ANCHOR: wrong
function wrong(): void {
  // A probe that fails reports a reading that is not a number. The controller raises it
  // rather than going quiet, and the lamp holds its state; what off means for the chicks is
  // the node's call.
  const controller = profile.controller()
  controller.evaluate(27.5)
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
    console.log('fresh     built again for each reading, the controller turns the lamp off at 31.8 C')
  }

  // A manifest no node could run is refused as it loads, with the reason. So is a misspelled
  // field, with the one it was probably meant to be and where it sits, rather than leaving
  // the default in its place without a word.
  for (const edited of [
    text.replace('"hysteresis": 0.5', '"hysteresis": 0.0'),
    text.replace('"saver_secs": 600', '"saver_secs": 60'),
    text.replace('"saver_below"', '"saver_bellow"'),
  ]) {
    try {
      Profile.fromJson(edited)
      console.log('a manifest no node could run was accepted, which should never happen')
    } catch (error) {
      console.log(`refused   ${(error as Error).message}`)
    }
  }

  // A kind the library does not ship loads with its parameters, but no built-in controller
  // decides it, so a node without a registry that knows it is refused rather than running
  // one that never switches the lamp.
  const unknown = Profile.fromJson(text.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
  try {
    unknown.controller()
    console.log('a custom kind ran without its policy, which should never happen')
  } catch (error) {
    console.log(`refused   ${(error as Error).message}`)
  }
}
// ANCHOR_END: wrong

async function main(): Promise<void> {
  const lamp = await example()
  assert.equal(lamp, true, 'the morning ends with the lamp on')
  kinds()
  await custom()
  wrong()
}

main()
