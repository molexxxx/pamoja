# @pamoja/bus

An in-memory typed publish and subscribe event bus. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/bus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/bus
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/bus.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/bus.ts):

```typescript
import { EventBus } from '@pamoja/bus'

async function main() {
  // A sampler announces something and whatever cares picks it up, with neither side
  // holding a reference to the other. This is how the parts of one node are wired.
  const hub = new EventBus(8)
  const control = await hub.subscribe()
  const logger = await hub.subscribe()

  await hub.publish(Buffer.from('battery.low'))
  const toControl = (await control.next())!
  const toLogger = (await logger.next())!
  console.log(`control saw ${toControl.toString()}, the logger saw ${toLogger.toString()}`)

  // A subscriber taken later starts from the next event, so it never sees what went out
  // before it existed.
  const late = await hub.subscribe()
  await hub.publish(Buffer.from('link.up'))
  const firstSeen = (await late.next())!
  console.log(`the late subscriber's first event is ${firstSeen.toString()}`)

  // The buffer is per subscriber and bounded, so one further behind than the capacity
  // drops what it missed and resumes with the most recent events. A slow reader costs
  // itself, not the publisher.
  const slow = new EventBus(2)
  const reader = await slow.subscribe()
  for (let count = 0; count < 5; count += 1) {
    await slow.publish(Buffer.from([count]))
  }
  const resumed = (await reader.next())!
  console.log(`after five events into a buffer of two, the reader resumes at ${resumed[0]}`)

  return { toControl, toLogger, firstSeen, resumed }
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-bus`](https://crates.io/crates/pamoja-bus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html), [docs.rs](https://docs.rs/pamoja-bus) |
| TypeScript | [`@pamoja/bus`](https://www.npmjs.com/package/@pamoja/bus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html) |
| Python | [`pamoja-bus`](https://pypi.org/project/pamoja-bus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html) |
| C# | [`Pamoja.Bus`](https://www.nuget.org/packages/Pamoja.Bus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html) |

## Documentation

- [`@pamoja/bus` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html), every class, function, and type this package exports.
- [The Event bus guide](https://pamoja.molex.cloud/docs/guides/bus.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
