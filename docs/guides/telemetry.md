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

A node records the same kinds of events as its link gets dearer, from the site's own
network through a metered link and satellite to no link at all: a loop tick, a sensor
reading, a low-battery warning, and a lost link. At each rung it prints which events
the reporter hands back, then the counts it would ship in place of the events.

The only level typed in is the `Trace` the reporter starts at, and it decides
nothing: `adapt_to` takes a link cost and the library derives the bar from it, so
what a reader sees is the price of the link rather than a number someone picked. The
same `reading.ok` is recorded on the metered link and on satellite, and the same
`battery.low` on the metered link and offline, so what decides whether an event
travels is the link and not the event.

It proves:

- A free link ships everything, even a loop tick.
- `Metered` puts the bar at `Info`, so the tick is only counted while the reading goes
  out, and a warning that goes out carries its measurement, `battery.low` at `0.18`.
- `Expensive` raises the bar to `Warn`: the same reading is only counted, while a lost
  link still goes.
- `Offline` keeps only `Error`, for the link's return: the low battery is counted and
  the lost link kept.
- Eight events reconcile as five that passed the bar and three counted only, and the
  counts by level add up to the eight, so thinning the stream loses nothing from the
  totals.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example telemetry_guide" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example telemetry_guide</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- telemetry" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- telemetry</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/telemetry.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/telemetry.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- telemetry" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- telemetry</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-telemetry` is `no_std` and allocation-free. An `Event` is a `Level`
and a `&'static str` code, built with `Event::debug("loop.tick")` and its siblings,
with an optional measurement from `with_value`. A `Reporter` starts at a threshold,
`adapt_to` moves it to what a `LinkCost` calls for, and `record` returns
`Some(event)` for one to ship and `None` for one only counted. `count`, `total`,
`emitted`, and `dropped` read the counters, and `snapshot` gives all of them at once
as a `Snapshot`, the per-level counts in `by_level`. The counters hold at `u32::MAX`
rather than wrap, so a node that runs for years reports a full count, never a small
one.

<!-- snippet: examples/guides/telemetry.rs#example -->
From [`examples/guides/telemetry.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/telemetry.rs):

```rust
use pamoja_telemetry::{Event, Level, LinkCost, Reporter};

// What a node does with an event the reporter hands back: on a link it sends it, and
// with no link it keeps it for when one returns.
let fate = |event: &Option<Event>, kept: &'static str| {
    if event.is_some() {
        kept
    } else {
        "counted only"
    }
};

// On the site's own network nothing is held back.
let mut reporter = Reporter::new(Level::Trace);
reporter.adapt_to(LinkCost::Free);
let tick = reporter.record(Event::debug("loop.tick"));
println!(
    "free      nothing is held back: loop.tick {}",
    fate(&tick, "sent")
);

// On a metered link the bar rises to Info. Routine detail stops going out; a reading
// and a warning still do, and a warning carries the measurement that raised it.
reporter.adapt_to(LinkCost::Metered);
let bar = reporter.threshold();
let tick = reporter.record(Event::debug("loop.tick"));
let reading = reporter.record(Event::info("reading.ok").with_value(4.8));
println!(
    "metered   nothing below {bar:?} is sent: loop.tick {}, reading.ok {}",
    fate(&tick, "sent"),
    fate(&reading, "sent")
);
let warned = reporter
    .record(Event::warn("battery.low").with_value(0.18))
    .expect("a warning is worth a metered link");
let measured = warned.value.expect("the measurement that raised it");
println!("metered   {} sent, carrying {measured:.2}", warned.code);

// On satellite the bar is Warn: the same reading is no longer worth its bytes, and a
// failure still is.
reporter.adapt_to(LinkCost::Expensive);
let bar = reporter.threshold();
let reading = reporter.record(Event::info("reading.ok").with_value(4.9));
let lost = reporter.record(Event::error("link.lost"));
println!(
    "satellite nothing below {bar:?} is sent: reading.ok {}, link.lost {}",
    fate(&reading, "sent"),
    fate(&lost, "sent")
);

// With no link at all only errors are kept, for the link's return.
reporter.adapt_to(LinkCost::Offline);
let bar = reporter.threshold();
let low = reporter.record(Event::warn("battery.low").with_value(0.17));
let lost = reporter.record(Event::error("link.lost"));
println!(
    "offline   nothing below {bar:?} is kept: battery.low {}, link.lost {}",
    fate(&low, "kept"),
    fate(&lost, "kept")
);

