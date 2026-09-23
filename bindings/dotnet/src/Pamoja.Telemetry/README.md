# Pamoja.Telemetry

Observability that ships only what is worth the bytes as link cost rises, while counting everything. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/telemetry.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html)

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-telemetry`](https://crates.io/crates/pamoja-telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html), [docs.rs](https://docs.rs/pamoja-telemetry), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-telemetry) |
| TypeScript | [`@pamoja/telemetry`](https://www.npmjs.com/package/@pamoja/telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-telemetry) |
| Python | [`pamoja-telemetry`](https://pypi.org/project/pamoja-telemetry/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-telemetry) |
| C# | [`Pamoja.Telemetry`](https://www.nuget.org/packages/Pamoja.Telemetry) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-telemetry) |

## Documentation

- [`Pamoja.Telemetry` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Telemetry.html), every type in this namespace.
- [The Telemetry guide](https://pamoja.molex.cloud/docs/guides/telemetry.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
