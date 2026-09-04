# Pamoja.Telemetry

Observability that ships only what is worth the bytes as link cost rises, while counting everything. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/telemetry.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Telemetry
```

```csharp
using Pamoja.Telemetry;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-telemetry`](https://crates.io/crates/pamoja-telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html), [docs.rs](https://docs.rs/pamoja-telemetry) |
| TypeScript | [`@pamoja/telemetry`](https://www.npmjs.com/package/@pamoja/telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html) |
| Python | [`pamoja-telemetry`](https://pypi.org/project/pamoja-telemetry/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html) |
| C# | [`Pamoja.Telemetry`](https://www.nuget.org/packages/Pamoja.Telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html) |

## Documentation

- [`Pamoja.Telemetry` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html), every type in this namespace.
- [The Telemetry guide](https://pamoja.molex.cloud/docs/guides/telemetry.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
