using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Warns before a falling level runs out, by projecting its rate of fall.</summary>
/// <remarks>Knowing a tank is empty is too late; this reports how many samples remain at the current rate, so the refill can be arranged while there is still time.</remarks>
public sealed class Depletion : IDisposable
{
    private readonly NativeHandle _handle;

    private Depletion(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_depletion_free, "depletion estimator");
    }

    /// <summary>Creates an estimator counting down to a threshold.</summary>
    /// <param name="threshold">The level treated as empty.</param>
    public Depletion(float threshold)
        : this(NativeMethods.pamoja_depletion_new(threshold))
    {
    }

    /// <summary>Records a level and estimates the samples left before the threshold.</summary>
    /// <param name="level">The latest measured level.</param>
    /// <returns>The samples remaining, or <c>null</c> while the level is steady or rising, and on the first reading, when no rate of fall is known yet.</returns>
    public uint? Update(float level) =>
        _handle.UseTry<uint>((IntPtr handle, out uint samples) =>
            NativeMethods.pamoja_depletion_update(handle, level, out samples));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
