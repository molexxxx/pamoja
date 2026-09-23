# @pamoja/bus

An in-memory typed publish and subscribe event bus, with publishers that never wait and subscribers that count what they miss. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/bus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html)

## Install

```sh
npm install @pamoja/bus
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/bus.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/bus.ts):

```typescript
import { EventPublisher } from '@pamoja/bus'

const QUIET_MS = 50

async function main() {
  // The station's wiring makes one bus and hands each part what it needs: a publisher
  // to announce, an endpoint to listen. No part holds a reference to another, so any of
  // them can be replaced without touching the rest.
  const bus = new EventPublisher(2)
  const power = bus.publisher()
  const sampler = bus.publisher()
  const heater = bus.subscribe()
  const logger = bus.subscribe()

  // One announcement reaches every part that listens, and each reads its own copy.
  const reached = power.publish('battery.low')
  console.log(`power     handed battery.low to ${reached} parts`)
  const heaterTook = await heater.nextText()
  console.log(`heater    took ${heaterTook}`)
  const loggerTook = await logger.nextText()
  console.log(`logger    took ${loggerTook}`)

  // Publishing never waits, even while the part's own wait is open, and a part hears
  // what it publishes.
  const waiting = heater.nextText()
  heater.publish('heater.off')
  const heard = await waiting
  console.log(`heater    heard its own ${heard}, sent while it waited`)

  // A part that joins late sees only what is published after it subscribes. There is
  // no history to replay.
  const radio = bus.subscribe()
  power.publish('battery.ok')
  const first = await radio.nextText()
  console.log(`radio     joined late, so the first event it sees is ${first}`)

  // Each endpoint buffers two events. The logger, busy writing to flash, falls behind
  // while the sampler publishes five readings: it loses the oldest events, resumes with
  // the newest, and counts what it lost.
  for (let reading = 0; reading < 5; reading += 1) {
    sampler.publish(`wind ${reading}`)
  }
  const resumed = await logger.nextText()
  const missed = logger.missed
  console.log(`logger    missed ${missed} and resumes at ${resumed}`)
  const newest = await logger.nextText()
  console.log(`logger    then took ${newest}`)

  // A wait with a limit gives up without taking anything, so a part can do other work
  // between events and lose nothing by it.
  try {
    await logger.nextText(QUIET_MS)
    console.log('logger    took an event no one published, which should never happen')
  } catch {
    console.log(`logger    heard nothing more within ${QUIET_MS} ms`)
  }

  return { reached, heaterTook, loggerTook, heard, first, missed, resumed, newest }
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-bus`](https://crates.io/crates/pamoja-bus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html), [docs.rs](https://docs.rs/pamoja-bus), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-bus) |
| TypeScript | [`@pamoja/bus`](https://www.npmjs.com/package/@pamoja/bus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-bus) |
| Python | [`pamoja-bus`](https://pypi.org/project/pamoja-bus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-bus) |
| C# | [`Pamoja.Bus`](https://www.nuget.org/packages/Pamoja.Bus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-bus) |

## Documentation

- [`@pamoja/bus` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html), every class, function, and type this package exports.
- [The Event bus guide](https://pamoja.molex.cloud/docs/guides/bus.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
