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
