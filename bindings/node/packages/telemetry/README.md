# @pamoja/telemetry

Observability that ships only what is worth the bytes as link cost rises, while counting everything. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/telemetry.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html)

## Install

```sh
npm install @pamoja/telemetry
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/telemetry.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/telemetry.ts):

```typescript
import { Level, LinkCost, Reporter, type TelemetryEvent } from '@pamoja/telemetry'

// What a node does with an event the reporter hands back: on a link it sends it, and with
// no link it keeps it for when one returns.
const fate = (event: TelemetryEvent | null, kept: string) => (event === null ? 'counted only' : kept)

// On the site's own network nothing is held back.
const reporter = new Reporter(Level.Trace)
reporter.adaptTo(LinkCost.Free)
let tick = reporter.record({ level: Level.Debug, code: 'loop.tick' })
console.log(`free      nothing is held back: loop.tick ${fate(tick, 'sent')}`)

// On a metered link the bar rises to Info. Routine detail stops going out; a reading and a
// warning still do, and a warning carries the measurement that raised it.
reporter.adaptTo(LinkCost.Metered)
tick = reporter.record({ level: Level.Debug, code: 'loop.tick' })
let reading = reporter.record({ level: Level.Info, code: 'reading.ok', value: 4.8 })
console.log(
  `metered   nothing below ${reporter.threshold} is sent: loop.tick ${fate(tick, 'sent')}, reading.ok ${fate(reading, 'sent')}`,
)
const warned = reporter.record({ level: Level.Warn, code: 'battery.low', value: 0.18 })!
console.log(`metered   ${warned.code} sent, carrying ${warned.value!.toFixed(2)}`)

// On satellite the bar is Warn: the same reading is no longer worth its bytes, and a
// failure still is.
reporter.adaptTo(LinkCost.Expensive)
reading = reporter.record({ level: Level.Info, code: 'reading.ok', value: 4.9 })
let lost = reporter.record({ level: Level.Error, code: 'link.lost' })
console.log(
  `satellite nothing below ${reporter.threshold} is sent: reading.ok ${fate(reading, 'sent')}, link.lost ${fate(lost, 'sent')}`,
)

// With no link at all only errors are kept, for the link's return.
reporter.adaptTo(LinkCost.Offline)
const low = reporter.record({ level: Level.Warn, code: 'battery.low', value: 0.17 })
lost = reporter.record({ level: Level.Error, code: 'link.lost' })
console.log(
  `offline   nothing below ${reporter.threshold} is kept: battery.low ${fate(low, 'kept')}, link.lost ${fate(lost, 'kept')}`,
)

// Only the stream was thinned, not the counts, so every event is still accounted for, and
// the snapshot is what the node ships in place of them.
const snapshot = reporter.snapshot()
console.log(
  `counts    of ${reporter.total} events, ${snapshot.emitted} passed the bar and ${snapshot.dropped} were counted only`,
)
console.log(
  `levels    trace ${snapshot.trace}, debug ${snapshot.debug}, info ${snapshot.info}, warn ${snapshot.warn}, error ${snapshot.error}`,
)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-telemetry`](https://crates.io/crates/pamoja-telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html), [docs.rs](https://docs.rs/pamoja-telemetry), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-telemetry) |
| TypeScript | [`@pamoja/telemetry`](https://www.npmjs.com/package/@pamoja/telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-telemetry) |
| Python | [`pamoja-telemetry`](https://pypi.org/project/pamoja-telemetry/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-telemetry) |
| C# | [`Pamoja.Telemetry`](https://www.nuget.org/packages/Pamoja.Telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-telemetry) |

## Documentation

- [`@pamoja/telemetry` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html), every class, function, and type this package exports.
- [The Telemetry guide](https://pamoja.molex.cloud/docs/guides/telemetry.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
