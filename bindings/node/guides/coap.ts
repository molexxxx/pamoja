// The CoAP guide example; see docs/guides/coap.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { CoapClient, CoapServer, Reliability } from '@pamoja/coap'

async function main() {
  // The gateway in the orchard's shed. It takes moisture readings from every row, and holds
  // the irrigation valve's state for the rows to observe. Port 0 lets the system pick a free
  // port, which the rows are pointed at below.
  const gateway = new CoapServer('127.0.0.1:0')
  await gateway.connect()
  await gateway.subscribe('orchard/+/moisture')
  await gateway.send('orchard/valve', 'closed')
  const port = gateway.localPort!
  console.log('gateway   takes moisture readings on orchard/+/moisture')

  // A battery-powered sensor in row 7. Its reading is confirmable, so it waits for the
  // gateway's acknowledgment and retransmits until one comes back.
  const row7 = new CoapClient({ host: '127.0.0.1', port, ackTimeoutMs: 200 })
  await row7.connect()
  await row7.send('orchard/row-7/moisture', '31')
  console.log('row-7     reported 31, and the gateway acknowledged it')
  const reading = (await gateway.recv())!
  console.log(`gateway   took ${reading.text!} from ${reading.topic}`)

  // Observing the valve registers the row with the gateway, which answers with the valve's
  // state now and notifies every change after it, as RFC 7641 describes.
  await row7.subscribe('orchard/valve')
  const current = (await row7.recv())!
  console.log(`row-7     observes ${current.topic}, which reads ${current.text!}`)
  await gateway.send('orchard/valve', 'open')
  const observers = gateway.observers('orchard/valve')
  console.log(`gateway   opened the valve for ${observers} observer`)
  const change = (await row7.recv())!
  console.log(`row-7     ${change.topic} now reads ${change.text!}`)

  // A path the gateway does not take is answered 4.04, and a confirmable send reports that
  // rather than counting the reading as delivered.
  try {
    await row7.send('orchard/row-7/battery', '3.1')
    console.log('row-7     the battery reading was taken, which should never happen')
  } catch (error) {
    console.log(`row-7     battery refused: ${(error as Error).message}`)
  }

  // Row 8 sends non-confirmable: once and unacknowledged, which costs the least radio time
  // and suits a reading whose loss costs nothing.
  const row8 = new CoapClient({ host: '127.0.0.1', port, reliability: Reliability.NonConfirmable })
  await row8.connect()
  await row8.send('orchard/row-8/moisture', '27')
  console.log('row-8     sent 27 without waiting for an answer')
  const unconfirmed = (await gateway.recv())!
  console.log(`gateway   took ${unconfirmed.text!} from ${unconfirmed.topic}`)

  // Row 9 is pointed at port 1, where nothing listens. A confirmable send retransmits on a
  // doubling wait and then gives up. RFC 7252's defaults would take more than a minute to
  // get there, so this one waits 20 ms and retransmits once.
  const row9 = new CoapClient({ host: '127.0.0.1', port: 1, ackTimeoutMs: 20, maxRetransmits: 1 })
  await row9.connect()
  try {
    await row9.send('orchard/row-9/moisture', '29')
    console.log('row-9     an empty port acknowledged it, which should never happen')
  } catch (error) {
    console.log(`row-9     gave up unacknowledged: ${(error as Error).message}`)
  }

  for (const link of [row7, row8, row9]) {
    await link.disconnect()
  }
  await gateway.disconnect()
  return { reading, current, change, observers, unconfirmed }
}

main()
// ANCHOR_END: example
  .then(check)

function check({
  reading,
  current,
  change,
  observers,
  unconfirmed,
}: {
  reading: { topic: string }
  current: { text?: string }
  change: { text?: string }
  observers: number
  unconfirmed: { topic: string }
}): void {
  assert.equal(reading.topic, 'orchard/row-7/moisture')
  assert.equal(current.text, 'closed')
  assert.equal(change.text, 'open')
  assert.equal(observers, 1)
  assert.equal(unconfirmed.topic, 'orchard/row-8/moisture')
}
