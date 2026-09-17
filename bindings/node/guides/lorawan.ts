// The LoRaWAN activation guide example; see docs/guides/lorawan.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { device, grantAccept, grantSession } from '@pamoja/lorawan'

// The root key is provisioned into the device at the factory and known to the network
// server. It is the only secret either side starts with; any 16 bytes stand in here.
const appKey = Buffer.alloc(16, 7)

// The device asks to join with a nonce it has not used before, which is what stops an old
// accept being replayed at it.
const devNonce = 1
const node = device(Buffer.alloc(8), Buffer.alloc(8), appKey)

// The network grants the join. It draws its own nonce, names the network the device is
// joining, and assigns the address the device will answer to from then on.
const devAddr = 0x26012e43
const offer = { appNonce: 2, netId: 19, devAddr }
const accept = grantAccept(offer, appKey, devNonce)
console.log(`granted   address 0x${devAddr.toString(16).toUpperCase()} in a ${accept.length}-byte accept`)

// The device verifies it against the root key. A join accept carries no device identifier,
// so only that key decides whether it is for this device.
const joined = node.acceptJoin(accept, devNonce)
console.log(`joined    the device took address 0x${joined.devAddr.toString(16).toUpperCase()}`)

// Neither side transmits a session key. Both derive the same pair from the root key and the
// two nonces, so the network reads what the device sends without ever having been told how.
const network = grantSession(offer, appKey, devNonce)
const uplink = joined.session().encodeUplink(1, 1, Buffer.from('level=high'))
const received = network.decode(uplink, 1)
console.log(`uplink    the network read ${received.payload.toString()}`)

// A single byte changed in the air fails that check, so no one else can admit the device or
// put words in its mouth.
const forged = Buffer.from(accept)
forged[1] ^= 0xff
try {
  node.acceptJoin(forged, devNonce)
  console.log('a forged accept was taken, which should never happen')
} catch (error) {
  console.log(`forged    accept refused: ${(error as Error).message}`)
}
// ANCHOR_END: example

assert.equal(joined.devAddr, devAddr)
assert.equal(received.payload.toString(), 'level=high')

// ANCHOR: device
import { LoraRegion, planFor } from '@pamoja/lora'
import * as lorawan from '@pamoja/lorawan'

const rootKey = Buffer.alloc(16, 7)
const devEui = Buffer.from('70b3d57ed0051234', 'hex')
const joinEui = Buffer.alloc(8)

// The device owns no radio and no clock. It takes the time in microseconds and says what to
// put on the air, so the same code runs over an SX1276, an SX1262, or nothing at all. This
// one's radio puts out 2 to 20 dBm, and its seed would come from the radio's noise.
const plan = planFor(LoraRegion.Us915)
const settings = { minOutputDbm: 2, maxOutputDbm: 20, seed: 1 }
const mhz = (hz: number) => (hz / 1e6).toFixed(1)
const sensor = lorawan.EndDevice.overTheAir(plan, devEui, joinEui, rootKey, settings)

// A US915 device joins in passes over the band, one 125 kHz channel from each group of eight.
// The accept is due on the downlink channel that join channel is answered on.
const request = sensor.join(1, 0)
console.log(
  `join      ${mhz(request.frequencyHz)} MHz at DR${request.dataRate}, ${request.outputDbm} dBm, ` +
    `${Math.floor(request.airtimeUs / 1000)} ms on air; ` +
    `the accept is due ${request.rx1.delayUs / 1_000_000} s later on ${mhz(request.rx1.frequencyHz)} MHz`,
)

// The network answers, and the device takes its address and session from the accept.
const grant = { appNonce: 2, netId: 19, devAddr: 0x26012e43 }
const heard = sensor.heard(lorawan.grantAccept(grant, rootKey, 1), 7)
if (heard.kind === 'Joined') {
  console.log(`joined    as 0x${heard.devAddr.toString(16).toUpperCase().padStart(8, '0')}`)
}

// A confirmed reading. While it waits on its windows, the device refuses to send another.
const reading = sensor.send(2, '21.5', 10_000_000, true)
console.log(
  `uplink    ${mhz(reading.frequencyHz)} MHz at DR${reading.dataRate}; ` +
    `the answer is due ${reading.rx1.delayUs / 1_000_000} s later on ${mhz(reading.rx1.frequencyHz)} MHz`,
)
try {
  sensor.send(2, '21.6', 10_000_000)
} catch (error) {
  if (lorawan.isDeviceError(error, 'Busy')) {
    console.log('busy      the reading before still waits on its windows')
  }
}

// The network acknowledges it and sends a setting back on the same port.
const answer = lorawan.grantSession(grant, rootKey, 1).encodeDownlink(0, 2, Buffer.from('set=19.0'), { ack: true })
const downlink = sensor.heard(answer, 7)
if (downlink.kind === 'Data') {
  const { acknowledged, port, payload } = downlink.delivery
  console.log(`downlink  acknowledged: ${acknowledged}, port ${port ?? 0} says ${payload.toString()}`)
}

// Before sleeping, the device saves what it settled with the network. After the power cut a
// fresh device resumes it on a clock that starts over, and sends its next reading with no join.
const saved = sensor.save(12_000_000)
const woken = lorawan.EndDevice.overTheAir(plan, devEui, joinEui, rootKey, settings)
woken.resume(saved, 0)
const next = woken.send(2, '21.7', 5_000_000)
console.log(
  `resumed   ${saved.length} saved bytes; ` +
    `the next reading goes out as uplink ${woken.fcntUp - 1} without joining again`,
)
// ANCHOR_END: device

assert.equal(next.carriesPayload, true)
assert.equal(woken.devAddr, 0x26012e43)
