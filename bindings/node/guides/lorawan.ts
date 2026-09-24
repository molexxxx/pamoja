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

// The network acknowledges it in the first window and sends a setting back on the same
// port. Naming the window holds the frame to the length that window's data rate carries.
const answer = lorawan.grantSession(grant, rootKey, 1).encodeDownlink(0, 2, Buffer.from('set=19.0'), { ack: true })
const downlink = sensor.heard(answer, 7, lorawan.ReceiveWindow.Rx1)
if (downlink.kind === 'Data') {
  const { acknowledged, port, payload } = downlink.delivery
  const readingWas = acknowledged ? 'acknowledged' : 'not acknowledged'
  console.log(`downlink  the reading was ${readingWas}, and port ${port ?? 0} says ${payload.toString()}`)
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

/** How many milliseconds a relay's scan period lasts, as its acknowledgment names it. */
function periodMs(periodicity: lorawan.CadPeriodicity): number {
  return { Ms1000: 1000, Ms500: 500, Ms250: 250, Ms100: 100, Ms50: 50, Ms20: 20 }[periodicity]
}

// ANCHOR: relay
import { Network } from '@pamoja/gateway'
import { CadPeriodicity, CadToRx, Relay, XtalAccuracy } from '@pamoja/lorawan'

// One site: a gateway on a hill, a relay on a rooftop in range of it, and a sensor in a
// cellar the gateway cannot hear at all.
const relayEui = Buffer.from('70b3d57ed0050001', 'hex')
const sensorEui = Buffer.from('70b3d57ed0050002', 'hex')
const band = planFor(LoraRegion.Eu868)
const radio = { minOutputDbm: 2, maxOutputDbm: 14, lowestHz: 863_000_000, highestHz: 870_000_000, seed: 1 }
const site = new Network(band, 0x00002a, null, 0x26010001)
site.register(relayEui, joinEui, rootKey)
site.register(sensorEui, joinEui, rootKey)
const heardAt = (transmission: lorawan.Transmission, atUs: number) => ({
  frequencyHz: transmission.frequencyHz,
  payload: transmission.frame,
  link: transmission.link,
  timestampUs: atUs,
})
const address = (value: number) => `0x${value.toString(16).toUpperCase().padStart(8, '0')}`

// The relay is an end device that also listens for others, so it joins the ordinary way.
const rooftop = Relay.overTheAir(band, relayEui, joinEui, rootKey, radio, XtalAccuracy.Ppm20, CadToRx.Symbols4)
const relayJoin = rooftop.join(1, 1_000_000)
const relayAccept = site.uplink(heardAt(relayJoin, 1_000_000))
rooftop.heardIn(lorawan.ReceiveWindow.Rx1, relayAccept.accept!.payload, 7)
console.log(`relay     joined as ${address(rooftop.devAddr!)}`)

// The sensor joins too. Its own uplinks never reach the gateway, but its join does, because
// the cellar door is open while it is installed.
const cellar = lorawan.EndDevice.overTheAir(band, sensorEui, joinEui, rootKey, radio)
const sensorJoin = cellar.join(2, 20_000_000)
const sensorAccept = site.uplink(heardAt(sensorJoin, 20_000_000))
cellar.heard(sensorAccept.accept!.payload, 7, lorawan.ReceiveWindow.Rx1)
const sensorAddr = cellar.devAddr!

// The network hands the relay the key that lets it verify the sensor's wake-up frames, in a
// command riding on the relay's own downlink.
const relayEmpty = rooftop.sendEmpty(40_000_000)
const relayCarried = site.uplink(heardAt(relayEmpty, 40_000_000))
const trust = site.trustCommand(sensorAddr, 0, 63, 0)
const configure = site.command(rooftop.devAddr!, relayCarried.slot!, [trust])
rooftop.heardIn(lorawan.ReceiveWindow.Rx1, configure.payload, 7)
console.log(`trusted   the relay now forwards for ${address(sensorAddr)}`)

// It scans once a second on the region's wake-on-radio channel.
rooftop.start(CadPeriodicity.Ms1000, 0)
const scan = rooftop.nextScan(60_000_000)!
console.log(`scan      ${mhz(scan.carrier.frequencyHz)} MHz at DR${scan.carrier.dataRate} every second`)

// The sensor turns relay mode on. Its uplink now goes out behind a frame whose preamble spans
// a whole scan period, because it does not yet know when the relay listens.
cellar.useRelay(true)
const relayedReading = cellar.send(2, '21.5', 61_000_000)
const exchange = relayedReading.relay!
console.log(
  `wake      ${exchange.wakeUp.frame.length} bytes with a ${exchange.wakeUp.link.preambleSymbols}-symbol ` +
    `preamble, ${Math.trunc((exchange.uplinkStartUs - exchange.wakeUp.startUs) / 1000)} ms before the uplink`,
)

// The relay hears it, knows the device, and answers with when it scanned, so every frame after
// this one carries only the preamble the two clocks could have drifted apart.
const woke = rooftop.heardWor(scan, exchange.wakeUp.frame, -90, 4, scan.startUs + 500_000)
if (woke.kind !== 'Uplink') {
  throw new Error('the relay knows this device')
}
const said = cellar.heardWorAck(woke.acknowledgment!.frame)
console.log(`ack       the relay scans every ${periodMs(said.cadPeriodicity)} ms and forwards at DR${said.relayDataRate}`)

// The uplink follows, and the relay wraps it in one of its own on port 226.
const dueUs = rooftop.heardUplink(relayedReading.frame, -88, 6, woke.listen!.startUs + 100_000)
const forwarded = rooftop.forward(dueUs)
const relayed = site.uplink(heardAt(forwarded, dueUs))
console.log(
  `forwarded ${relayed.payload!.toString()} from ${address(relayed.devAddr)}, ` +
    `heard by ${address(relayed.relay!.relay)} at ${relayed.relay!.rssiDbm} dBm`,
)

// The answer goes back the same way: the network answers the sensor, the relay unwraps it and
// sends it on, and the sensor hears it in the window it keeps for a relay.
const relayedAnswer = site.answer(sensorAddr, relayed.slot!, 2, Buffer.from('set=19.0'))
const passed = rooftop.heardIn(lorawan.ReceiveWindow.Rx1, relayedAnswer.payload, 7)
if (passed.kind !== 'Downlink') {
  throw new Error('a downlink for the sensor')
}
const delivered = cellar.heard(passed.downlink.frame, 7, lorawan.ReceiveWindow.Rxr)
if (delivered.kind === 'Data') {
  console.log(
    `downlink  port ${delivered.delivery.port ?? 0} says ${delivered.delivery.payload.toString()}, ` +
      `${Math.trunc(exchange.rxr.delayUs / 1_000_000)} s after the uplink`,
  )
}
// ANCHOR_END: relay
