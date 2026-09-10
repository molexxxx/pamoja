// The rules guide example; see docs/guides/rules.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Trigger } from '@pamoja/kit'
import { LoopbackBroker } from '@pamoja/loopback'

// A rule is a file: the topic it watches, the line a reading crosses, the release band
// that stops it firing over and over, and what to do on the way down and on the way
// back. The same file runs in every language; here the program reads it as data and
// drives the loop itself, with the kit's trigger deciding the condition.
const rules = JSON.parse(`{ "rules": [ {
  "name": "water-when-dry",
  "when": { "topic": "garden/bed-1/moisture", "compare": "below",
            "threshold": 30.0, "hysteresis": 5.0 },
  "then": [ { "do": "drive", "actuator": "bed-valve", "on": true },
            { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" } ],
  "otherwise": [ { "do": "drive", "actuator": "bed-valve", "on": false },
                 { "do": "publish", "topic": "garden/bed-1/valve", "payload": "closed" } ]
} ] }`)
const rule = rules.rules[0]
const when = rule.when
const clears = when.threshold + when.hysteresis
console.log(`the rule watches ${when.topic} ${when.compare} ${when.threshold}, clearing above ${clears}`)

async function main() {
  // Three parties on one broker: the node that reads the bed, the engine that holds the
  // valve, and a watcher on the topic the rule publishes to.
  const broker = new LoopbackBroker()
  const probe = broker.link()
  const engine = broker.link()
  const watcher = broker.link()
  await probe.connect()
  await engine.connect()
  await watcher.connect()
  await watcher.subscribe('garden/bed-1/valve')
  await engine.subscribe(when.topic)

  // The condition is the trigger; the actions are the program's own.
  const trigger =
    when.compare === 'below'
      ? Trigger.below(when.threshold, when.hysteresis)
      : Trigger.above(when.threshold, when.hysteresis)
  const valve = { open: false, switches: 0 }
  const run = async (actions: Array<Record<string, string | boolean>>) => {
    for (const action of actions) {
      if (action.do === 'drive') {
        valve.open = action.on as boolean
        valve.switches += 1
      } else {
        await engine.send(action.topic as string, action.payload as string)
      }
    }
  }

  // The bed dries out and is watered back: the rule fires once on the way down and once
  // on the way back, and holds its state for the readings in between.
  for (const reading of [42, 31, 28, 33, 36]) {
    await probe.send(when.topic, String(reading))
    const message = (await engine.recv())!
    const edge = trigger.update(message.number!)
    if (edge === 'set') await run(rule.then)
    else if (edge === 'cleared') await run(rule.otherwise)
    console.log(`${reading}: ${edge ?? 'no edge'}, valve ${valve.open ? 'on' : 'off'}`)
  }

  // The watcher on the other topic heard each edge as the rule published it.
  const heard: string[] = []
  for (let i = 0; i < 2; i += 1) {
    heard.push((await watcher.recv())!.text!)
  }
  console.log(`the watcher heard ${heard.join(', ')}`)
  console.log(`the valve switched ${valve.switches} times`)

  return { heard, valve }
}

main()
// ANCHOR_END: example
  .then(check)

function check({ heard, valve }: { heard: string[]; valve: { open: boolean; switches: number } }): void {
  assert.deepEqual(heard, ['open', 'closed'])
  assert.equal(valve.open, false)
  assert.equal(valve.switches, 2)
}
