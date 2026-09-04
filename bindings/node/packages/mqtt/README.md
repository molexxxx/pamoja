# @pamoja/mqtt

An MQTT client with the topic and wildcard rules, as the core transport. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mqtt.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/mqtt
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/mqtt.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mqtt.ts):

```typescript
import assert from 'node:assert/strict'

import { MqttClient, Qos } from '@pamoja/mqtt'

async function main() {
  // MQTT numbers its three delivery guarantees 0, 1 and 2 on the wire; the binding names
  // them, in that order.
  assert.deepEqual(Object.values(Qos), ['AtMostOnce', 'AtLeastOnce', 'ExactlyOnce'])

  // Nothing listens on this port, so the broker is unreachable. Constructing the client
  // touches nothing; only connecting does.
  const client = new MqttClient({
    clientId: 'guide-node',
    host: '127.0.0.1',
    port: 47811,
    keepAliveSecs: 1,
    qos: Qos.ExactlyOnce,
  })
  assert.equal(await client.isConnected(), false)

  // A refused connection surfaces as a transport error and leaves the client as it was, so
  // the same object can be retried once the broker is back.
  await assert.rejects(() => client.connect(), /transport error/)
  assert.equal(await client.isConnected(), false)
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mqtt`](https://crates.io/crates/pamoja-mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html), [docs.rs](https://docs.rs/pamoja-mqtt) |
| TypeScript | [`@pamoja/mqtt`](https://www.npmjs.com/package/@pamoja/mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html) |
| Python | [`pamoja-mqtt`](https://pypi.org/project/pamoja-mqtt/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html) |
| C# | [`Pamoja.Mqtt`](https://www.nuget.org/packages/Pamoja.Mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html) |

## Documentation

- [`@pamoja/mqtt` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html), every class, function, and type this package exports.
- [The MQTT guide](https://pamoja.molex.cloud/docs/guides/mqtt.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
