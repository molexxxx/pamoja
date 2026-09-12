// The LoRaWAN gateway guide example; see docs/guides/gateway.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { PacketKind, TxStatus, acknowledgment, encode, parse } from '@pamoja/gateway'
import { link } from '@pamoja/lora'

// A gateway on a Raspberry Pi, whose identifier is written from its network interface.
const gateway = 'b827ebfffe010203'
const dr5 = link(7, 125_000)

// It heard a packet on 868.1 MHz, and forwards it with the levels it was heard at and the
// concentrator's own timestamp of the reception.
const heard = {
  frequencyHz: 868_100_000,
  payload: Buffer.from('TEST_PACKET_1234'),
  link: dr5,
  rssiDbm: -35,
  snrDb: 5.1,
  timestampUs: 3_512_348_611,
}
const datagram = encode({ kind: PacketKind.PushData, token: 0x1234, gateway, packets: [heard] })
console.log(`push      ${datagram.length} bytes, token ${(0x1234).toString(16)}`)

// The server reads it. Nothing about the packet has to be decoded by hand: the frequency is
// in hertz, the datarate identifier is the link settings, and the payload is bytes.
const received = parse(datagram).packets![0]
console.log(
  `heard     ${received.frequencyHz} Hz at SF${received.link!.spreadingFactor}, ` +
    `${received.link!.bandwidthHz / 1000} kHz, ${received.rssiDbm} dBm, ` +
    `SNR ${received.snrDb} dB, ${received.payload.length} bytes`,
)

// Every uplink is acknowledged at once, by token, before anything is processed.
const ack = acknowledgment(parse(datagram))!
console.log(`ack       ${encode(ack).length} bytes`)

// Later the server sends one back, at the concentrator timestamp that hits the device's
// receive window, with the inverted polarity a LoRaWAN device listens for.
const downlink = encode({
  kind: PacketKind.PullResp,
  token: 0x00ab,
  transmit: {
    frequencyHz: 869_525_000,
    payload: Buffer.from('downlink'),
    link: dr5,
    timestampUs: 3_513_348_611,
    powerDbm: 27,
    invertPolarity: true,
    withoutCrc: true,
  },
})
const transmit = parse(downlink).transmit!
console.log(
  `downlink  ${transmit.frequencyHz} Hz at ${transmit.powerDbm} dBm, ` +
    `inverted IQ ${transmit.invertPolarity}`,
)

// The gateway answers with what became of it. A packet already scheduled in that window is
// refused rather than dropped silently.
const refused = encode({
  kind: PacketKind.TxAck,
  token: 0x00ab,
  gateway,
  txStatus: TxStatus.CollisionPacket,
})
const status = parse(refused).txStatus!
console.log(`txack     ${status}, scheduled ${status === TxStatus.None}`)
// ANCHOR_END: example

assert.equal(received.payload.toString(), 'TEST_PACKET_1234')
assert.deepEqual([...encode(ack)], [2, 0x12, 0x34, 0x01])
assert.equal(status, 'COLLISION_PACKET')
// ANCHOR: network
import { Network } from '@pamoja/gateway'
import { planFor, LoraRegion } from '@pamoja/lora'
import { device, session } from '@pamoja/lorawan'

// One site, on the band it operates in, admitting one device it was told about.
const devEui = Buffer.alloc(8, 0x11)
const appEui = Buffer.alloc(8, 0x22)
const appKey = Buffer.alloc(16, 0x33)
const site = new Network(planFor(LoraRegion.Eu868), 0x00002a, null, 0x26010001)
site.register(devEui, appEui, appKey)

// The gateway forwards a join request it heard. Nothing about the device is known here beyond
// the key it was provisioned with, which is what verifies the request.
const joiner = device(devEui, appEui, appKey)
const joined = site.uplink({
  frequencyHz: 868_100_000,
  payload: joiner.joinRequest(0x0102),
  link: dr5,
  timestampUs: 1_000_000,
})
console.log(
  `joined    0x${joined.devAddr.toString(16).padStart(8, '0')} at ${joined.accept!.timestampUs} us, ` +
    `inverted IQ ${joined.accept!.invertPolarity}`,
)

