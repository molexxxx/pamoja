// The engine surface guide example; see docs/guides/transport.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { Transport } from '@pamoja/core'
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

async function main() {
  // Whatever a link is underneath, MQTT, CoAP, or the in-process broker below, it reaches
  // the rest of the framework as one Transport. Anything that takes a link takes that, so
  // a node is written once and pointed at whichever link it has.
  const broker = new LoopbackBroker()
  const gateway = broker.link()
  await gateway.connect()
  await gateway.subscribe('sensors/1/temperature')

  // The fault injector is a Transport wrapping a Transport, so it composes anywhere a link
  // does. This one fails its next send and passes everything after through.
  const flaky = Transport.faulty(broker.rung(), 1)
  assert.equal(flaky.isAvailable, true)

  // Composing consumes the transport, because whatever it was composed into owns it from
  // here. The handle is emptied rather than left aliasing what now belongs to something
  // else, so it cannot be sent on twice.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(flaky)
  assert.equal(flaky.isAvailable, false)
  await ladder.connect()

  // The injected failure lands, so the reading is buffered rather than lost.
  assert.equal(await ladder.send('sensors/1/temperature', Buffer.from('20.1')), Delivery.Buffered)
  assert.equal(await ladder.buffered(), 1)

  // The next reading joins the back of the queue instead of overtaking it, even though the
  // link would take it now. Order on the wire is the order the readings were taken.
  assert.equal(await ladder.send('sensors/1/temperature', Buffer.from('20.4')), Delivery.Buffered)
  assert.equal(await ladder.buffered(), 2)

  // Flushing forwards the backlog oldest first, and the subscriber sees it in order.
  assert.equal(await ladder.flush(), 2)
  assert.equal(await ladder.buffered(), 0)
  assert.equal((await gateway.recv())?.payload.toString(), '20.1')
  assert.equal((await gateway.recv())?.payload.toString(), '20.4')
}

main()
// ANCHOR_END: example
