# @pamoja/coap

A CoAP client and the server it reports to, over UDP, with confirmable delivery and observe. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/coap.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html)

## Install

```sh
npm install @pamoja/coap
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/coap.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/coap.ts):

```typescript
import { CoapClient, CoapServer, Reliability } from '@pamoja/coap'

async function main() {
  // The gateway in the orchard's shed. It takes moisture readings from every row, and holds
  // the irrigation valve's state for the rows to observe. Port 0 lets the system pick a free
  // port, which the rows are pointed at below.
  const gateway = new CoapServer('127.0.0.1:0')
  await gateway.connect()
  await gateway.subscribe('orchard/+/moisture')
  await gateway.send('orchard/valve', 'closed')
  const port = gateway.localPort!
  console.log('gateway   takes moisture readings on orchard/+/moisture')

  // A battery-powered sensor in row 7. Its reading is confirmable, so it waits for the
  // gateway's acknowledgment and retransmits until one comes back.
  const row7 = new CoapClient({ host: '127.0.0.1', port, ackTimeoutMs: 200 })
  await row7.connect()
  await row7.send('orchard/row-7/moisture', '31')
  console.log('row-7     reported 31, and the gateway acknowledged it')
  const reading = (await gateway.recv())!
  console.log(`gateway   took ${reading.text!} from ${reading.topic}`)

  // Observing the valve registers the row with the gateway, which answers with the valve's
  // state now and notifies every change after it, as RFC 7641 describes.
  await row7.subscribe('orchard/valve')
  const current = (await row7.recv())!
  console.log(`row-7     observes ${current.topic}, which reads ${current.text!}`)
  await gateway.send('orchard/valve', 'open')
  const observers = gateway.observers('orchard/valve')
  console.log(`gateway   opened the valve for ${observers} observer`)
  const change = (await row7.recv())!
  console.log(`row-7     ${change.topic} now reads ${change.text!}`)

  // A path the gateway does not take is answered 4.04, and a confirmable send reports that
  // rather than counting the reading as delivered.
  try {
    await row7.send('orchard/row-7/battery', '3.1')
    console.log('row-7     the battery reading was taken, which should never happen')
  } catch (error) {
    console.log(`row-7     battery refused: ${(error as Error).message}`)
  }

  // Row 8 sends non-confirmable: once and unacknowledged, which costs the least radio time
  // and suits a reading whose loss costs nothing.
  const row8 = new CoapClient({ host: '127.0.0.1', port, reliability: Reliability.NonConfirmable })
  await row8.connect()
  await row8.send('orchard/row-8/moisture', '27')
  console.log('row-8     sent 27 without waiting for an answer')
  const unconfirmed = (await gateway.recv())!
  console.log(`gateway   took ${unconfirmed.text!} from ${unconfirmed.topic}`)

  // Row 9 is pointed at port 1, where nothing listens. A confirmable send retransmits on a
  // doubling wait and then gives up. RFC 7252's defaults would take more than a minute to
  // get there, so this one waits 20 ms and retransmits once.
  const row9 = new CoapClient({ host: '127.0.0.1', port: 1, ackTimeoutMs: 20, maxRetransmits: 1 })
  await row9.connect()
  try {
    await row9.send('orchard/row-9/moisture', '29')
    console.log('row-9     an empty port acknowledged it, which should never happen')
  } catch (error) {
    console.log(`row-9     gave up unacknowledged: ${(error as Error).message}`)
  }

  for (const link of [row7, row8, row9]) {
    await link.disconnect()
  }
  await gateway.disconnect()
  return { reading, current, change, observers, unconfirmed }
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-coap`](https://crates.io/crates/pamoja-coap) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_coap/index.html), [docs.rs](https://docs.rs/pamoja-coap), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-coap) |
| TypeScript | [`@pamoja/coap`](https://www.npmjs.com/package/@pamoja/coap) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-coap) |
| Python | [`pamoja-coap`](https://pypi.org/project/pamoja-coap/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/coap.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-coap) |
| C# | [`Pamoja.Coap`](https://www.nuget.org/packages/Pamoja.Coap) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Coap.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-coap) |

## Documentation

- [`@pamoja/coap` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html), every class, function, and type this package exports.
- [The CoAP guide](https://pamoja.molex.cloud/docs/guides/coap.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