// Only the stream was thinned, not the counts, so every event is still accounted for,
// and the snapshot is what the node ships in place of them.
let snapshot = reporter.snapshot();
println!(
    "counts    of {} events, {} passed the bar and {} were counted only",
    reporter.total(),
    snapshot.emitted,
    snapshot.dropped
);
let at = |level: Level| snapshot.by_level[level as usize];
println!(
    "levels    trace {}, debug {}, info {}, warn {}, error {}",
    at(Level::Trace),
    at(Level::Debug),
    at(Level::Info),
    at(Level::Warn),
    at(Level::Error)
);
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/telemetry` exports `Reporter`, `Level`, and `LinkCost`. An
event is a plain object, `{ level, code, value? }`, and `record` returns it to ship or
`null`. `threshold` is a property that can also be set directly, `adaptTo(cost)`
derives it from the link, and `total`, `emitted`, and `dropped` are properties;
`count(level)` and `snapshot()`, with its per-level counts named `trace` through
`error`, are methods. `linkCostThreshold(cost)` answers what a cost calls for without
a reporter.

<!-- snippet: bindings/node/guides/telemetry.ts#example -->
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
<!-- end -->

## Python

In Python, `pamoja.telemetry` exports `Reporter`, `Event`, `Level`, and `LinkCost`,
the last two string enums. `Event(level, code, value=None)` is recorded with
`record`, which returns it to ship or `None`. A reporter starts at `Info` unless told
otherwise; `threshold` is a settable property, `adapt_to(cost)` derives it from the
link, and `total`, `emitted`, and `dropped` are properties. `snapshot()` gives the
counts by level as attributes, `trace` through `error`, and `link_cost_threshold(cost)`
answers what a cost calls for.

<!-- snippet: bindings/python/guides/telemetry.py#example -->
From [`bindings/python/guides/telemetry.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/telemetry.py):

```python
from pamoja.telemetry import Event, Level, LinkCost, Reporter


def fate(event: Event | None, kept: str) -> str:
    """What a node does with an event the reporter hands back: on a link it sends it, and
    with no link it keeps it for when one returns."""
    return "counted only" if event is None else kept


# On the site's own network nothing is held back.
reporter = Reporter(Level.TRACE)
reporter.adapt_to(LinkCost.FREE)
tick = reporter.record(Event(Level.DEBUG, "loop.tick"))
print(f"free      nothing is held back: loop.tick {fate(tick, 'sent')}")

# On a metered link the bar rises to Info. Routine detail stops going out; a reading and a
# warning still do, and a warning carries the measurement that raised it.
reporter.adapt_to(LinkCost.METERED)
tick = reporter.record(Event(Level.DEBUG, "loop.tick"))
reading = reporter.record(Event(Level.INFO, "reading.ok", 4.8))
print(
    f"metered   nothing below {reporter.threshold.value} is sent: "
    f"loop.tick {fate(tick, 'sent')}, reading.ok {fate(reading, 'sent')}"
)
warned = reporter.record(Event(Level.WARN, "battery.low", 0.18))
print(f"metered   {warned.code} sent, carrying {warned.value:.2f}")

# On satellite the bar is Warn: the same reading is no longer worth its bytes, and a
# failure still is.
reporter.adapt_to(LinkCost.EXPENSIVE)
reading = reporter.record(Event(Level.INFO, "reading.ok", 4.9))
lost = reporter.record(Event(Level.ERROR, "link.lost"))
print(
    f"satellite nothing below {reporter.threshold.value} is sent: "
    f"reading.ok {fate(reading, 'sent')}, link.lost {fate(lost, 'sent')}"
)

# With no link at all only errors are kept, for the link's return.
reporter.adapt_to(LinkCost.OFFLINE)
low = reporter.record(Event(Level.WARN, "battery.low", 0.17))
lost = reporter.record(Event(Level.ERROR, "link.lost"))
print(
    f"offline   nothing below {reporter.threshold.value} is kept: "
    f"battery.low {fate(low, 'kept')}, link.lost {fate(lost, 'kept')}"
)

# Only the stream was thinned, not the counts, so every event is still accounted for, and
# the snapshot is what the node ships in place of them.
snapshot = reporter.snapshot()
print(
    f"counts    of {reporter.total} events, {snapshot.emitted} passed the bar and "
    f"{snapshot.dropped} were counted only"
)
print(
    f"levels    trace {snapshot.trace}, debug {snapshot.debug}, info {snapshot.info}, "
    f"warn {snapshot.warn}, error {snapshot.error}"
)
```
<!-- end -->

