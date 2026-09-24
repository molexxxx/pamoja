// The LoRaWAN gateway guide example; see docs/guides/gateway.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { PacketKind, TxStatus, acknowledgment, encode, parse } from '@pamoja/gateway'
import { link } from '@pamoja/lora'
import { session } from '@pamoja/lorawan'

function aGatewayAndItsServerTradeDatagrams() {
  // A gateway on a Raspberry Pi, whose identifier is written from its network interface.
  const gateway = 'b827ebfffe010203'

  // Every few seconds it sends a PULL_DATA, which holds a path open through whatever
  // translates its address, so the server has somewhere to send a downlink. The server
  // answers each one, and a gateway that stops hearing answers knows the path is gone.
  const pull = encode({ kind: PacketKind.PullData, token: 0x7a01, gateway })
  const held = acknowledgment(parse(pull))!
  console.log(
    `pull      ${pull.length} bytes out and ${encode(held).length} back hold the downlink path open`,
  )

  // A node sends a reading, and the gateway hears it on 868.1 MHz at SF9, near the edge of
  // its range. It forwards the frame as it arrived, with the levels, the concentrator's own
  // timestamp, and its counts since the last report. It holds no key and reads none of it.
  const node = session(0x26010001, Buffer.alloc(16, 0x44), Buffer.alloc(16, 0x55))
  const frame = node.encodeUplink(7, 2, Buffer.from('21.5'))
  const heard = {
    frequencyHz: 868_100_000,
    payload: frame,
    link: link(9, 125_000),
    rssiDbm: -97,
    snrDb: -3.2,
    timestampUs: 3_512_348_611,
  }
  const counts = {
    received: 2,
    receivedOk: 1,
    forwarded: 1,
    acknowledgedPercent: 100,
    downlinks: 0,
    transmitted: 0,
  }
  const datagram = encode({
    kind: PacketKind.PushData,
    token: 0x1234,
    gateway,
    packets: [heard],
    status: counts,
  })
  console.log(
    `push      a reading and the gateway's counts, ${datagram.length} bytes, ` +
      `token ${(0x1234).toString(16).padStart(4, '0')}`,
  )

  // The server reads it. The frequency is in hertz, the datarate identifier is the link
  // settings, and the payload is bytes, so nothing is decoded by hand.
  const forwarded = parse(datagram)
  const received = forwarded.packets![0]
  console.log(
    `heard     ${received.frequencyHz} Hz at SF${received.link!.spreadingFactor}, ` +
      `${received.link!.bandwidthHz / 1000} kHz, ${received.rssiDbm!.toFixed(0)} dBm, ` +
      `SNR ${received.snrDb!.toFixed(1)} dB, CRC ${received.crc!.toLowerCase()}, ` +
      `${received.payload.length} bytes`,
  )
  const report = forwarded.status!
  console.log(
    `counts    ${report.received} received, ${report.receivedOk} with a good CRC, ` +
      `${report.forwarded} forwarded, ${report.acknowledgedPercent!.toFixed(1)}% acknowledged`,
  )

  // It is acknowledged at once, by token, before anything in it is read.
  const ack = acknowledgment(parse(datagram))!
  console.log(
    `ack       token ${ack.token.toString(16).padStart(4, '0')} acknowledged in ` +
      `${encode(ack).length} bytes`,
  )

  // An answer goes back in a PULL_RESP, timed in the concentrator's own microseconds for
  // the device's first receive window, a second after the uplink ended, with the inverted
  // polarity a LoRaWAN device listens for.
  const answer = node.encodeDownlink(0, 2, Buffer.from('ok'))
  const pullResp = encode({
    kind: PacketKind.PullResp,
    token: 0x00ab,
    transmit: {
      frequencyHz: 868_100_000,
      payload: answer,
      link: link(9, 125_000),
      timestampUs: 3_513_348_611,
      powerDbm: 14,
      invertPolarity: true,
    },
  })
  const transmit = parse(pullResp).transmit!
  const iq = transmit.invertPolarity ? 'IQ inverted' : 'IQ upright'
  console.log(
    `downlink  at ${transmit.timestampUs} us on ${transmit.frequencyHz} Hz, ` +
      `${transmit.powerDbm} dBm, ${iq}`,
  )

  // The gateway answers each PULL_RESP with a TX_ACK saying what became of it: scheduled,
  // or refused with a reason, such as a window that had already passed.
  for (const said of [TxStatus.None, TxStatus.TooLate]) {
    const reported = encode({ kind: PacketKind.TxAck, token: 0x00ab, gateway, txStatus: said })
    const status = parse(reported).txStatus!
    const meaning = status === TxStatus.None ? "it goes out in the device's window" : 'it was not sent'
    console.log(`txack     ${status}: ${meaning}`)
  }
  return { held, frame, received, ack, transmit }
}
// ANCHOR_END: example

