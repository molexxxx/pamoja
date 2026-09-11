// The event bus guide example; see docs/guides/bus.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { EventBus } from '@pamoja/bus'

async function main() {
  // A sampler announces something and whatever cares picks it up, with neither side
  // holding a reference to the other. This is how the parts of one node are wired.
  const hub = new EventBus(8)
  const control = await hub.subscribe()
  const logger = await hub.subscribe()

  await hub.publish('battery.low')
  const toControl = (await control.nextText())!
  const toLogger = (await logger.nextText())!
  console.log(`control saw ${toControl}, the logger saw ${toLogger}`)

  // A subscriber taken later starts from the next event, so it never sees what went out
  // before it existed.
  const late = await hub.subscribe()
  await hub.publish('link.up')
  const firstSeen = (await late.nextText())!
  console.log(`the late subscriber's first event is ${firstSeen}`)

  // The buffer is per subscriber and bounded, so one further behind than the capacity
  // drops what it missed and resumes with the most recent events. A slow reader costs
  // itself, not the publisher.
  const slow = new EventBus(2)
  const reader = await slow.subscribe()
  for (let count = 0; count < 5; count += 1) {
    await slow.publish(String(count))
  }
  const resumed = (await reader.nextText())!
  console.log(`after five events into a buffer of two, the reader resumes at ${resumed}`)

  return { toControl, toLogger, firstSeen, resumed }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: {
  toControl: string
  toLogger: string
  firstSeen: string
  resumed: string
}): void {
  assert.equal(seen.toControl, 'battery.low')
  assert.equal(seen.toLogger, 'battery.low')
  assert.equal(seen.firstSeen, 'link.up')
  assert.equal(seen.resumed, '3')
}
