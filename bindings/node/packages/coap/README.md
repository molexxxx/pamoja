# @pamoja/coap

A CoAP client over UDP with confirmable delivery and observe. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/coap.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/coap
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/coap.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/coap.ts):

```typescript
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-coap`](https://crates.io/crates/pamoja-coap) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_coap/index.html), [docs.rs](https://docs.rs/pamoja-coap) |
| TypeScript | [`@pamoja/coap`](https://www.npmjs.com/package/@pamoja/coap) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html) |
| Python | [`pamoja-coap`](https://pypi.org/project/pamoja-coap/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/coap.html) |
| C# | [`Pamoja.Coap`](https://www.nuget.org/packages/Pamoja.Coap) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Coap.html) |

## Documentation

- [`@pamoja/coap` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html), every class, function, and type this package exports.
- [The CoAP guide](https://pamoja.molex.cloud/docs/guides/coap.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
