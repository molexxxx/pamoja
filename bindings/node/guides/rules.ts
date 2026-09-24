// The rules guide example; see docs/guides/rules.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { LoopbackBroker } from '@pamoja/loopback'
import { type RuleAction, RuleActionKind, RuleEngine } from '@pamoja/profile'

// A rule is a file: the topic it watches, the line a reading crosses, the release band
// that stops it firing over and over, and what to do on the way down and on the way back.
// Two rules watch one bed here: one waters it when it dries past 30 and stops once it is
// wetter than 35, and one raises an alarm when it is soaked past 60.
const file = `{ "rules": [
  { "name": "water-when-dry",
    "when": { "topic": "garden/bed-1/moisture", "below": 30.0, "hysteresis": 5.0 },
    "then": [ { "drive": "bed-valve", "on": true },
              { "publish": "garden/bed-1/valve", "payload": "open" } ],
    "otherwise": [ { "drive": "bed-valve", "on": false },
                   { "publish": "garden/bed-1/valve", "payload": "closed" } ] },
  { "name": "flood-alarm",
    "when": { "topic": "garden/bed-1/moisture", "above": 60.0, "hysteresis": 5.0 },
    "then": [ { "publish": "garden/alarm", "payload": "waterlogged" } ] }
] }`

// Says what one action does, in the words the output uses.
const described = (action: RuleAction) =>
  action.kind === RuleActionKind.Drive
    ? `drive ${action.actuator} ${action.on ? 'on' : 'off'}`
    : `publish ${action.payload} to ${action.topic}`

async function main() {
  // Three parties on one broker: the node that reads the bed, the engine that holds the
  // valve, and a watcher on the topics the rules publish to.
  const broker = new LoopbackBroker()
  const probe = broker.link()
  const watcher = broker.link()
  const link = broker.link()
  await probe.connect()
  await watcher.connect()
  await link.connect()
  await watcher.subscribe('garden/bed-1/valve')
  await watcher.subscribe('garden/alarm')

  // The engine runs the file off its link: it listens on every topic a rule watches,
  // switches the outputs it was given by the names the file uses, and publishes over the
  // same link. Here the valve is a list of the settings it was given.
  const valve: boolean[] = []
  const engine = new RuleEngine(file, link, { actuators: { 'bed-valve': (on) => valve.push(on) } })
  await engine.listen()
  console.log(`watches   ${engine.topics.join(', ')}, and drives ${engine.actuators.join(', ')}`)

  // The bed dries out, is watered, and floods. A rule fires only as its condition sets or
  // clears, and the readings in between change nothing. At 65 two rules fire on one
  // reading, in the order the file lists them.
  for (const reading of [42, 31, 28, 33, 65, 50]) {
    await probe.send('garden/bed-1/moisture', String(reading))
    const fired = (await engine.step())!
    const at = String(reading).padEnd(10)
    if (fired.length === 0) {
      console.log(`${at}nothing fired`)
    }
    for (const one of fired) {
      if (one.actions.length === 0) {
        console.log(`${at}${one.rule} ${one.edge}, with nothing to do`)
      } else {
        console.log(`${at}${one.rule} ${one.edge}: ${one.actions.map(described).join(', ')}`)
      }
    }
  }

  // The watcher heard every message the rules published, in the order they went out.
  const heard: string[] = []
  for (let i = 0; i < 3; i += 1) {
    heard.push((await watcher.recv())!.text!)
  }
  console.log(`heard     ${heard.join(', ')}`)
  console.log(`valve     switched ${valve.length} times, and it is ${valve[valve.length - 1] ? 'on' : 'off'}`)
  return { heard, valve, engine }
}

main()
  // ANCHOR_END: example
  .then(({ heard, valve, engine }) => {
    assert.deepEqual(heard, ['open', 'closed', 'waterlogged'])
    assert.deepEqual(valve, [true, false])
    assert.equal(engine.isSet('water-when-dry'), false)
    assert.equal(engine.isSet('flood-alarm'), false)
    wrong()
  })

// ANCHOR: wrong
import { RuleEvaluator } from '@pamoja/profile'

function wrong(): void {
  // A program that moves its own messages hands each reading to an evaluator, the engine's
  // deciding half on its own, and carries out what it says.
  const judge = RuleEvaluator.fromJson(file)

  // A reading that is not a number, such as the NaN a failed probe reports, is refused on a
  // watched topic rather than leaving every rule as it was with nothing to say why.
  try {
    judge.evaluate('garden/bed-1/moisture', Number.NaN)
    console.log('a reading of NaN was judged, which should never happen')
  } catch (error) {
    console.log(`refused   ${(error as Error).message}`)
  }

  // A topic no rule watches is not judged at all, so even a NaN there says nothing.
  if (judge.evaluate('garden/bed-2/moisture', Number.NaN).length === 0) {
    console.log('ignored   no rule watches garden/bed-2/moisture, so even a NaN there is not judged')
  }

  // A file no engine could run is refused as it loads, with the rule and the reason.
  for (const edited of [
    file.split('garden/bed-1/moisture').join('garden/+/moisture'),
    file.replace('"flood-alarm"', '"water-when-dry"'),
  ]) {
    try {
      RuleEvaluator.fromJson(edited)
      console.log('a file no engine could run was accepted, which should never happen')
    } catch (error) {
      console.log(`refused   ${(error as Error).message}`)
    }
  }

  // With no release band, readings that hover at the line set and clear the rule on every
  // sample, and each edge switches the valve. The band of 5 holds it through them.
  const fires = (text: string) => {
    const rules = RuleEvaluator.fromJson(text)
    let count = 0
    for (const reading of [29.9, 30.1, 29.8, 30.2]) {
      count += rules.evaluate('garden/bed-1/moisture', reading).length
    }
    return count
  }
  const bare = fires(file.split('"hysteresis": 5.0').join('"hysteresis": 0.0'))
  const banded = fires(file)
  console.log(
    `chatter   4 readings hovering at 30 fire the rule ${bare} times with no release band, ${banded} with a band of 5`,
  )
}
// ANCHOR_END: wrong
