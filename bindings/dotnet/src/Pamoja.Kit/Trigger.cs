using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>What a <see cref="Trigger"/> reports when a reading changes its state.</summary>
public enum Edge
{
    /// <summary>The reading just crossed the line: the condition became true.</summary>
    Set = 1,

    /// <summary>The reading just came back past the release band: the condition stopped holding.</summary>
    Cleared = 2,
}

/// <summary>Fires once when a reading crosses a line, and not again until it has come back past the release band.</summary>
/// <remarks>A thermostat answers the same question as a level to hold; a trigger answers it as an event to act on, which is what a rule wants.</remarks>
public sealed class Trigger : IDisposable
{
    private readonly NativeHandle _handle;

    private Trigger(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_trigger_free, "trigger");
    }

    /// <summary>Creates a trigger that fires when a reading rises above the line and clears once it has fallen below the line by the hysteresis.</summary>
    /// <param name="threshold">The line a rising reading crosses.</param>
    /// <param name="hysteresis">How far below the line the reading must fall to clear.</param>
    /// <returns>The trigger, which starts cleared.</returns>
    public static Trigger Above(float threshold, float hysteresis) =>
        new(NativeMethods.pamoja_trigger_above(threshold, hysteresis));

    /// <summary>Creates a trigger that fires when a reading falls below the line and clears once it has risen above the line by the hysteresis.</summary>
    /// <param name="threshold">The line a falling reading crosses.</param>
    /// <param name="hysteresis">How far above the line the reading must rise to clear.</param>
    /// <returns>The trigger, which starts cleared.</returns>
    public static Trigger Below(float threshold, float hysteresis) =>
        new(NativeMethods.pamoja_trigger_below(threshold, hysteresis));

    /// <summary>Feeds a reading in and reports the edge it caused.</summary>
    /// <param name="reading">The latest measured value.</param>
    /// <returns><see cref="Edge.Set"/> or <see cref="Edge.Cleared"/> the moment the state changes, or <c>null</c> while nothing changed.</returns>
    public Edge? Update(float reading) =>
        _handle.Use(handle => NativeMethods.pamoja_trigger_update(handle, reading) switch
        {
            PamojaEdge.Set => Edge.Set,
            PamojaEdge.Cleared => Edge.Cleared,
            _ => (Edge?)null,
        });

    /// <summary>Gets whether the condition currently holds.</summary>
    public bool IsSet =>
        _handle.Use(NativeMethods.pamoja_trigger_is_set);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
