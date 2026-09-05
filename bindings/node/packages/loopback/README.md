# @pamoja/loopback

An in-process transport with topic matching and a fault injector, for testing with no broker. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/loopback.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/loopback
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/loopback.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/loopback.ts):

```typescript
import { LoopbackBroker } from '@pamoja/loopback'

async function main() {
  // One broker and two links off it, all in this process. Nothing binds a port and nothing
  // has to be running for the traffic below to flow, which is what makes this the link to
  // develop a node against before it has a real one.
  const broker = new LoopbackBroker()
  const publisher = broker.link()
  const subscriber = broker.link()
  await publisher.connect()
  await subscriber.connect()

  // A `+` stands for exactly one level, so this takes the node's temperature but not the
  // raw reading a level below it.
  await subscriber.subscribe('sensors/+/temperature')
  await publisher.send('sensors/8/temperature/raw', Buffer.from('2150'))
  await publisher.send('sensors/8/temperature', Buffer.from('21.5'))

  const message = (await subscriber.recv())!
  console.log(`sensors/+/temperature took ${message.payload.toString()} from ${message.topic}`)

  // A `#` covers every level that remains, so a second link takes the whole subtree,
  // including the reading the single-level filter passed over.
  const watcher = broker.link()
  await watcher.connect()
  await watcher.subscribe('sensors/#')
  await publisher.send('sensors/8/temperature/raw', Buffer.from('2150'))

  const deep = (await watcher.recv())!
  console.log(`sensors/#             took ${deep.payload.toString()} from ${deep.topic}`)

  // A link that has been disconnected reports the failure instead of dropping the reading,
  // which is the case a test wants to reach without unplugging anything.
  await publisher.disconnect()
  try {
    await publisher.send('sensors/8/temperature', Buffer.from('21.6'))
    console.log('a disconnected link took a reading, which should never happen')
  } catch (error) {
    console.log(`disconnected refused the reading: ${(error as Error).message}`)
  }

  return { message, deep }
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-loopback`](https://crates.io/crates/pamoja-loopback) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html), [docs.rs](https://docs.rs/pamoja-loopback) |
| TypeScript | [`@pamoja/loopback`](https://www.npmjs.com/package/@pamoja/loopback) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html) |
| Python | [`pamoja-loopback`](https://pypi.org/project/pamoja-loopback/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/loopback.html) |
| C# | [`Pamoja.Loopback`](https://www.nuget.org/packages/Pamoja.Loopback) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Loopback.html) |

## Documentation

- [`@pamoja/loopback` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_loopback.html), every class, function, and type this package exports.
- [The Loopback guide](https://pamoja.molex.cloud/docs/guides/loopback.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
