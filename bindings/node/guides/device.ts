// A part pamoja has never heard of: a soil probe and a valve of the maker's own, run
// against a rule and published with nothing plugged in.

import assert from 'node:assert/strict'

// ANCHOR: parts
import { Transport } from '@pamoja/core'
import { Calibration, Thermostat } from '@pamoja/kit'
import { Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Replay } from '@pamoja/sim'
import { Store } from '@pamoja/sync'

// A capacitive soil probe on an analog-to-digital converter. Whatever reads the chip is
// `adc`: anything with a `read` that hands back counts, so a replay stands in for it here
// and the converter's own driver does on the node. The probe turns counts into percent,
// and nothing downstream needs to know there was a chip at all.
class SoilProbe {
  constructor(
    private readonly adc: { read(): Promise<number> },
    private readonly calibration: Calibration,
  ) {}

  async read(): Promise<number> {
    return this.calibration.apply(await this.adc.read())
  }
}

// A solenoid valve on a relay. It keeps what it was last told and counts the changes,
// which is what a test needs and what a real one does before it drives the pin.
class Valve {
  open = false
  switched = 0

  async apply(open: boolean): Promise<void> {
    if (open !== this.open) {
      this.open = open
      this.switched += 1
    }
  }
}
// ANCHOR_END: parts

// ANCHOR: example
const TOPIC = 'garden/bed-1/moisture'

async function main() {
  // Bone dry read 3200 counts on this probe and a soaked pot read 1400, measured once and
  // kept. The replay hands back the counts a bed reads as it dries and is watered.
  const counts = new Replay([2900, 2700, 2300, 2450, 2750, 2500])
  const probe = new SoilProbe(counts, Calibration.twoPoint(3200, 0, 1400, 100))
  const valve = new Valve()

  // Water below 30% and stop above 45%. A valve that adds water is what `heating` names,
  // so the band sits at 37.5 with 7.5 either side.
  const rule = Thermostat.heating(37.5, 7.5)

  // The link, with its first two sends failing the way a radio does at dusk. The ladder
  // keeps what it could not send and replays it in order once a send goes through.
  const broker = new LoopbackBroker()
  const gateway = broker.link()
  await gateway.connect()
  await gateway.subscribe(TOPIC)
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.faulty(broker.rung(), 2))
  await ladder.connect()

  // The loop: read, decide, act, publish. Nothing in it knows the probe is homemade.
  for (let sample = 0; sample < 6; sample += 1) {
    const moisture = await probe.read()
    await valve.apply(rule.update(moisture))
    const report = moisture.toFixed(1)
    const delivery = await ladder.send(TOPIC, report)
    console.log(`bed at ${report}%, valve ${valve.open ? 'open' : 'closed'}, ${delivery}`)
    if ((await ladder.buffered()) > 0) {
      const caughtUp = await ladder.flush()
      if (caughtUp > 0) console.log(`link back, ${caughtUp} readings caught up`)
    }
  }
  console.log(`the valve switched ${valve.switched} times`)

  // On the gateway, in the order they were read, outage included.
  const got: string[] = []
  for (let n = 0; n < 6; n += 1) {
    got.push((await gateway.recv())!.text!)
  }
  console.log(`gateway got ${got.join(', ')}`)

  return { got, switched: valve.switched, open: valve.open, left: await ladder.buffered() }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: { got: string[]; switched: number; open: boolean; left: number }): void {
  assert.deepEqual(seen.got, ['16.7', '27.8', '50.0', '41.7', '25.0', '38.9'])
  assert.equal(seen.switched, 3)
  assert.ok(seen.open)
  assert.equal(seen.left, 0)
}
