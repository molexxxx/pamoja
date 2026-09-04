// The event bus guide example; see docs/guides/bus.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { EventBus } from '@pamoja/bus'

async function main(): Promise<void> {
  // A sampler announces a reading and whatever cares about readings picks it up, with
  // neither side holding a reference to the other.
  const hub = new EventBus(8)
  const sampler = await hub.subscribe()
  const logger = await hub.subscribe()

  await hub.publish(Buffer.from('battery.low'))
  assert.equal((await sampler.next())!.toString(), 'battery.low')
  assert.equal((await logger.next())!.toString(), 'battery.low')

  // An endpoint taken later starts from the next event, so it never sees what went out
  // before it existed.
  const late = await hub.subscribe()
  await hub.publish(Buffer.from('link.up'))
  assert.equal((await late.next())!.toString(), 'link.up')
  assert.equal((await sampler.next())!.toString(), 'link.up')

  // The buffer is per endpoint and bounded, so an endpoint further behind than the
  // capacity drops what it missed and resumes with the most recent events.
  const slow = new EventBus(2)
  const reader = await slow.subscribe()
  for (let count = 0; count < 5; count += 1) {
    await slow.publish(Buffer.from([count]))
  }
  assert.equal((await reader.next())![0], 3)
  assert.equal((await reader.next())![0], 4)
}

main()
// ANCHOR_END: example