## C#

In C#, `Pamoja.Telemetry` holds `Reporter`, which starts at `TelemetryLevel.Info`
unless told otherwise and belongs in a `using`. A `TelemetryEvent` record carries the
level, the code, and an optional value, and `Record` returns it to ship or `null`.
`Threshold` can be read and set, `AdaptTo(cost)` derives it from the link, `Total`,
`Emitted`, and `Dropped` are properties, and `Snapshot()` returns a
`TelemetrySnapshot` with the counts by level. `Reporter.ThresholdFor(cost)` answers
what a cost calls for.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/TelemetryGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/TelemetryGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/TelemetryGuide.cs):

```csharp
// What a node does with an event the reporter hands back: on a link it sends it,
// and with no link it keeps it for when one returns.
static string Fate(TelemetryEvent? evt, string kept) => evt is null ? "counted only" : kept;

// On the site's own network nothing is held back.
using var reporter = new Reporter(TelemetryLevel.Trace);
reporter.AdaptTo(LinkCost.Free);
TelemetryEvent? tick = reporter.Record(new TelemetryEvent(TelemetryLevel.Debug, "loop.tick"));
Console.WriteLine($"free      nothing is held back: loop.tick {Fate(tick, "sent")}");

// On a metered link the bar rises to Info. Routine detail stops going out; a
// reading and a warning still do, and a warning carries the measurement that
// raised it.
reporter.AdaptTo(LinkCost.Metered);
tick = reporter.Record(new TelemetryEvent(TelemetryLevel.Debug, "loop.tick"));
TelemetryEvent? reading =
    reporter.Record(new TelemetryEvent(TelemetryLevel.Info, "reading.ok", 4.8f));
Console.WriteLine(
    $"metered   nothing below {reporter.Threshold} is sent: loop.tick {Fate(tick, "sent")}, reading.ok {Fate(reading, "sent")}");
TelemetryEvent warned =
    reporter.Record(new TelemetryEvent(TelemetryLevel.Warn, "battery.low", 0.18f))!.Value;
Console.WriteLine(Invariant($"metered   {warned.Code} sent, carrying {warned.Value:F2}"));

// On satellite the bar is Warn: the same reading is no longer worth its bytes, and
// a failure still is.
reporter.AdaptTo(LinkCost.Expensive);
reading = reporter.Record(new TelemetryEvent(TelemetryLevel.Info, "reading.ok", 4.9f));
TelemetryEvent? lost = reporter.Record(new TelemetryEvent(TelemetryLevel.Error, "link.lost"));
Console.WriteLine(
    $"satellite nothing below {reporter.Threshold} is sent: reading.ok {Fate(reading, "sent")}, link.lost {Fate(lost, "sent")}");

// With no link at all only errors are kept, for the link's return.
reporter.AdaptTo(LinkCost.Offline);
TelemetryEvent? low = reporter.Record(new TelemetryEvent(TelemetryLevel.Warn, "battery.low", 0.17f));
lost = reporter.Record(new TelemetryEvent(TelemetryLevel.Error, "link.lost"));
Console.WriteLine(
    $"offline   nothing below {reporter.Threshold} is kept: battery.low {Fate(low, "kept")}, link.lost {Fate(lost, "kept")}");

// Only the stream was thinned, not the counts, so every event is still accounted
// for, and the snapshot is what the node ships in place of them.
TelemetrySnapshot snapshot = reporter.Snapshot();
Console.WriteLine(
    $"counts    of {reporter.Total} events, {snapshot.Emitted} passed the bar and {snapshot.Dropped} were counted only");
Console.WriteLine(
    $"levels    trace {snapshot.Trace}, debug {snapshot.Debug}, info {snapshot.Info}, warn {snapshot.Warn}, error {snapshot.Error}");
```
<!-- end -->

## Values at a glance

**The levels,** lowest first. A reporter ships an event at or above its threshold:

| Level | Meant for |
| --- | --- |
| Trace | step-by-step detail while developing |
| Debug | routine detail, such as a loop tick |
| Info | a normal reading or state change |
| Warn | something worth attention, such as a low battery |
| Error | a failure, such as a lost link |

**The link costs,** and the bar each one sets:

