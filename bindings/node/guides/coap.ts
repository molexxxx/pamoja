// The CoAP guide example; see docs/guides/coap.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { CoapClient, Reliability } from '@pamoja/coap'

async function main() {
  // CoAP runs over UDP and opens no session, so connecting only binds a local socket.
  // Nothing is listening on the far side here, and nothing needs to be.
  const reporter = new CoapClient({
    host: '127.0.0.1',
    port: 5683,
    reliability: Reliability.NonConfirmable,
  })
  assert.equal(await reporter.isConnected(), false)
  await reporter.connect()
  assert.equal(await reporter.isConnected(), true)

  // Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which is
  // what a battery-powered node sends when one missed reading costs nothing.
  await reporter.send('sensors/1/temperature', Buffer.from('21.5'))

  // Confirmable delivery retransmits until an ACK arrives. RFC 7252 fixes the defaults at a
  // two-second wait and four retransmissions; both are cut short here.
  const commander = new CoapClient({
    host: '127.0.0.1',
    port: 5683,
    reliability: Reliability.Confirmable,
    ackTimeoutMs: 20,
    maxRetransmits: 1,
  })
  await commander.connect()
  await assert.rejects(() => commander.send('actuators/valve', Buffer.from('open')))

  await reporter.disconnect()
  assert.equal(await reporter.isConnected(), false)
}

main()
// ANCHOR_END: example
