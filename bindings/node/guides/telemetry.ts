// The telemetry guide example; see docs/guides/telemetry.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Level, LinkCost, Reporter, linkCostThreshold } from '@pamoja/telemetry'

// The node is willing to record everything, then finds out it is reporting over a metered
// link, which puts the bar at Info.
const reporter = new Reporter(Level.Trace)
reporter.adaptTo(LinkCost.Metered)
console.log(`on a metered link, nothing below ${reporter.threshold} is sent`)

// Routine detail stops going out. A reading and the warning that follows it still do, and
// a shipped event comes back with the measurement that triggered it.
const tick = reporter.record({ level: Level.Debug, code: 'loop.tick' })
const reading = reporter.record({ level: Level.Info, code: 'reading.ok', value: 4.8 })
console.log(`loop.tick sent: ${tick !== null}`)
console.log(`reading.ok sent: ${reading !== null}`)
const warned = reporter.record({ level: Level.Warn, code: 'battery.low', value: 0.18 })!
console.log(`sent      ${warned.code} carrying ${warned.value}`)

// The node falls back to satellite, which raises the bar to Warn. The same reading is no
// longer worth its bytes; a failure still is.
reporter.adaptTo(LinkCost.Expensive)
const dearer = reporter.record({ level: Level.Info, code: 'reading.ok', value: 4.9 })
const lost = reporter.record({ level: Level.Error, code: 'link.lost' })
console.log(`on satellite, reading.ok sent: ${dearer !== null}`)
console.log(`on satellite, link.lost sent: ${lost !== null}`)

// Only the stream was thinned, not the counts, so every event is still accounted for and
// the snapshot is what the node ships in place of them.
const counts = reporter.snapshot()
console.log(
  `of ${reporter.total} events, ${counts.emitted} went out and ${counts.dropped} were counted only`,
)
// ANCHOR_END: example

assert.equal(reporter.threshold, Level.Warn)
assert.equal(tick, null)
assert.notEqual(reading, null)
assert.equal(warned.code, 'battery.low')
assert.equal(warned.value, 0.18)
assert.equal(dearer, null)
assert.notEqual(lost, null)
assert.equal(counts.emitted, 3)
assert.equal(counts.dropped, 2)
assert.equal(reporter.total, 5)

// Offline is the last rung: a node with no link at all still keeps its failures.
assert.equal(linkCostThreshold(LinkCost.Offline), Level.Error)
