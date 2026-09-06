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
