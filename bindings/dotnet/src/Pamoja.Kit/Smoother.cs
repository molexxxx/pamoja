using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Smooths a noisy reading by weighting each new sample against the running value.</summary>
/// <remarks>One noisy reading is rarely worth acting on. Feeding samples through a smoother lets a trend show without a single spike triggering a pump or an alarm.</remarks>
public sealed class Smoother : IDisposable
{
    private readonly NativeHandle _handle;

    private Smoother(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_smoother_free, "smoother");
    }

    /// <summary>Creates a smoother at the given weight.</summary>
    /// <param name="weight">How much each new sample counts, between 0 and 1.</param>
    public Smoother(float weight)
        : this(NativeMethods.pamoja_smoother_new(weight))
    {
    }

    /// <summary>Folds a sample in and returns the smoothed value.</summary>
    /// <param name="sample">The latest raw reading.</param>
    /// <returns>The smoothed value.</returns>
    public float Update(float sample) =>
        _handle.Use(handle => NativeMethods.pamoja_smoother_update(handle, sample));

    /// <summary>Gets the current value, or <c>null</c> before the first sample.</summary>
    public float? Value =>
        _handle.UseTry<float>(NativeMethods.pamoja_smoother_value);

    /// <summary>Clears the smoother back to its initial state.</summary>
    public void Reset() =>
        _handle.Use(NativeMethods.pamoja_smoother_reset);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
