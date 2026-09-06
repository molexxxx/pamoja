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

A node willing to record everything adapts to a metered link, then records a
loop tick, a sensor reading and the low-battery warning that follows it. It
falls back to satellite, records the same reading again and a lost-link
failure, and prints which of the five come back to be shipped. It ends on the
snapshot, the few integers a node ships in place of the events themselves.

The only level typed in is the `Trace` the reporter starts at, and it decides
nothing: the bar is the link's before the first event is recorded. `adapt_to`
takes a link cost and the library derives the level from it, so what a reader
sees is the price of the link rather than a number someone picked. The same
`reading.ok` is recorded under both costs, so what decides whether it travels
is the link and not the event.

It proves:

- A link cost sets the bar: `Metered` puts it at `Info` and `Expensive` raises
  it to `Warn`, so how much a node says follows what the link costs and not the
  level it was built with.
- The same `reading.ok` is handed back on the metered link and held back on the
  satellite one, so each event is judged against the bar in force when it is
  recorded.
- A shipped event comes back with its code and its measurement, `battery.low`
  at `0.18`, so a transport has the number that triggered it.
- A held-back event is still counted: two readings were recorded at `Info` even
  though only the first one went out.
- Five events recorded reconcile as three shipped and two dropped, so thinning
  the stream loses nothing from the totals.
- `Offline` is the last rung, holding back everything below `Error`, so a node
  with no link keeps its failures and nothing else.

## Rust

<!-- snippet: examples/tests/guides/telemetry.rs#example -->
From [`examples/tests/guides/telemetry.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/telemetry.rs):

```rust
use pamoja_telemetry::{Event, Level, LinkCost, Reporter};

// The node is willing to record everything, then finds out it is reporting over a
// metered link, which puts the bar at Info.
let mut reporter = Reporter::new(Level::Trace);
reporter.adapt_to(LinkCost::Metered);
let bar = reporter.threshold();
println!("on a metered link, nothing below {bar:?} is sent");

// Routine detail stops going out. A reading and the warning that follows it still do,
// and a shipped event comes back with the measurement that triggered it.
let tick = reporter.record(Event::debug("loop.tick"));
println!("loop.tick sent: {}", tick.is_some());
let reading = reporter.record(Event::info("reading.ok").with_value(4.8));
println!("reading.ok sent: {}", reading.is_some());
let warned = reporter
    .record(Event::warn("battery.low").with_value(0.18))
    .expect("a warning is worth a metered link");
let measured = warned.value.expect("the measurement that triggered it");
println!("sent      {} carrying {measured}", warned.code);

// The node falls back to satellite, which raises the bar to Warn. The same reading is
// no longer worth its bytes; a failure still is.
reporter.adapt_to(LinkCost::Expensive);
let dearer = reporter.record(Event::info("reading.ok").with_value(4.9));
let lost = reporter.record(Event::error("link.lost"));
println!("on satellite, reading.ok sent: {}", dearer.is_some());
println!("on satellite, link.lost sent: {}", lost.is_some());

// Only the stream was thinned, not the counts, so every event is still accounted for
// and the snapshot is what the node ships in place of them.
let counts = reporter.snapshot();
let (seen, sent, only_counted) = (reporter.total(), counts.emitted, counts.dropped);
println!("of {seen} events, {sent} went out and {only_counted} were counted only");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/telemetry.ts#example -->
From [`bindings/node/guides/telemetry.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/telemetry.ts):

```typescript
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
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/telemetry.py#example -->
From [`bindings/python/guides/telemetry.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/telemetry.py):

```python
from pamoja.telemetry import Event, Level, LinkCost, Reporter, link_cost_threshold

# The node is willing to record everything, then finds out it is reporting over a metered
# link, which puts the bar at INFO.
reporter = Reporter(Level.TRACE)
reporter.adapt_to(LinkCost.METERED)
print(f"on a metered link, nothing below {reporter.threshold.value} is sent")

# Routine detail stops going out. A reading and the warning that follows it still do, and
# a shipped event comes back with the measurement that triggered it.
tick = reporter.record(Event(Level.DEBUG, "loop.tick"))
reading = reporter.record(Event(Level.INFO, "reading.ok", 4.8))
print(f"loop.tick sent: {tick is not None}")
print(f"reading.ok sent: {reading is not None}")
warned = reporter.record(Event(Level.WARN, "battery.low", 0.18))
print(f"sent      {warned.code} carrying {warned.value}")

# The node falls back to satellite, which raises the bar to WARN. The same reading is no
# longer worth its bytes; a failure still is.
reporter.adapt_to(LinkCost.EXPENSIVE)
dearer = reporter.record(Event(Level.INFO, "reading.ok", 4.9))
lost = reporter.record(Event(Level.ERROR, "link.lost"))
print(f"on satellite, reading.ok sent: {dearer is not None}")
print(f"on satellite, link.lost sent: {lost is not None}")

# Only the stream was thinned, not the counts, so every event is still accounted for and
# the snapshot is what the node ships in place of them.
counts = reporter.snapshot()
print(f"of {reporter.total} events, {counts.emitted} went out and {counts.dropped} were counted only")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/TelemetryGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/TelemetryGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/TelemetryGuide.cs):

```csharp
// The node is willing to record everything, then finds out it is reporting over a
// metered link, which puts the bar at Info.
using var reporter = new Reporter(TelemetryLevel.Trace);
reporter.AdaptTo(LinkCost.Metered);
Console.WriteLine($"on a metered link, nothing below {reporter.Threshold} is sent");

// Routine detail stops going out. A reading and the warning that follows it still
// do, and a shipped event comes back with the measurement that triggered it.
TelemetryEvent? tick =
    reporter.Record(new TelemetryEvent(TelemetryLevel.Debug, "loop.tick"));
TelemetryEvent? reading =
    reporter.Record(new TelemetryEvent(TelemetryLevel.Info, "reading.ok", 4.8f));
Console.WriteLine($"loop.tick sent: {tick is not null}");
Console.WriteLine($"reading.ok sent: {reading is not null}");
TelemetryEvent warned =
    reporter.Record(new TelemetryEvent(TelemetryLevel.Warn, "battery.low", 0.18f))!.Value;
Console.WriteLine($"sent      {warned.Code} carrying {warned.Value}");

// The node falls back to satellite, which raises the bar to Warn. The same reading
// is no longer worth its bytes; a failure still is.
reporter.AdaptTo(LinkCost.Expensive);
TelemetryEvent? dearer =
    reporter.Record(new TelemetryEvent(TelemetryLevel.Info, "reading.ok", 4.9f));
TelemetryEvent? lost =
    reporter.Record(new TelemetryEvent(TelemetryLevel.Error, "link.lost"));
Console.WriteLine($"on satellite, reading.ok sent: {dearer is not null}");
Console.WriteLine($"on satellite, link.lost sent: {lost is not null}");

// Only the stream was thinned, not the counts, so every event is still accounted
// for and the snapshot is what the node ships in place of them.
TelemetrySnapshot counts = reporter.Snapshot();
Console.WriteLine(
    $"of {reporter.Total} events, {counts.Emitted} went out and {counts.Dropped}"
    + " were counted only");
```
<!-- end -->

## Reference

<!-- table: reference telemetry -->
- Rust: [`pamoja-telemetry`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-telemetry)
- TypeScript: [`@pamoja/telemetry`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-telemetry)
- Python: [`pamoja.telemetry`](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-telemetry)
- C#: [`Pamoja.Telemetry`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-telemetry)
<!-- end -->