// The device reads the accept and sends a reading. The site decrypts it and says where an
// answer goes, which is the uplink window plus the delay the region recommends.
const granted = joiner.acceptJoin(joined.accept!.payload, 0x0102)
const carried = site.uplink({
  frequencyHz: 868_100_000,
  payload: granted.session().encodeUplink(0, 2, Buffer.from('21.5')),
  link: dr5,
  timestampUs: 9_000_000,
})
console.log(
  `uplink    frame ${carried.fcnt}, ${carried.payload!.length} bytes, ` +
    `answer at ${carried.slot!.timestampUs} us on ${carried.slot!.frequencyHz} Hz`,
)

// The answer goes out in that window, encrypted with the session the join granted.
const answer = site.answer(carried.devAddr, carried.slot!, 2, Buffer.from('ok'))
console.log(`downlink  ${answer.payload.length} bytes at ${answer.timestampUs} us`)

// A gateway hears every network in range, and a frame from one this site never granted is
// reported rather than refused.
const stranger = site.uplink({
  frequencyHz: 868_100_000,
  payload: session(0x12345678, Buffer.alloc(16, 0x09), Buffer.alloc(16, 0x08)).encodeUplink(
    0,
    1,
    Buffer.from('hello'),
  ),
  link: dr5,
})
console.log(
  `foreign   0x${stranger.devAddr.toString(16).padStart(8, '0')} belongs to another network`,
)
// ANCHOR_END: network

assert.equal(carried.payload!.toString(), '21.5')
assert.equal(carried.slot!.timestampUs, 10_000_000)
assert.equal(stranger.outcome, 'Foreign')

// ANCHOR: station
import {
  DISCOVERY_PATH,
  STATION_PROTOCOL_VERSION,
  StationKind,
  stationDiscovery,
  stationHeard,
  stationEncode,
  stationRouterParse,
} from '@pamoja/gateway'

// The same gateway, now speaking the other protocol. It is configured with an address, and
// asks on that path for the websocket its session runs on.
const stationEui = 'b827ebfffe010203'
const ask = stationDiscovery(stationEui)
console.log(`ask       ${DISCOVERY_PATH} ${ask}`)

// The server answers with where to connect, or with why it will not have this station.
const routed = stationRouterParse(
  '{"router":"b827:ebff:fe01:203","muxs":"::0","uri":"ws://lns.example.invalid:3001/router"}',
)
console.log(`open      ${routed.uri}`)

// A station opens with what it is, which is how the server knows what it can do.
const hello = stationEncode({
  kind: StationKind.Version,
  station: 'pamoja',
  firmware: '0.1.18',
  package: 'pamoja-gateway',
  model: 'linux',
  protocol: STATION_PROTOCOL_VERSION,
  features: 'gps',
})
console.log(`version   ${hello}`)

// Now a device sends a reading, and the radio hears the frame. A station holds no key, so it
// does not read the payload: it splits the frame into the fields the protocol names and lets
// the server judge them.
const active = session(0x26010001, Buffer.alloc(16, 0x44), Buffer.alloc(16, 0x55))
const frame = active.encodeUplink(7, 2, Buffer.from('21.5'))
const reported = stationHeard(frame, 5, 868_100_000, {
  rctx: 0,
  xtime: 1_000_000,
  rssi: -35,
  snr: 5.1,
})
console.log(`updf      ${stationEncode(reported)}`)
console.log(
  `heard     0x${(reported.devAddr ?? 0).toString(16).padStart(8, '0')} counter ${reported.fcnt} on port ${reported.fport}`,
)
console.log(`payload   ${reported.payload?.length} bytes, still encrypted`)
// ANCHOR_END: station

assert.equal(ask, '{"router":"b827:ebff:fe01:203"}')
assert.equal(routed.uri, 'ws://lns.example.invalid:3001/router')
assert.equal(reported.devAddr, 0x26010001)
assert.equal(reported.fcnt, 7)
assert.equal(reported.fport, 2)
assert.notEqual(reported.payload?.toString(), '21.5')
