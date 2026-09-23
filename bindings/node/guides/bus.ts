// The event bus guide example; see docs/guides/bus.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { EventPublisher } from '@pamoja/bus'

const QUIET_MS = 50

async function main() {
  // The station's wiring makes one bus and hands each part what it needs: a publisher
  // to announce, an endpoint to listen. No part holds a reference to another, so any of
  // them can be replaced without touching the rest.
  const bus = new EventPublisher(2)
  const power = bus.publisher()
  const sampler = bus.publisher()
  const heater = bus.subscribe()
  const logger = bus.subscribe()

  // One announcement reaches every part that listens, and each reads its own copy.
  const reached = power.publish('battery.low')
  console.log(`power     handed battery.low to ${reached} parts`)
  const heaterTook = await heater.nextText()
  console.log(`heater    took ${heaterTook}`)
  const loggerTook = await logger.nextText()
  console.log(`logger    took ${loggerTook}`)

  // Publishing never waits, even while the part's own wait is open, and a part hears
  // what it publishes.
  const waiting = heater.nextText()
  heater.publish('heater.off')
  const heard = await waiting
  console.log(`heater    heard its own ${heard}, sent while it waited`)

  // A part that joins late sees only what is published after it subscribes. There is
  // no history to replay.
  const radio = bus.subscribe()
  power.publish('battery.ok')
  const first = await radio.nextText()
  console.log(`radio     joined late, so the first event it sees is ${first}`)

  // Each endpoint buffers two events. The logger, busy writing to flash, falls behind
  // while the sampler publishes five readings: it loses the oldest events, resumes with
  // the newest, and counts what it lost.
  for (let reading = 0; reading < 5; reading += 1) {
    sampler.publish(`wind ${reading}`)
  }
  const resumed = await logger.nextText()
  const missed = logger.missed
  console.log(`logger    missed ${missed} and resumes at ${resumed}`)
  const newest = await logger.nextText()
  console.log(`logger    then took ${newest}`)

  // A wait with a limit gives up without taking anything, so a part can do other work
  // between events and lose nothing by it.
  try {
    await logger.nextText(QUIET_MS)
    console.log('logger    took an event no one published, which should never happen')
  } catch {
    console.log(`logger    heard nothing more within ${QUIET_MS} ms`)
  }

  return { reached, heaterTook, loggerTook, heard, first, missed, resumed, newest }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: {
  reached: number
  heaterTook: string
  loggerTook: string
  heard: string
  first: string
  missed: number
  resumed: string
  newest: string
}): void {
  assert.equal(seen.reached, 2)
  assert.equal(seen.heaterTook, 'battery.low')
  assert.equal(seen.loggerTook, 'battery.low')
  assert.equal(seen.heard, 'heater.off')
  assert.equal(seen.first, 'battery.ok')
  assert.equal(seen.missed, 5)
  assert.equal(seen.resumed, 'wind 3')
  assert.equal(seen.newest, 'wind 4')
}
