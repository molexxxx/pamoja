using System.Globalization;
using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>A rolling window of the most recent readings, with the stats over them.</summary>
/// <remarks>
/// The core helpers are generic over their capacity, which cannot cross the C ABI,
/// so these are built with room for <see cref="Kit.WindowCapacity"/> readings
/// and keep fewer when asked. A reading that is not a finite number is not kept.
/// </remarks>
public sealed class Window : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an empty window.</summary>
    /// <param name="capacity">How many readings to keep, from 1 to 32; 32 unless told fewer.</param>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="capacity"/> is not from 1 to 32.</exception>
    public Window(int capacity = Kit.WindowCapacity)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_window_with_capacity(Capacities.Check(capacity, 1)),
            NativeMethods.pamoja_window_free,
            "window",
            serialized: true);
    }

    /// <summary>How many readings the window holds.</summary>
    public int Count => checked((int)_handle.Use(NativeMethods.pamoja_window_len));

    /// <summary>How many readings it holds before it starts dropping the oldest.</summary>
    public int Capacity => checked((int)_handle.Use(NativeMethods.pamoja_window_capacity));

    /// <summary>Whether the window holds as many readings as it keeps.</summary>
    public bool IsFull => _handle.Use(NativeMethods.pamoja_window_is_full);

    /// <summary>Adds a reading, dropping the oldest once the window is full.</summary>
    /// <param name="reading">The reading to add. One that is not a finite number is not kept.</param>
    public void Push(float reading) =>
        _handle.Use(handle => NativeMethods.pamoja_window_push(handle, reading));

    /// <summary>The most recent reading, or <c>null</c> while the window is empty.</summary>
    /// <returns>The latest reading, if there is one.</returns>
    public float? Latest() => _handle.UseTry<float>(NativeMethods.pamoja_window_latest);

    /// <summary>The oldest reading still held, or <c>null</c> while the window is empty.</summary>
    /// <returns>The oldest reading, if there is one.</returns>
    public float? Oldest() => _handle.UseTry<float>(NativeMethods.pamoja_window_oldest);

    /// <summary>The mean of the readings, or <c>null</c> while the window is empty.</summary>
    /// <returns>The mean, if there is one.</returns>
    public float? Mean() => _handle.UseTry<float>(NativeMethods.pamoja_window_mean);

    /// <summary>The smallest reading, or <c>null</c> while the window is empty.</summary>
    /// <returns>The minimum, if there is one.</returns>
    public float? Min() => _handle.UseTry<float>(NativeMethods.pamoja_window_min);

    /// <summary>The largest reading, or <c>null</c> while the window is empty.</summary>
    /// <returns>The maximum, if there is one.</returns>
    public float? Max() => _handle.UseTry<float>(NativeMethods.pamoja_window_max);

    /// <summary>The spread between the smallest and largest readings, or <c>null</c> while empty.</summary>
    /// <returns>The range, if there is one.</returns>
    public float? Range() => _handle.UseTry<float>(NativeMethods.pamoja_window_range);

    /// <summary>The population variance of the readings, 0 for one reading, or <c>null</c> while empty.</summary>
    /// <returns>The variance, if there is one.</returns>
    public float? Variance() => _handle.UseTry<float>(NativeMethods.pamoja_window_variance);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Rejects a single wild reading, where an average would let it pull the answer.</summary>
public sealed class Median : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an empty median filter.</summary>
    /// <param name="capacity">
    /// How many readings to take the median over, from 1 to 32; 32 unless told fewer. A
    /// small odd window, such as 5, follows a real change in a few readings; a window of
    /// 32 follows it 16 readings late.
    /// </param>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="capacity"/> is not from 1 to 32.</exception>
    public Median(int capacity = Kit.WindowCapacity)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_median_with_capacity(Capacities.Check(capacity, 1)),
            NativeMethods.pamoja_median_free,
            "median filter",
            serialized: true);
    }

    /// <summary>The current median, or <c>null</c> before the first reading.</summary>
    public float? Value => _handle.UseTry<float>(NativeMethods.pamoja_median_value);

    /// <summary>How many readings the filter keeps.</summary>
    public int Capacity => checked((int)_handle.Use(NativeMethods.pamoja_median_capacity));

    /// <summary>Folds a reading in and returns the median of the window.</summary>
    /// <param name="reading">The reading to add. One that is not a finite number is not kept.</param>
    /// <returns>The median across the window.</returns>
    public float Update(float reading) =>
        _handle.Use(handle => NativeMethods.pamoja_median_update(handle, reading));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Fits a line through recent readings, so a slow drift shows before it matters.</summary>
