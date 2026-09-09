// The own-link guide example; see docs/guides/link.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Transport, type TransportHandlers, type TransportMessage } from '@pamoja/core'
import { Ladder } from '@pamoja/ladder'
import { Store } from '@pamoja/sync'

// A link over two queues, standing in for a radio or cloud SDK. Nothing about it
// names a broker: it needs only the operations the contract asks for, and `recv` is
// what makes it a link that delivers rather than an uplink.
class QueueLink implements TransportHandlers {
  sent: TransportMessage[] = []
  filters: string[] = []
  private inbox: TransportMessage[] = []
  private waiting: ((message: TransportMessage) => void)[] = []

  async connect(): Promise<void> {}

  async send(topic: string, payload: Buffer): Promise<void> {
    this.sent.push({ topic, payload })
  }

  async subscribe(topic: string): Promise<void> {
    this.filters.push(topic)
  }

  recv(): Promise<TransportMessage> {
    const next = this.inbox.shift()
    return next ? Promise.resolve(next) : new Promise((resolve) => this.waiting.push(resolve))
  }

  // The vendor side: a message arriving from the radio.
  deliver(message: TransportMessage): void {
    const waiter = this.waiting.shift()
    if (waiter) waiter(message)
    else this.inbox.push(message)
  }
}

async function main() {
  // The link is a rung like any shipped transport, and the ladder is the link a node
  // is written against.
  const link = new QueueLink()
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.fromHandlers(link))
  await ladder.connect()

  // A reading out through the ladder lands in the link, topic and bytes intact.
  await ladder.send('sensors/1', Buffer.from('21.5'))
  const carried = link.sent[0]
  console.log(`link carried: ${carried.topic} ${carried.payload.toString()}`)

  // A subscription placed on the ladder reaches the link.
  await ladder.subscribe('commands/#')
  const filter = link.filters[0]
  console.log(`link subscribed to: ${filter}`)

  // What the link delivers comes back through the ladder.
  link.deliver({ topic: 'commands/1', payload: Buffer.from('open') })
  const command = (await ladder.recv())!
  console.log(`command over the ladder: ${command.topic} ${command.payload.toString()}`)

  return { carried, filter, command }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: {
  carried: TransportMessage
  filter: string
  command: TransportMessage
}): void {
  assert.equal(seen.carried.topic, 'sensors/1')
  assert.equal(seen.carried.payload.toString(), '21.5')
  assert.equal(seen.filter, 'commands/#')
  assert.equal(seen.command.topic, 'commands/1')
  assert.equal(seen.command.payload.toString(), 'open')
}
