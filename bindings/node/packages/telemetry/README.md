# @pamoja/telemetry

Observability that ships only what is worth the bytes as link cost rises, while counting everything. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/telemetry.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/telemetry
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/telemetry.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/telemetry.ts):

```typescript
import assert from 'node:assert/strict'

import { Level, LinkCost, Reporter, linkCostThreshold } from '@pamoja/telemetry'

// The node is willing to record everything, then finds out it is reporting over a metered
// link, which puts the bar at Info.
const reporter = new Reporter(Level.Trace)
reporter.adaptTo(LinkCost.Metered)
assert.equal(reporter.threshold, Level.Info)

// Routine detail stops going out. A reading and the warning that follows it still do, and
// a shipped event comes back with the measurement that triggered it.
assert.equal(reporter.record({ level: Level.Debug, code: 'loop.tick' }), null)
assert.notEqual(reporter.record({ level: Level.Info, code: 'reading.ok', value: 4.8 }), null)
const warned = reporter.record({ level: Level.Warn, code: 'battery.low', value: 0.18 })
assert.equal(warned?.code, 'battery.low')
assert.equal(warned?.value, 0.18)

// The node falls back to satellite, which raises the bar to Warn. The same reading is no
// longer worth its bytes; a failure still is.
reporter.adaptTo(LinkCost.Expensive)
assert.equal(reporter.record({ level: Level.Info, code: 'reading.ok', value: 4.9 }), null)
assert.notEqual(reporter.record({ level: Level.Error, code: 'link.lost' }), null)

// Only the stream was thinned, not the counts, so all five events are still accounted for
// and the snapshot is what the node ships in place of them.
const counts = reporter.snapshot()
assert.equal(counts.info, 2)
assert.equal(counts.emitted, 3)
assert.equal(counts.dropped, 2)
assert.equal(reporter.total, 5)

// Offline is the last rung: a node with no link at all still keeps its failures.
assert.equal(linkCostThreshold(LinkCost.Offline), Level.Error)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-telemetry`](https://crates.io/crates/pamoja-telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html), [docs.rs](https://docs.rs/pamoja-telemetry) |
| TypeScript | [`@pamoja/telemetry`](https://www.npmjs.com/package/@pamoja/telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html) |
| Python | [`pamoja-telemetry`](https://pypi.org/project/pamoja-telemetry/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html) |
| C# | [`Pamoja.Telemetry`](https://www.nuget.org/packages/Pamoja.Telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html) |

## Documentation

- [`@pamoja/telemetry` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html), every class, function, and type this package exports.
- [The Telemetry guide](https://pamoja.molex.cloud/docs/guides/telemetry.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
