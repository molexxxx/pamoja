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
        // ANCHOR_END: example
    }
}
