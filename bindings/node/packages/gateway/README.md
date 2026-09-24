# @pamoja/gateway

What a LoRaWAN gateway speaks: the Semtech packet forwarder protocol and the Basics Station protocol on both sides, the network side of a single site, and a bridge from the radio to the link that leaves it. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/gateway.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gateway.html)

## Install

```sh
npm install @pamoja/gateway
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/gateway.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gateway.ts):

```typescript
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-gateway`](https://crates.io/crates/pamoja-gateway) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gateway/index.html), [docs.rs](https://docs.rs/pamoja-gateway), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gateway) |
| TypeScript | [`@pamoja/gateway`](https://www.npmjs.com/package/@pamoja/gateway) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gateway.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gateway) |
| Python | [`pamoja-gateway`](https://pypi.org/project/pamoja-gateway/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gateway.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gateway) |
| C# | [`Pamoja.Gateway`](https://www.nuget.org/packages/Pamoja.Gateway) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gateway.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gateway) |

## Documentation

- [`@pamoja/gateway` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gateway.html), every class, function, and type this package exports.
- [The LoRaWAN gateways guide](https://pamoja.molex.cloud/docs/guides/gateway.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
