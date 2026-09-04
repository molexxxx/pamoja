# Telemetry

A node that ships every event it produces spends more on reporting than on the
job it was installed to do, and over a satellite link those bytes are money.
pamoja separates the detail a node records from the detail it sends: a reporter
holds a level threshold, hands back only the events at or above it, and counts
every event either way. The threshold follows what the link currently costs, so
the same code reports in full over ethernet and holds back everything but
failures when the node is on its own. It owns no transport, so where the events
it hands back actually go is the caller's business.

## What the example does

It takes one node through two changes of link cost, records five events across
the range of levels, and checks which of them come back to be shipped and what
the counters say once the dust settles.

It proves:

- A metered link sets the bar at `Info` and an expensive one at `Warn`, so how
  much a node says follows what the link costs rather than a build-time setting.
- An event that clears the bar comes back with its code and its measurement
  intact, ready to hand to a transport.
- A dropped event is still counted: two readings were recorded at `Info` even
  though only the first one went out.
- Five events recorded, three shipped and two dropped, so the totals reconcile
  and the snapshot of them is what the node sends in place of the stream.
- `Offline` is the last rung, holding back everything below `Error`.

## Rust

<!-- snippet: examples/tests/guides/telemetry.rs#example -->
From [`examples/tests/guides/telemetry.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/telemetry.rs):

```rust
use pamoja_telemetry::{Event, Level, LinkCost, Reporter};

// The node is willing to record everything, then finds out it is reporting over a
// metered link, which puts the bar at Info.
let mut reporter = Reporter::new(Level::Trace);
reporter.adapt_to(LinkCost::Metered);
assert_eq!(reporter.threshold(), Level::Info);

// Routine detail stops going out. A reading and the warning that follows it still do,
// and a shipped event comes back with the measurement that triggered it.
assert!(reporter.record(Event::debug("loop.tick")).is_none());
let reading = Event::info("reading.ok").with_value(4.8);
assert!(reporter.record(reading).is_some());
let warned = reporter
    .record(Event::warn("battery.low").with_value(0.18))
    .expect("a warning is worth a metered link");
assert_eq!(warned.code, "battery.low");
assert_eq!(warned.value, Some(0.18));

// The node falls back to satellite, which raises the bar to Warn. The same reading is
// no longer worth its bytes; a failure still is.
reporter.adapt_to(LinkCost::Expensive);
let reading = Event::info("reading.ok").with_value(4.9);
assert!(reporter.record(reading).is_none());
assert!(reporter.record(Event::error("link.lost")).is_some());

// Only the stream was thinned, not the counts, so all five events are still accounted
// for and the snapshot is what the node ships in place of them.
let counts = reporter.snapshot();
assert_eq!(counts.by_level[Level::Info as usize], 2);
assert_eq!(counts.emitted, 3);
assert_eq!(counts.dropped, 2);
assert_eq!(reporter.total(), 5);

// Offline is the last rung: a node with no link at all still keeps its failures.
assert_eq!(LinkCost::Offline.threshold(), Level::Error);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/telemetry.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/telemetry.py#example -->
From [`bindings/python/guides/telemetry.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/telemetry.py):

```python
from pamoja.telemetry import Event, Level, LinkCost, Reporter, link_cost_threshold

# The node is willing to record everything, then finds out it is reporting over a
# metered link, which puts the bar at INFO.
reporter = Reporter(Level.TRACE)
reporter.adapt_to(LinkCost.METERED)
assert reporter.threshold == Level.INFO

# Routine detail stops going out. A reading and the warning that follows it still do,
# and a shipped event comes back with the measurement that triggered it.
assert reporter.record(Event(Level.DEBUG, "loop.tick")) is None
assert reporter.record(Event(Level.INFO, "reading.ok", 4.8)) is not None
warned = reporter.record(Event(Level.WARN, "battery.low", 0.18))
assert warned is not None
assert warned.code == "battery.low"
assert warned.value == 0.18

# The node falls back to satellite, which raises the bar to WARN. The same reading is
# no longer worth its bytes; a failure still is.
reporter.adapt_to(LinkCost.EXPENSIVE)
assert reporter.record(Event(Level.INFO, "reading.ok", 4.9)) is None
assert reporter.record(Event(Level.ERROR, "link.lost")) is not None

# Only the stream was thinned, not the counts, so all five events are still accounted
# for and the snapshot is what the node ships in place of them.
counts = reporter.snapshot()
assert counts.info == 2
assert counts.emitted == 3
assert counts.dropped == 2
assert reporter.total == 5

# Offline is the last rung: a node with no link at all still keeps its failures.
assert link_cost_threshold(LinkCost.OFFLINE) == Level.ERROR
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/TelemetryGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/TelemetryGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/TelemetryGuide.cs):

```csharp
// The node is willing to record everything, then finds out it is reporting
// over a metered link, which puts the bar at Info.
using var reporter = new Reporter(TelemetryLevel.Trace);
reporter.AdaptTo(LinkCost.Metered);
Expect(reporter.Threshold == TelemetryLevel.Info, "a metered link ships from Info up");

// Routine detail stops going out. A reading and the warning that follows it
// still do, and a shipped event comes back with the measurement that
// triggered it.
Expect(
    reporter.Record(new TelemetryEvent(TelemetryLevel.Debug, "loop.tick")) is null,
    "routine detail is not worth a metered link");
Expect(
    reporter.Record(new TelemetryEvent(TelemetryLevel.Info, "reading.ok", 4.8f)) is not null,
    "a reading still goes out");
TelemetryEvent? warned =
    reporter.Record(new TelemetryEvent(TelemetryLevel.Warn, "battery.low", 0.18f));
Expect(warned?.Code == "battery.low", "and so does the warning that follows it");
Expect(warned?.Value == 0.18f, "carrying the measurement that triggered it");

// The node falls back to satellite, which raises the bar to Warn. The same
// reading is no longer worth its bytes; a failure still is.
reporter.AdaptTo(LinkCost.Expensive);
Expect(
    reporter.Record(new TelemetryEvent(TelemetryLevel.Info, "reading.ok", 4.9f)) is null,
    "the same reading is dropped on a satellite link");
Expect(
    reporter.Record(new TelemetryEvent(TelemetryLevel.Error, "link.lost")) is not null,
    "a failure is worth the bytes at any cost short of offline");

// Only the stream was thinned, not the counts, so all five events are still
// accounted for and the snapshot is what the node ships in place of them.
TelemetrySnapshot counts = reporter.Snapshot();
Expect(counts.Info == 2, "both readings were counted, though one never shipped");
Expect(counts.Emitted == 3, "three events went out");
Expect(counts.Dropped == 2, "two were held back");
Expect(reporter.Total == 5, "and every one of the five is accounted for");

// Offline is the last rung: a node with no link at all still keeps its failures.
Expect(
    Reporter.ThresholdFor(LinkCost.Offline) == TelemetryLevel.Error,
    "an offline node records only failures");
```
<!-- end -->

## Reference

<!-- table: reference telemetry -->
- Rust: [`pamoja-telemetry`](https://docs.rs/pamoja-telemetry) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html))
- TypeScript: [`@pamoja/telemetry`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html)
- Python: [`pamoja.telemetry`](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html)
- C#: [`Reporter`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.Reporter.html), [`TelemetryEvent`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.TelemetryEvent.html), [`TelemetryLevel`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.TelemetryLevel.html), [`TelemetrySnapshot`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.TelemetrySnapshot.html), [`LinkCost`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.LinkCost.html)
<!-- end -->
