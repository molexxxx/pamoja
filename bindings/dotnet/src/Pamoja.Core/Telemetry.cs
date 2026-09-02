using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>How urgent a telemetry event is.</summary>
public enum TelemetryLevel
{
    /// <summary>Fine-grained detail, useful only when chasing a specific problem.</summary>
    Trace = 0,

    /// <summary>Diagnostic detail for development.</summary>
    Debug = 1,

    /// <summary>A normal, noteworthy event.</summary>
    Info = 2,

    /// <summary>Something unexpected that the node recovered from.</summary>
    Warn = 3,

    /// <summary>A failure that needs attention.</summary>
    Error = 4,
}

/// <summary>What the link back to the network currently costs.</summary>
public enum LinkCost
{
    /// <summary>Bytes are effectively free, such as on wired power and ethernet.</summary>
    Free = 0,

    /// <summary>Bytes are paid for, such as on a cellular plan.</summary>
    Metered = 1,

    /// <summary>Bytes are scarce, such as on a satellite or long-range radio link.</summary>
    Expensive = 2,

    /// <summary>Nothing can be shipped at all.</summary>
    Offline = 3,
}

/// <summary>A structured telemetry event.</summary>
/// <remarks>
/// The code is a stable, short label such as <c>battery.low</c> rather than a
/// free-form message, so events stay small and group cleanly into counts.
/// </remarks>
/// <param name="Level">How urgent the event is.</param>
/// <param name="Code">A stable, short identifier for what happened.</param>
/// <param name="Value">An optional measurement, such as the charge that triggered it.</param>
public readonly record struct TelemetryEvent(
    TelemetryLevel Level,
    string Code,
    float? Value = null);

/// <summary>A count of everything a reporter has seen.</summary>
/// <param name="Trace">How many events were seen at trace level.</param>
/// <param name="Debug">How many events were seen at debug level.</param>
/// <param name="Info">How many events were seen at info level.</param>
/// <param name="Warn">How many events were seen at warn level.</param>
/// <param name="Error">How many events were seen at error level.</param>
/// <param name="Emitted">How many events passed the filter and were shipped.</param>
/// <param name="Dropped">How many events the filter dropped.</param>
public readonly record struct TelemetrySnapshot(
    uint Trace,
    uint Debug,
    uint Info,
    uint Warn,
    uint Error,
    uint Emitted,
    uint Dropped);

/// <summary>Records events, ships the ones worth their bytes, and counts them all.</summary>
/// <remarks>
/// A node that ships every event it produces will spend more on reporting than on
/// the job it was installed to do, and on a satellite link that is money. The
/// reporter keeps counting what it drops, so the aggregate picture survives even
/// when the detail cannot be sent.
/// </remarks>
public sealed class Reporter : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a reporter that ships events at or above a level.</summary>
    /// <param name="threshold">The lowest level to ship.</param>
    /// <exception cref="PamojaException">The native reporter could not be created.</exception>
    public Reporter(TelemetryLevel threshold = TelemetryLevel.Info)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_reporter_new((PamojaTelemetryLevel)threshold),
            NativeMethods.pamoja_reporter_free,
            "reporter");
    }

    /// <summary>Gets or sets the level this reporter is shipping from.</summary>
    public TelemetryLevel Threshold
    {
        get => (TelemetryLevel)_handle.Use(NativeMethods.pamoja_reporter_threshold);
        set => _handle.Use(handle =>
        {
            NativeMethods.pamoja_reporter_set_threshold(handle, (PamojaTelemetryLevel)value);
            return 0;
        });
    }

    /// <summary>Gets how many events have been seen across every level.</summary>
    public uint Total => _handle.Use(NativeMethods.pamoja_reporter_total);

    /// <summary>Gets how many events passed the threshold and were shipped.</summary>
    public uint Emitted => _handle.Use(NativeMethods.pamoja_reporter_emitted);

    /// <summary>Gets how many events the threshold dropped.</summary>
    public uint Dropped => _handle.Use(NativeMethods.pamoja_reporter_dropped);

    /// <summary>Returns the level a link cost calls for.</summary>
    /// <param name="cost">What the link currently costs.</param>
    /// <returns>The lowest level still worth its bytes at that cost.</returns>
    public static TelemetryLevel ThresholdFor(LinkCost cost) =>
        (TelemetryLevel)NativeMethods.pamoja_link_cost_threshold((PamojaLinkCost)cost);

    /// <summary>Moves the threshold to match what the link now costs.</summary>
    /// <param name="cost">What the link currently costs.</param>
    public void AdaptTo(LinkCost cost) => _handle.Use(handle =>
    {
        NativeMethods.pamoja_reporter_adapt_to(handle, (PamojaLinkCost)cost);
        return 0;
    });

    /// <summary>Records an event, returning it when it should be shipped.</summary>
    /// <remarks>
    /// Only the level reaches the native reporter, because the level is the whole
    /// of what it decides on. The event comes straight back when it passed.
    /// </remarks>
    /// <param name="telemetryEvent">The event that occurred.</param>
    /// <returns>
    /// The same event when it passed the threshold, or <c>null</c> when it was
    /// counted and dropped.
    /// </returns>
    public TelemetryEvent? Record(TelemetryEvent telemetryEvent)
    {
        bool shipped = _handle.Use(handle => NativeMethods.pamoja_reporter_record(
            handle, (PamojaTelemetryLevel)telemetryEvent.Level));
        return shipped ? telemetryEvent : null;
    }

    /// <summary>Returns how many events have been seen at a level, shipped or not.</summary>
    /// <param name="level">The level to count.</param>
    /// <returns>The number of events recorded at that level.</returns>
    public uint Count(TelemetryLevel level) => _handle.Use(handle =>
        NativeMethods.pamoja_reporter_count(handle, (PamojaTelemetryLevel)level));

    /// <summary>Takes a snapshot of the counters to ship in place of the stream.</summary>
    /// <returns>The per-level counts and the shipped and dropped totals.</returns>
    public TelemetrySnapshot Snapshot()
    {
        PamojaTelemetrySnapshot snapshot =
            _handle.Use(NativeMethods.pamoja_reporter_snapshot);
        PamojaLevelCounts counts = snapshot.ByLevel;
        return new TelemetrySnapshot(
            counts[(int)TelemetryLevel.Trace],
            counts[(int)TelemetryLevel.Debug],
            counts[(int)TelemetryLevel.Info],
            counts[(int)TelemetryLevel.Warn],
            counts[(int)TelemetryLevel.Error],
            snapshot.Emitted,
            snapshot.Dropped);
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