function checkTheDatagrams(traded: ReturnType<typeof aGatewayAndItsServerTradeDatagrams>): void {
  assert.equal(encode(traded.held).length, 4)
  assert.deepEqual(traded.received.payload, traded.frame)
  assert.equal(traded.ack.token, 0x1234)
  assert.equal(traded.transmit.timestampUs, 3_513_348_611)
}

// ANCHOR: network
import { Network } from '@pamoja/gateway'
import { LoraRegion, planFor } from '@pamoja/lora'
import { device } from '@pamoja/lorawan'

function aDeviceJoinsASiteAndIsAnswered() {
  // One site, on the band it operates in, admitting one device it was told about: its EUI
  // from its label, the application it joins, and the root key it was provisioned with.
  const devEui = Buffer.from('70b3d57ed0001234', 'hex')
  const joinEui = Buffer.from('70b3d57ed0000000', 'hex')
  const appKey = Buffer.alloc(16, 0x2b)
  const site = new Network(planFor(LoraRegion.Eu868), 0x000013, null, 0x26010001)
  site.register(devEui, joinEui, appKey)

  // The gateway forwards a join request it heard. Nothing about the device is known here
  // beyond the key, which is what verifies the request, and the accept is timed for the
  // join window, five seconds after the request.
  const eu868 = planFor(LoraRegion.Eu868)
  const dr5 = eu868.linkSettings(5)!
  const joiner = device(devEui, joinEui, appKey)
  const heardAt = 1_000_000
  const joined = site.uplink({
    frequencyHz: 868_100_000,
    payload: joiner.joinRequest(0x0102),
    link: dr5,
    timestampUs: heardAt,
  })
  const acceptedAt = joined.accept!.timestampUs!
  console.log(
    `joined    0x${joined.devAddr.toString(16).padStart(8, '0')}, accepted at ${acceptedAt} us, ` +
      `${(acceptedAt - heardAt) / 1_000_000} s after the request`,
  )

  // The device reads the accept and sends a reading. The site decrypts it and says where
  // an answer goes: the uplink's own channel, a second after it ended.
  const granted = joiner.acceptJoin(joined.accept!.payload, 0x0102).session()
  const carried = {
    frequencyHz: 868_100_000,
    payload: granted.encodeUplink(0, 2, Buffer.from('21.5')),
    link: dr5,
    timestampUs: 9_000_000,
  }
  const reading = site.uplink(carried)
  console.log(
    `uplink    frame ${reading.fcnt} on port ${reading.fport} says ${reading.payload!.toString()}, ` +
      `answer at ${reading.slot!.timestampUs} us on ${reading.slot!.frequencyHz} Hz`,
  )

  // The answer goes out in that window, encrypted with the session the join granted.
  const answer = site.answer(reading.devAddr, reading.slot!, 2, Buffer.from('ok'))
  console.log(`answer    ${answer.payload.length} bytes at ${answer.timestampUs} us`)

  // The same frame again, as a replay would send it, is refused: its counter was seen.
  try {
    site.uplink(carried)
    assert.fail('a counter is taken once')
  } catch (refused) {
    console.log(`replay    ${(refused as Error).message}`)
  }

  // A gateway hears every network in range, and a frame from one this site never granted
  // is reported as another network's rather than refused.
  const elsewhere = session(0x12345678, Buffer.alloc(16, 0x09), Buffer.alloc(16, 0x08))
  const stranger = site.uplink({
    frequencyHz: 868_300_000,
    payload: elsewhere.encodeUplink(0, 1, Buffer.from('hello')),
    link: dr5,
  })
  console.log(
    `foreign   0x${stranger.devAddr.toString(16).padStart(8, '0')} belongs to another network`,
  )
  return { reading, stranger }
}
// ANCHOR_END: network

function checkTheSite(site: ReturnType<typeof aDeviceJoinsASiteAndIsAnswered>): void {
  assert.equal(site.reading.payload!.toString(), '21.5')
  assert.equal(site.reading.slot!.timestampUs, 10_000_000)
  assert.equal(site.stranger.outcome, 'Foreign')
}

