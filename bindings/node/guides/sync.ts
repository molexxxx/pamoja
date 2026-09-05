// The store-and-forward guide example; see docs/guides/sync.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Store } from '@pamoja/sync'

async function main() {
  // A node with nowhere to send buffers its readings. This queue is held in memory, so it
  // lasts as long as the process; Store.file(dir) is the same queue on disk, which is what
  // a node uses to survive a reboot with its backlog intact.
  const outbox = Store.memory()
  for (const reading of ['20.1', '20.4', '20.2']) {
    await outbox.append(Buffer.from(reading))
  }
  console.log(`queued    ${await outbox.len()} readings with no link`)

  // Peek reads the oldest record without taking it, so a send that fails part-way leaves
  // the queue exactly as it was.
  const oldest = (await outbox.peek())!
  console.log(`oldest    ${oldest.toString()} and still ${await outbox.len()} held`)

  // The link returns and the queue drains oldest first, in the order the readings were
  // taken rather than the order they happen to come back off a buffer.
  const drained: string[] = []
  for (let record = await outbox.pop(); record !== null; record = await outbox.pop()) {
    drained.push(record.toString())
  }
  console.log(`drained   ${drained.join(', ')}`)

  // A bounded queue refuses the append that would overflow it. A full store is
  // backpressure the caller is told about, not a reading dropped behind its back.
  const bounded = Store.memory(2)
  await bounded.append(Buffer.from('20.1'))
  await bounded.append(Buffer.from('20.4'))
  try {
    await bounded.append(Buffer.from('20.2'))
    console.log('a full queue took a third reading, which should never happen')
  } catch (error) {
    console.log(`full      refused the third reading: ${(error as Error).message}`)
  }

  return { oldest, drained, left: await outbox.len(), held: await bounded.len() }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: { oldest: Buffer; drained: string[]; left: number; held: number }): void {
  assert.equal(seen.oldest.toString(), '20.1')
  assert.deepEqual(seen.drained, ['20.1', '20.4', '20.2'])
  assert.equal(seen.left, 0)
  assert.equal(seen.held, 2)
}
