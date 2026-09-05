// The CoAP guide example; see docs/guides/coap.md.

// ANCHOR: example
import { CoapClient, Reliability } from '@pamoja/coap'

async function main(): Promise<void> {
  // CoAP runs over UDP and opens no session, so connecting only binds a local socket.
  // Nothing is listening on the far side here, and for a non-confirmable send nothing
  // needs to be.
  const reporter = new CoapClient({
    host: '127.0.0.1',
    port: 5683,
    reliability: Reliability.NonConfirmable,
  })
  await reporter.connect()
  console.log(`reporter  connected: ${await reporter.isConnected()}`)

  // Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which is
  // what a battery-powered node sends when one missed reading costs nothing.
  await reporter.send('sensors/1/temperature', Buffer.from('21.5'))
  console.log('reporter  sent 21.5 and did not wait for an answer')

  // A command is different: it has to arrive. Confirmable delivery retransmits until an
  // acknowledgement comes back. RFC 7252 fixes the defaults at a two-second wait and four
  // retransmissions; both are cut short here so the guide does not sit waiting.
  const commander = new CoapClient({
    host: '127.0.0.1',
    port: 5683,
    reliability: Reliability.Confirmable,
    ackTimeoutMs: 20,
    maxRetransmits: 1,
  })
  await commander.connect()
  try {
    await commander.send('actuators/valve', Buffer.from('open'))
    console.log('commander the valve acknowledged the command')
  } catch (error) {
    console.log(`commander gave up unacknowledged: ${(error as Error).message}`)
  }

  await reporter.disconnect()
  console.log(`reporter  disconnected: ${!(await reporter.isConnected())}`)
}

main()
// ANCHOR_END: example