// ANCHOR: station
import {
  DISCOVERY_PATH,
  STATION_PROTOCOL_VERSION,
  StationKind,
  stationDiscovery,
  stationDiscoveryParse,
  stationEncode,
  stationHeard,
  stationParse,
  stationRouterAccepted,
  stationRouterParse,
  stationXtime,
  stationXtimeParts,
} from '@pamoja/gateway'
import { version } from '@pamoja/core'

function aStationSessionFromBothSides() {
  // The station asks its configured address where its network server is, naming itself.
  // The server reads who asked, and sends it to the websocket its session runs on.
  const station = 'b827ebfffe010203'
  const asking = stationDiscovery(station)
  console.log(`ask       ${DISCOVERY_PATH} ${asking}`)
  const asked = stationDiscoveryParse(asking)
  const answer = stationRouterAccepted(
    asked,
    '0000000000000001',
    'ws://lns.example.invalid:3001/router',
  )
  console.log(`open      ${stationRouterParse(answer).uri}`)

  // Once the websocket is open the station speaks first, saying what it is.
  const hello = stationEncode({
    kind: StationKind.Version,
    station: 'pamoja',
    firmware: version(),
    package: 'pamoja-gateway',
    model: 'linux',
    protocol: STATION_PROTOCOL_VERSION,
    features: 'gps',
  })
  const said = stationParse(hello)
  console.log(`version   ${said.station} ${said.firmware} on ${said.model}, protocol ${said.protocol}`)

  // The radio hears a node's reading 3512.348611 seconds into the station's first run. A
  // station holds no key, so it splits the frame into the fields the protocol names and
  // lets the server judge them, with its own clock for the moment it arrived.
  const node = session(0x26010001, Buffer.alloc(16, 0x44), Buffer.alloc(16, 0x55))
  const frame = node.encodeUplink(7, 2, Buffer.from('21.5'))
  const heardAt = stationXtime(0, 1, 3_512_348_611)
  const updf = stationEncode(
    stationHeard(frame, 5, 868_100_000, { rctx: 0, xtime: heardAt, rssi: -97, snr: -3.2 }),
  )
  const uplink = stationParse(updf)
  console.log(
    `updf      0x${(uplink.devAddr! >>> 0).toString(16).padStart(8, '0')} counter ${uplink.fcnt} ` +
      `on port ${uplink.fport}, DR${uplink.dataRate}, ${uplink.payload!.length} bytes still encrypted`,
  )

  // The server answers in the receive windows the region gives: the first at the uplink's
  // own rate and channel, the second where the plan fixes it. It hands the station's clock
  // back untouched, so the station can time the answer from the moment it heard the uplink.
  const eu868 = planFor(LoraRegion.Eu868)
  const rx1Rate = eu868.rx1DataRate(uplink.dataRate!, 0)!
  const rx2 = eu868.rx2()
  const dnmsg = stationEncode({
    kind: StationKind.Downlink,
    devEui: '70b3d57ed0001234',
    class: 0,
    diid: 1n,
    pdu: node.encodeDownlink(0, 2, Buffer.from('ok')),
    rxDelay: 1,
    rx1: { dataRate: rx1Rate, frequencyHz: uplink.frequencyHz! },
    rx2: { dataRate: rx2.dataRate, frequencyHz: rx2.frequencyHz },
    priority: 0,
    xtime: uplink.levels!.xtime,
    rctx: uplink.levels!.rctx,
  })
  const told = stationParse(dnmsg)
  const delay = told.rxDelay ?? 1
  console.log(
    `dnmsg     RX1 DR${told.rx1!.dataRate} on ${told.rx1!.frequencyHz} Hz or ` +
      `RX2 DR${told.rx2!.dataRate} on ${told.rx2!.frequencyHz} Hz, ${delay} s after the uplink`,
  )

  // The station opens the first window a second after the uplink on its own clock, puts
  // the answer on the air, and reports it by the identifier the server gave it.
  const uplinkAt = stationXtimeParts(told.xtime!)
  const sentAt = stationXtime(uplinkAt.unit, uplinkAt.session, uplinkAt.micros + delay * 1_000_000)
  const dntxed = stationEncode({
    kind: StationKind.Transmitted,
    diid: told.diid,
    devEui: told.devEui,
    rctx: told.rctx ?? 0,
    xtime: sentAt,
    txtime: stationXtimeParts(sentAt).micros / 1e6,
  })
  const reported = stationParse(dntxed)
  const went = stationXtimeParts(reported.xtime!)
  console.log(`dntxed    downlink ${reported.diid} went out at ${went.micros} us of run ${went.session}`)
  return { asked, station, uplink, went }
}
// ANCHOR_END: station

