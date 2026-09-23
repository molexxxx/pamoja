using Pamoja.Telemetry;

using static Guides.Guide;
using static System.FormattableString;

namespace Guides;

/// <summary>The telemetry guide example; see docs/guides/telemetry.md.</summary>
public static class TelemetryGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
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
        // ANCHOR_END: example

        Expect(reporter.Threshold == TelemetryLevel.Error, "no link raises the bar to Error");
        Expect(warned.Code == "battery.low", "the warning carried its code");
        Expect(snapshot.Emitted == 5, "five events passed the bar");
        Expect(snapshot.Dropped == 3, "three were counted instead");
        Expect(reporter.Total == 8, "and every one of the eight is accounted for");
    }
}
