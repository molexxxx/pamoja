// The own-link guide example; see docs/guides/link.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Transport, type DeliveredMessage, type TransportHandlers } from '@pamoja/core'
import { Ladder } from '@pamoja/ladder'
import { Store } from '@pamoja/sync'

// A cellular modem reached through its vendor's SDK, which this class stands in for.
// Nothing in it names a broker or a protocol: it needs only the operations the
// contract asks for, and `recv` is what makes it a link that delivers.
class Modem implements TransportHandlers {
  sent: { topic: string; text: string }[] = []
  filters: string[] = []
  noSignal = false
  private inbox: (DeliveredMessage | Error)[] = []
  private waiting: ((next: DeliveredMessage | Error) => void)[] = []

  connect(): void {}

  send(topic: string, payload: Buffer): void {
    if (this.noSignal) throw new Error('no signal')
    this.sent.push({ topic, text: payload.toString() })
  }

  subscribe(topic: string): void {
    this.filters.push(topic)
  }

  async recv(): Promise<DeliveredMessage> {
    const next =
      this.inbox.shift() ?? (await new Promise<DeliveredMessage | Error>((hand) => this.waiting.push(hand)))
    if (next instanceof Error) throw next
    return next
  }

  // The vendor's side: a message from the network, or the loss of the session,
  // handed to the wait that is open or kept for the next one.
  handOver(next: DeliveredMessage | Error): void {
    const waiter = this.waiting.shift()
    if (waiter) waiter(next)
    else this.inbox.push(next)
  }
}

// A satellite messenger sends and never receives. With no `recv` it is an uplink,
// which a ladder never listens on or subscribes.
class Satellite implements TransportHandlers {
  sent: { topic: string; text: string }[] = []

  connect(): void {}

  send(topic: string, payload: Buffer): void {
    this.sent.push({ topic, text: payload.toString() })
  }

  subscribe(): void {
    throw new Error('a satellite messenger only sends')
  }
}

async function main() {
  // The modem is the cheaper link and goes first. The messenger has no `recv`, so it
  // goes on as an uplink.
  const modem = new Modem()
  const satellite = new Satellite()
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.fromHandlers(modem))
  await ladder.rung(Transport.fromHandlers(satellite))
  await ladder.connect()

  // A reading goes out over the first link that takes it.
  await ladder.send('waves/height', '1.8')
  const carried = modem.sent[0]
  console.log(`modem     carried ${carried.topic} ${carried.text}`)

  // A subscription reaches every link that listens, and only those: the messenger,
  // whose subscribe would throw, is never asked.
  await ladder.subscribe('commands/#')
  const filter = modem.filters[0]
  console.log(`modem     listens on ${filter}, and the satellite was never asked`)

  // What the modem hands over comes back through the ladder.
  modem.handOver({ topic: 'commands/interval', payload: '600' })
  const command = (await ladder.recv())!
  console.log(`buoy      took ${command.topic} ${command.text!}`)

  // A link that refuses a send passes the reading down to the next link.
  modem.noSignal = true
  await ladder.send('waves/height', '2.4')
  const relayed = satellite.sent[0]
  console.log(`satellite carried ${relayed.topic} ${relayed.text} while the modem had no signal`)

  // A link that fails while listening says why, once, and has ended after that. With
  // no other link listening, the ladder then has nothing to wait on.
  modem.handOver(new Error('the modem lost its session'))
  let lost = ''
  try {
    await ladder.recv()
  } catch (error) {
    lost = (error as Error).message
  }
  console.log(`buoy      lost the modem: ${lost}`)
  let idle = ''
  try {
    await ladder.recv()
  } catch (error) {
    idle = (error as Error).message
  }
  console.log(`buoy      has no link left to listen on: ${idle}`)

  // Connecting again brings the modem back, and the ladder places its filter on it
  // again, so a link's subscribe runs once for every connect.
  await ladder.connect()
  console.log(`modem     reconnected, and the ladder placed ${modem.filters[1]} on it again`)
  modem.handOver({ topic: 'commands/interval', payload: '900' })
  const later = (await ladder.recv())!
  console.log(`buoy      took ${later.topic} ${later.text!}`)

  return { carried, filters: modem.filters, command, relayed, lost, idle, later }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: {
  carried: { topic: string; text: string }
  filters: string[]
  command: { topic: string; text?: string }
  relayed: { topic: string; text: string }
  lost: string
  idle: string
  later: { topic: string; text?: string }
}): void {
  assert.deepEqual(seen.carried, { topic: 'waves/height', text: '1.8' })
  assert.deepEqual(seen.filters, ['commands/#', 'commands/#'])
  assert.equal(seen.command.text, '600')
  assert.deepEqual(seen.relayed, { topic: 'waves/height', text: '2.4' })
  assert.equal(seen.lost, 'transport error: the modem lost its session')
  assert.equal(seen.idle, 'resource is closed')
  assert.equal(seen.later.text, '900')
}
