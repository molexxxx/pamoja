// The store-and-forward guide example; see docs/guides/sync.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { Store } from '@pamoja/sync'

async function main() {
  // A node with nowhere to send buffers its readings. This queue is held in memory, so
  // it lasts as long as the process; Store.file(dir) is the same queue on disk.
  const outbox = Store.memory()
  for (const reading of ['20.1', '20.4', '20.2']) {
    await outbox.append(Buffer.from(reading))
  }
  assert.equal(await outbox.len(), 3)

  // Peek reads the oldest record without taking it, so a send that fails part-way leaves
  // the queue exactly as it was.
  assert.equal((await outbox.peek())?.toString(), '20.1')
  assert.equal(await outbox.len(), 3)

  // The link returns and the queue drains oldest first, in the order the readings were
  // taken rather than the order they happen to come back off a buffer.
  const drained: string[] = []
  let record = await outbox.pop()
  while (record !== null) {
    drained.push(record.toString())
    record = await outbox.pop()
  }
  assert.deepEqual(drained, ['20.1', '20.4', '20.2'])
  assert.equal(await outbox.len(), 0)

  // A bounded queue refuses the append that would overflow it. A full store is
  // backpressure the caller is told about, not a reading dropped behind its back.
  const bounded = Store.memory(2)
  await bounded.append(Buffer.from('20.1'))
  await bounded.append(Buffer.from('20.4'))
  await assert.rejects(() => bounded.append(Buffer.from('20.2')))
  assert.equal(await bounded.len(), 2)
}

main()
// ANCHOR_END: example
