// The store-and-forward guide example; see docs/guides/sync.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { Transport } from '@pamoja/core'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

const TOPIC = 'apiary/hive-3/weight'

async function main() {
  // The scale logs its weight to a queue on its SD card, bounded so a long outage cannot
  // fill the card. The directory is the queue, so the scale can lose power at any moment
  // and lose nothing it logged.
  const dir = mkdtempSync(join(tmpdir(), 'pamoja-hive-'))
  let outbox = Store.file(dir, 3)
  for (const weight of ['41.2', '41.5', '40.9']) {
    await outbox.append(weight)
  }
  const logged = await outbox.len()
  console.log(`hive      logged ${logged} weights with no link, the most its store holds`)

  // A full store refuses the next weight rather than dropping one it already holds.
  try {
    await outbox.append('41.1')
  } catch (error) {
    console.log(`hive      was refused a 4th: ${(error as Error).message}`)
  }

  // The scale reboots. Its queue is the directory, so it comes back whole and in order.
  outbox = Store.file(dir, 3)
  const held = await outbox.len()
  const oldest = (await outbox.peekText())!
  console.log(`hive      restarted and still holds ${held}, oldest first: ${oldest}`)

  // The cellular uplink carries one weight, then drops. A weight leaves the queue only
  // once a link has taken it, so what the uplink never took stays, in order.
  const cellular = new LoopbackBroker()
  const uplink = Transport.degraded(cellular.rung(), { up: 1, down: 10 })
  await uplink.connect()
  let forwarded = 0
  try {
    await outbox.drainTo(uplink, TOPIC)
  } catch (error) {
    forwarded = held - (await outbox.len())
    console.log(`uplink    forwarded ${forwarded}, then failed: ${(error as Error).message}`)
  }
  const left = await outbox.len()
  const next = (await outbox.peekText())!
  console.log(`hive      still holds ${left}, oldest first: ${next}`)

  // The beekeeper's gateway comes within reach, and the scale drains the rest onto it.
  const visit = new LoopbackBroker()
  const gateway = visit.link()
  await gateway.connect()
  await gateway.subscribe(TOPIC)
  const toGateway = visit.rung()
  await toGateway.connect()
  await outbox.drainTo(toGateway, TOPIC)
  const took: string[] = []
  for (let weight = 0; weight < left; weight += 1) {
    took.push((await gateway.recv())!.text!)
  }
  console.log(`gateway   took ${took.join(', ')} when the beekeeper came by`)
  const empty = await outbox.len()
  console.log(`hive      holds ${empty} once the backlog is through`)

  rmSync(dir, { recursive: true })
  return { counts: [logged, held, forwarded, left, empty], oldest, next, took }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: { counts: number[]; oldest: string; next: string; took: string[] }): void {
  assert.deepEqual(seen.counts, [3, 3, 1, 2, 0])
  assert.equal(seen.oldest, '41.2')
  assert.equal(seen.next, '41.5')
  assert.deepEqual(seen.took, ['41.5', '40.9'])
}
