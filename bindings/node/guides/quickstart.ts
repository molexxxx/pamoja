// The first example on the README and the site: a reading taken off a wire on a field
// node, sent over a link, and checked on the gateway that receives it, with nothing
// plugged in and nothing running.

import assert from 'node:assert/strict'

// ANCHOR: example
import { packSamples, unpackSamples } from '@pamoja/codec'
import { Smoother } from '@pamoja/kit'
import { LoopbackBroker } from '@pamoja/loopback'
import { DeviceIdentity, fingerprint, verifyMessage } from '@pamoja/security'
import { ds18b20 } from '@pamoja/sensors'

// The device's identity is provisioned once and never leaves it. The gateway is told only
// the public half, which is how it recognises this device later.
const SEED = Buffer.alloc(32, 7)
const TOPIC = 'sensors/1/temperature'

async function main(): Promise<Buffer> {
  // The link. A loopback broker stands in for MQTT or CoAP, so this runs with no network
  // and nothing listening. Point the node at a real transport and nothing below changes.
  const broker = new LoopbackBroker()
  const node = broker.link()
  const gateway = broker.link()
  await node.connect()
  await gateway.connect()
  await gateway.subscribe(TOPIC)

  const device = DeviceIdentity.fromSeed(SEED)
  const known = device.publicKey()
  console.log(`gateway trusts device ${fingerprint(known)}`)

  // A stand-in for the thermometer. On a running node these nine bytes arrive from the
  // 1-Wire bus; here the library builds what a part at 25.0625 C would send.
  const offTheBus = ds18b20.buildScratchpad(25.0625, 12, 75, -10)

  // On the node. The part checksums every read, so a value mangled on a long run is an
  // error rather than a plausible temperature a couple of degrees off.
  const celsius = ds18b20.parseScratchpad(offTheBus).microCelsius / 1e6
  console.log(`read      ${celsius.toFixed(4)} C`)

  // Readings jitter, so smooth them, and send a batch rather than one at a time.
  // Successive readings differ by very little, so the differences cost a fraction of what
  // the readings would on a link that charges by the byte.
  const smoother = new Smoother(0.5)
  const batch = [celsius, celsius + 0.5, celsius + 0.4].map((sample) =>
    Math.round(smoother.update(sample) * 100),
  )
  const packed = packSamples(batch)
  console.log(`packed    ${batch.length} readings into ${packed.length} bytes`)

  // Sign the batch and send it. The signature travels with the payload as one message, so
  // there is nothing to keep together and split correctly at the far end.
  await node.send(TOPIC, device.signMessage(packed))

  // On the gateway. Verifying returns the payload, so a reading that was altered on the
  // way, or signed by some other device, never reaches the code that unpacks it.
  const received = await gateway.recv()
  const payload = verifyMessage(known, received!.payload)
  if (payload === null) {
    console.log('gateway   rejected the reading')
  } else {
    console.log(`gateway   accepted ${unpackSamples(payload).join(', ')} in hundredths of a degree`)
  }

  return received!.payload
}

main()
// ANCHOR_END: example
  .then(check)

function check(message: Buffer): void {
  const knownKey = DeviceIdentity.fromSeed(SEED).publicKey()
  assert.deepEqual(unpackSamples(verifyMessage(knownKey, message)!), [2506, 2531, 2539])
  assert.ok(message.length < 64 + 3 * 8)

  // A message edited in transit does not verify, so the gateway never unpacks it.
  const edited = Buffer.from(message)
  edited[edited.length - 1] ^= 0xff
  assert.equal(verifyMessage(knownKey, edited), null)
}