| Link cost | Bar | What still travels |
| --- | --- | --- |
| Free | Trace | everything |
| Metered | Info | readings, warnings, and failures |
| Expensive | Warn | warnings and failures |
| Offline | Error | failures, held for the link's return |

**What a snapshot holds:**

| Field | What it is |
| --- | --- |
| by level | how many events were recorded at each of the five levels, shipped or not |
| emitted | how many passed the bar |
| dropped | how many were counted only |

Every count stops at 4,294,967,295 rather than wrapping to zero.

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| make an event | `Event::info("reading.ok")`, and `trace`, `debug`, `warn`, `error`; `with_value(v)` |
| start a reporter | `Reporter::new(level)` |
| follow the link | `adapt_to(cost)`; `threshold()`, `set_threshold(level)` |
| record | `record(event)` gives `Some(event)` to ship or `None` |
| read the counts | `count(level)`, `total()`, `emitted()`, `dropped()`, `snapshot()` |
| the bar for a cost | `LinkCost::Metered.threshold()` |

### TypeScript

| To | Call |
| --- | --- |
| make an event | `{ level: Level.Info, code: 'reading.ok', value?: 4.8 }` |
| start a reporter | `new Reporter(level)` |
| follow the link | `adaptTo(cost)`; `threshold`, which can be set |
| record | `record(event)` gives the event to ship or `null` |
| read the counts | `count(level)`, `total`, `emitted`, `dropped`, `snapshot()` |
| the bar for a cost | `linkCostThreshold(LinkCost.Metered)` |

### Python

| To | Call |
| --- | --- |
| make an event | `Event(Level.INFO, "reading.ok", 4.8)` |
| start a reporter | `Reporter(level=Level.INFO)` |
| follow the link | `adapt_to(cost)`; `threshold`, which can be set |
| record | `record(event)` gives the event to ship or `None` |
| read the counts | `count(level)`, `total`, `emitted`, `dropped`, `snapshot()` |
| the bar for a cost | `link_cost_threshold(LinkCost.METERED)` |

### C#

| To | Call |
| --- | --- |
| make an event | `new TelemetryEvent(TelemetryLevel.Info, "reading.ok", 4.8f)` |
| start a reporter | `new Reporter(level = TelemetryLevel.Info)` |
| follow the link | `AdaptTo(cost)`; `Threshold`, which can be set |
| record | `Record(event)` gives the event to ship or `null` |
| read the counts | `Count(level)`, `Total`, `Emitted`, `Dropped`, `Snapshot()` |
| the bar for a cost | `Reporter.ThresholdFor(LinkCost.Metered)` |

<!-- languages end -->

## When it goes wrong

A reporter refuses nothing: every event is recorded and counted, and the bar only
decides which ones it hands back. The mistakes that cost an afternoon:

- **Nothing arrives, though events were recorded.** The reporter hands an event back;
  it does not send it. Send what `record` returns over the node's link, or, with no link,
  keep it in a store until one returns.
- **Every event arrives on a dear link.** The bar follows the link only when told. Call
  `adapt_to` whenever the link changes, such as when a transport ladder falls back to a
  dearer rung, or the reporter keeps the bar it started with.
- **The totals look wrong after a restart.** Counts live in the reporter, not in storage,
  so a restarted node counts from zero. Ship a snapshot before a planned restart, or let
  the server add each boot's counts to the last.
- **A dashboard shows a small count on an old node.** Counts stop at their ceiling rather
  than wrap, so a count that stops growing at 4,294,967,295 is full. Ship snapshots often
  enough that the server holds the running total.
- **A warning arrives without its reading.** A measurement travels only if the event
  carries it: record `battery.low` with the reading as its value, as the example does.

## Where next

<!-- table: next telemetry -->
- [Power](power.md): Duty cycling and an energy-aware governor that stretches work as the battery drains and holds its mode against a wandering charge.
- [MQTT](mqtt.md): An MQTT client with the topic and wildcard rules, acknowledged delivery, retained messages, a last will, and TLS, as the core transport.
- [Rules](rules.md): Rules between nodes as a file.
- Also in Trust and operation: [Audit log](audit.md), [Secured session](session.md), [Signed updates](update.md).
<!-- end -->

## Reference

<!-- table: reference telemetry -->
- Rust: [`pamoja-telemetry`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-telemetry)
- TypeScript: [`@pamoja/telemetry`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-telemetry)
- Python: [`pamoja.telemetry`](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-telemetry)
- C#: [`Pamoja.Telemetry`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-telemetry)
<!-- end -->
