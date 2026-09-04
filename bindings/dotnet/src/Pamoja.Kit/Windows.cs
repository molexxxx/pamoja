using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>A rolling window of the most recent readings, with the stats over them.</summary>
/// <remarks>
/// The core helpers are generic over their capacity, which cannot cross the C ABI,
/// so these are built at one documented size: <see cref="Capacity"/>.
/// </remarks>
public sealed class Window : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an empty window.</summary>
    /// <exception cref="PamojaException">The native window could not be created.</exception>
    public Window()
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_window_new(), NativeMethods.pamoja_window_free, "window");
    }

    /// <summary>How many readings the window holds.</summary>
    public int Count => checked((int)_handle.Use(NativeMethods.pamoja_window_len));

    /// <summary>How many readings it holds before it starts dropping the oldest.</summary>
    public int Capacity => checked((int)_handle.Use(NativeMethods.pamoja_window_capacity));

    /// <summary>Adds a reading, dropping the oldest once the window is full.</summary>
    /// <param name="reading">The reading to add.</param>
    public void Push(float reading) =>
        _handle.Use(handle => NativeMethods.pamoja_window_push(handle, reading));

    /// <summary>The mean of the readings, or <c>null</c> while the window is empty.</summary>
    /// <returns>The mean, if there is one.</returns>
    public float? Mean() => _handle.UseTry<float>(NativeMethods.pamoja_window_mean);

    /// <summary>The smallest reading, or <c>null</c> while the window is empty.</summary>
    /// <returns>The minimum, if there is one.</returns>
    public float? Min() => _handle.UseTry<float>(NativeMethods.pamoja_window_min);

    /// <summary>The largest reading, or <c>null</c> while the window is empty.</summary>
    /// <returns>The maximum, if there is one.</returns>
    public float? Max() => _handle.UseTry<float>(NativeMethods.pamoja_window_max);

    /// <summary>The spread between the smallest and largest readings.</summary>
    /// <returns>The range, if there is one.</returns>
    public float? Range() => _handle.UseTry<float>(NativeMethods.pamoja_window_range);

    /// <summary>The variance of the readings, or <c>null</c> without enough of them.</summary>
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
    /// <exception cref="PamojaException">The native filter could not be created.</exception>
    public Median()
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_median_new(), NativeMethods.pamoja_median_free, "median filter");
    }

    /// <summary>The current median, or <c>null</c> before the first reading.</summary>
    public float? Value => _handle.UseTry<float>(NativeMethods.pamoja_median_value);

    /// <summary>Folds a reading in and returns the median of the window.</summary>
    /// <param name="reading">The reading to add.</param>
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
    /// <exception cref="PamojaException">The native estimator could not be created.</exception>
    public Trend()
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_trend_new(), NativeMethods.pamoja_trend_free, "trend estimator");
    }

    /// <summary>
    /// The fitted slope in units per reading, or <c>null</c> without enough
    /// readings. A positive slope is a rising signal.
    /// </summary>
    public float? Slope => _handle.UseTry<float>(NativeMethods.pamoja_trend_slope);

    /// <summary>Adds a reading.</summary>
    /// <param name="reading">The reading to add.</param>
    public void Push(float reading) =>
        _handle.Use(handle => NativeMethods.pamoja_trend_push(handle, reading));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Flags a reading that stands out from the ones around it.</summary>
public sealed class Anomaly : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a detector that flags a reading so many deviations out.</summary>
    /// <param name="sigmas">How far from the mean a reading must be to be flagged.</param>
    /// <exception cref="PamojaException">The native detector could not be created.</exception>
    public Anomaly(float sigmas)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_anomaly_new(sigmas),
            NativeMethods.pamoja_anomaly_free,
            "anomaly detector");
    }

    /// <summary>Folds a reading in and reports whether it stands out.</summary>
    /// <param name="reading">The reading to check.</param>
    /// <returns>
    /// Whether the reading is further from the mean than the configured number of
    /// deviations. A window still filling reports <c>false</c>.
    /// </returns>
    public bool Check(float reading) =>
        _handle.Use(handle => NativeMethods.pamoja_anomaly_check(handle, reading));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