public sealed class Trend : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an empty trend estimator.</summary>
    /// <param name="capacity">
    /// How many readings to fit the line through, from 2 to 32, since a line needs two;
    /// 32 unless told fewer.
    /// </param>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="capacity"/> is not from 2 to 32.</exception>
    public Trend(int capacity = Kit.WindowCapacity)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_trend_with_capacity(Capacities.Check(capacity, 2)),
            NativeMethods.pamoja_trend_free,
            "trend estimator",
            serialized: true);
    }

    /// <summary>
    /// The fitted slope in units per reading, or <c>null</c> without two readings. A
    /// positive slope is a rising signal.
    /// </summary>
    public float? Slope => _handle.UseTry<float>(NativeMethods.pamoja_trend_slope);

    /// <summary>How many readings the estimator keeps.</summary>
    public int Capacity => checked((int)_handle.Use(NativeMethods.pamoja_trend_capacity));

    /// <summary>Adds a reading.</summary>
    /// <param name="reading">The reading to add. One that is not a finite number is not kept.</param>
    public void Push(float reading) =>
        _handle.Use(handle => NativeMethods.pamoja_trend_push(handle, reading));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Flags a reading that stands out from the ones before it.</summary>
public sealed class Anomaly : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a detector that flags a reading so many deviations out.</summary>
    /// <param name="sigmas">How far from the mean a reading must be to be flagged; 3 is the usual choice.</param>
    /// <param name="capacity">
    /// How many earlier readings the baseline keeps, from 2 to 32, since a spread needs
    /// two; 32 unless told fewer.
    /// </param>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="capacity"/> is not from 2 to 32.</exception>
    public Anomaly(float sigmas, int capacity = Kit.WindowCapacity)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_anomaly_with_capacity(sigmas, Capacities.Check(capacity, 2)),
            NativeMethods.pamoja_anomaly_free,
            "anomaly detector",
            serialized: true);
    }

    /// <summary>How many readings the baseline keeps.</summary>
    public int Capacity => checked((int)_handle.Use(NativeMethods.pamoja_anomaly_capacity));

    /// <summary>Folds a reading in and reports whether it stands out.</summary>
    /// <param name="reading">The reading to check.</param>
    /// <returns>
    /// Whether the reading is further from the mean than the configured number of
    /// deviations. Nothing is flagged before two readings are held; from the third on, a
    /// reading can be, and one that is not a finite number always is.
    /// </returns>
    public bool Check(float reading) =>
        _handle.Use(handle => NativeMethods.pamoja_anomaly_check(handle, reading));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Checks the capacity asked of a windowed helper.</summary>
internal static class Capacities
{
    /// <summary>Refuses a capacity the helper cannot keep.</summary>
    /// <param name="capacity">The capacity asked for.</param>
    /// <param name="least">The fewest readings the helper can answer from.</param>
    /// <returns>The capacity, as the C ABI takes it.</returns>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="capacity"/> is out of range.</exception>
    internal static nuint Check(int capacity, int least)
    {
        if (capacity < least || capacity > Kit.WindowCapacity)
        {
            throw new ArgumentOutOfRangeException(
                nameof(capacity),
                string.Create(
                    CultureInfo.InvariantCulture,
                    $"capacity must be a whole number from {least} to {Kit.WindowCapacity}, not {capacity}"));
        }

        return (nuint)capacity;
    }
}