function checkTheStation(session: ReturnType<typeof aStationSessionFromBothSides>): void {
  assert.equal(session.asked, session.station)
  assert.equal(session.uplink.devAddr, 0x26010001)
  assert.equal(session.uplink.fcnt, 7)
  assert.notEqual(session.uplink.payload!.toString(), '21.5')
  assert.equal(session.went.micros, 3_513_348_611)
}

// ANCHOR: hardware
import { createSocket } from 'node:dgram'
import { DEFAULT_PORT, Crc } from '@pamoja/gateway'

async function aNetworkServerBesideTheGateway(): Promise<void> {
  // The daemon forwards to 127.0.0.1 when it runs on the same Pi. To serve gateways
  // elsewhere, listen on 0.0.0.0 behind a firewall that admits only them: the protocol has
  // no authentication of its own.
  const site = new Network(planFor(LoraRegion.Eu868), 0x000013)
  site.register(
    Buffer.from('70b3d57ed0001234', 'hex'),
    Buffer.from('70b3d57ed0000000', 'hex'),
    Buffer.alloc(16, 0x2b),
  )

  const socket = createSocket('udp4')
  const bound = await new Promise<boolean>((resolve) => {
    socket.once('error', () => resolve(false))
    socket.bind(DEFAULT_PORT, '127.0.0.1', () => resolve(true))
  })
  if (!bound) {
    console.log(`absent    another program holds port ${DEFAULT_PORT}`)
    return
  }

  // A pamoja gateway holds its path open every five seconds, so six seconds of silence
  // means none is running. After that the server keeps answering until a minute passes
  // with nothing heard.
  let downlinks: { address: string; port: number } | null = null
  let token = 0
  const next = (waitMs: number) =>
    new Promise<{ datagram: Buffer; from: { address: string; port: number } } | null>((resolve) => {
      const timer = setTimeout(() => {
        socket.removeAllListeners('message')
        resolve(null)
      }, waitMs)
      socket.once('message', (datagram, from) => {
        clearTimeout(timer)
        resolve({ datagram, from })
      })
    })

  for (let arrived = await next(6_000); arrived; arrived = await next(60_000)) {
    const { datagram, from } = arrived
    let packet
    try {
      packet = parse(datagram)
    } catch (why) {
      console.log(`ignored   ${(why as Error).message}`)
      continue
    }
    const ack = acknowledgment(packet)
    if (ack) {
      socket.send(encode(ack), from.port, from.address)
    }

    if (packet.kind === PacketKind.PullData) {
      if (!downlinks) {
        console.log(`gateway   ${packet.gateway} holds its downlink path open`)
      }
      downlinks = from
    } else if (packet.kind === PacketKind.PushData) {
      for (const heard of packet.packets ?? []) {
        if (heard.crc !== Crc.Ok) {
          continue
        }
        let transmit
        try {
          const event = site.uplink(heard)
          if (event.outcome === 'Joined') {
            console.log(`joined    0x${event.devAddr.toString(16).padStart(8, '0')}`)
            transmit = event.accept!
          } else if (event.outcome === 'Data') {
            console.log(
              `reading   0x${event.devAddr.toString(16).padStart(8, '0')} says ${event.payload!.toString()}`,
            )
            transmit = site.answer(event.devAddr, event.slot!, 2, Buffer.from('ok'))
          } else {
            continue
          }
        } catch (why) {
          console.log(`refused   ${(why as Error).message}`)
          continue
        }
        if (downlinks) {
          token = (token + 1) & 0xffff
          socket.send(
            encode({ kind: PacketKind.PullResp, token, transmit }),
            downlinks.port,
            downlinks.address,
          )
        }
      }
    } else if (packet.kind === PacketKind.TxAck) {
      console.log(`txack     ${packet.txStatus}`)
    }
  }
  if (!downlinks) {
    console.log('absent    no gateway reported in, so nothing was answered')
  }
  socket.close()
}
// ANCHOR_END: hardware

async function run(): Promise<void> {
  checkTheDatagrams(aGatewayAndItsServerTradeDatagrams())
  checkTheSite(aDeviceJoinsASiteAndIsAnswered())
  checkTheStation(aStationSessionFromBothSides())
  await aNetworkServerBesideTheGateway()
}

run().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
