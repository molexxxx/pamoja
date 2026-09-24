# @pamoja/mqtt

An MQTT client with the topic and wildcard rules, acknowledged delivery, retained messages, a last will, and TLS, as the core transport. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mqtt.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mqtt.html)

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

const connection = (connected: boolean) => (connected ? 'still connected' : 'not connected')

async function main() {
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

  // A node publishes under that pattern. At least once has the broker acknowledge each
  // message, where at most once would send it and forget it. It also leaves a will: should
  // it drop off the network without saying goodbye, the broker publishes offline on its
  // status topic for it.
  const node = new MqttClient({
    clientId: 'node-1',
    host: BROKER,
    port: PORT,
    qos: Qos.AtLeastOnce,
    will: { topic: 'sites/node-1/status', payload: 'offline', qos: Qos.AtLeastOnce, retain: true },
  })
  await node.connect()
  await node.publish('sensors/1/temperature', '21.5')
  console.log('node      published 21.5 to sensors/1/temperature')

  // The gateway receives it with the topic attached, which is how it knows which node
  // sent the reading without the payload having to repeat it.
  const received = (await gateway.recv())!
  console.log(`gateway   got ${received.text!} on ${received.topic}`)

  // The node says it is up, retained, so a dashboard that opens later sees it at once.
  // `publishConfirmed` resolves once the broker acknowledges the message.
  await node.publishConfirmed('sites/node-1/status', 'online', { retain: true })
  console.log('node      the broker holds online on sites/node-1/status')

  // A dashboard that subscribes afterwards still gets it, because the broker keeps the
  // last retained message on each topic for whoever subscribes next.
  const dashboard = new MqttClient({ clientId: 'site-dashboard', host: BROKER, port: PORT })
  await dashboard.connect()
  await dashboard.subscribe('sites/node-1/status')
  const status = (await dashboard.recv())!
  console.log(`dashboard ${status.topic} is ${status.text!}`)

  // Two days of readings saved at one a minute, sent as one message, make a packet over
  // the connection's 10 KiB limit. The send is refused before anything leaves and the
  // connection stays up; a node that must send it raises the limit on every client that
  // shares the topic, or splits it.
  const backlog = Array(2 * 24 * 60).fill('21.5').join(',')
  try {
    await node.publish('sensors/1/backlog', backlog)
    console.log('node      sent an oversized backlog, which should never happen')
  } catch (error) {
    console.log(`node      backlog refused: ${(error as Error).message}`)
  }
  const afterRefusal = await node.isConnected()
  console.log(`node      ${connection(afterRefusal)}`)

  // Before it leaves, the node says so itself. A clean disconnect discards the will, which
  // is only for a node that drops off without this goodbye.
  await node.publishConfirmed('sites/node-1/status', 'offline', { retain: true })
  const goodbye = (await dashboard.recv())!
  console.log(`dashboard ${goodbye.topic} is ${goodbye.text!}`)

  // Disconnecting leaves the client reusable, so a node that loses its link can reconnect
  // the same object when the broker comes back.
  await node.disconnect()
  const afterDisconnect = await node.isConnected()
  console.log(`node      ${connection(afterDisconnect)} after disconnecting`)
  await gateway.disconnect()
  await dashboard.disconnect()

  // A broker that is not there is reported rather than leaving a client that looks
  // connected, so a retry loop has something to test.
  const nowhere = new MqttClient({ clientId: 'node-2', host: BROKER, port: 1, keepAliveSecs: 1 })
  try {
    await nowhere.connect()
    console.log('an unreachable broker accepted a connection, which should never happen')
  } catch (error) {
    console.log(`unreachable broker refused: ${(error as Error).message}`)
  }

  return { received, status, goodbye, afterRefusal, afterDisconnect }
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
