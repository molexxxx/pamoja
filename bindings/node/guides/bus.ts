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

  await hub.publish(Buffer.from('battery.low'))
  const toControl = (await control.next())!
  const toLogger = (await logger.next())!
  console.log(`control saw ${toControl.toString()}, the logger saw ${toLogger.toString()}`)

  // A subscriber taken later starts from the next event, so it never sees what went out
  // before it existed.
  const late = await hub.subscribe()
  await hub.publish(Buffer.from('link.up'))
  const firstSeen = (await late.next())!
  console.log(`the late subscriber's first event is ${firstSeen.toString()}`)

  // The buffer is per subscriber and bounded, so one further behind than the capacity
  // drops what it missed and resumes with the most recent events. A slow reader costs
  // itself, not the publisher.
  const slow = new EventBus(2)
  const reader = await slow.subscribe()
  for (let count = 0; count < 5; count += 1) {
    await slow.publish(Buffer.from([count]))
  }
  const resumed = (await reader.next())!
  console.log(`after five events into a buffer of two, the reader resumes at ${resumed[0]}`)

  return { toControl, toLogger, firstSeen, resumed }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: {
  toControl: Buffer
  toLogger: Buffer
  firstSeen: Buffer
  resumed: Buffer
}): void {
  assert.equal(seen.toControl.toString(), 'battery.low')
  assert.equal(seen.toLogger.toString(), 'battery.low')
  assert.equal(seen.firstSeen.toString(), 'link.up')
  assert.deepEqual([...seen.resumed], [3])
}
