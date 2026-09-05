using Pamoja.Telemetry;

using static Guides.Guide;

namespace Guides;

/// <summary>The telemetry guide example; see docs/guides/telemetry.md.</summary>
public static class TelemetryGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
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
        // ANCHOR_END: example

        Expect(reporter.Threshold == TelemetryLevel.Warn, "satellite raises the bar to Warn");
        Expect(tick is null, "routine detail does not travel on a metered link");
        Expect(reading is not null, "a reading still does");
        Expect(warned.Code == "battery.low", "and so does the warning after it");
        Expect(warned.Value == 0.18f, "carrying the measurement that triggered it");
        Expect(dearer is null, "the same reading is not worth a satellite hop");
        Expect(lost is not null, "a failure always is");
        Expect(counts.Emitted == 3, "three events went out");
        Expect(counts.Dropped == 2, "two were counted instead");
        Expect(reporter.Total == 5, "and every one of the five is accounted for");
        Expect(
            Reporter.ThresholdFor(LinkCost.Offline) == TelemetryLevel.Error,
            "an offline node records only failures");
    }
}
