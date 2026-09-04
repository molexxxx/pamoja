// The telemetry guide example; see docs/guides/telemetry.md.

// ANCHOR: example
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
// ANCHOR_END: example
