// The telemetry guide example; see docs/guides/telemetry.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
// ANCHOR_END: example

assert.equal(reporter.threshold, Level.Error)
assert.equal(warned.code, 'battery.low')
assert.equal(snapshot.emitted, 5)
assert.equal(snapshot.dropped, 3)
assert.equal(reporter.total, 8)
