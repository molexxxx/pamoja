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
import { MqttClient, Qos } from '@pamoja/mqtt'

// The broker on the site. The guide's CI runs one on localhost; point these at yours and
// nothing else changes.
const BROKER = '127.0.0.1'
const PORT = 1883

async function main(): Promise<{ topic: string; payload: Buffer }> {
  // The gateway takes every temperature on the site. A `+` stands for exactly one level,
  // so this matches every node's temperature and nothing deeper.
  const gateway = new MqttClient({
    clientId: 'site-gateway',
    host: BROKER,
    port: PORT,
    qos: Qos.AtLeastOnce,
  })
  await gateway.connect()
  await gateway.subscribe('sensors/+/temperature')
  console.log('gateway   subscribed to sensors/+/temperature')

  // A node publishes under that pattern. At-least-once means the broker acknowledges the
  // message, so a node knows its reading was taken rather than hoping.
  const node = new MqttClient({
    clientId: 'node-1',
    host: BROKER,
    port: PORT,
    qos: Qos.AtLeastOnce,
  })
  await node.connect()
  await node.publish('sensors/1/temperature', '21.5')
  console.log('node      published 21.5 to sensors/1/temperature')

  // The gateway receives it with the topic attached, which is how it knows which node
  // sent the reading without the payload having to repeat it.
  const received = (await gateway.recv())!
  console.log(`gateway   got ${received.payload.toString()} on ${received.topic}`)

  // Disconnecting leaves the client reusable, so a node that loses its link can reconnect
  // the same object when the broker comes back.
  await node.disconnect()
  console.log(`node      disconnected, still connected: ${await node.isConnected()}`)
  await gateway.disconnect()

  // A broker that is not there is reported rather than leaving a client that looks
  // connected, so a retry loop has something to test.
  const nowhere = new MqttClient({ clientId: 'node-2', host: BROKER, port: 1, keepAliveSecs: 1 })
  try {
    await nowhere.connect()
    console.log('an unreachable broker accepted a connection, which should never happen')
  } catch (error) {
    console.log(`unreachable broker refused: ${(error as Error).message}`)
  }

  return received
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mqtt`](https://crates.io/crates/pamoja-mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mqtt/index.html), [docs.rs](https://docs.rs/pamoja-mqtt), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-mqtt) |
| TypeScript | [`@pamoja/mqtt`](https://www.npmjs.com/package/@pamoja/mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-mqtt) |
| Python | [`pamoja-mqtt`](https://pypi.org/project/pamoja-mqtt/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mqtt.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-mqtt) |
| C# | [`Pamoja.Mqtt`](https://www.nuget.org/packages/Pamoja.Mqtt) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mqtt.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-mqtt) |

## Documentation

- [`@pamoja/mqtt` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html), every class, function, and type this package exports.
- [The MQTT guide](https://pamoja.molex.cloud/docs/guides/mqtt.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
