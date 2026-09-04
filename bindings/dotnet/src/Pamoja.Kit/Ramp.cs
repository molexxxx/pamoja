using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Limits how fast a value may change, so a load is never slammed.</summary>
/// <remarks>Jumping a motor or a valve straight to a new target draws a surge and wears the mechanism; a ramp walks it there instead.</remarks>
public sealed class Ramp : IDisposable
{
    private readonly NativeHandle _handle;

    private Ramp(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_ramp_free, "ramp");
    }

    /// <summary>Creates a rate limiter.</summary>
    /// <param name="start">The value to start at.</param>
    /// <param name="maxStep">The largest change allowed in one step.</param>
    public Ramp(float start, float maxStep)
        : this(NativeMethods.pamoja_ramp_new(start, maxStep))
    {
    }

    /// <summary>Moves one step toward a target and returns the new value.</summary>
    /// <param name="target">The value being approached.</param>
    /// <returns>The rate-limited value.</returns>
    public float Update(float target) =>
        _handle.Use(handle => NativeMethods.pamoja_ramp_update(handle, target));

    /// <summary>Gets the current value.</summary>
    public float Value =>
        _handle.Use(NativeMethods.pamoja_ramp_value);

    /// <summary>Forces the value without rate limiting.</summary>
    /// <param name="value">The value to jump to.</param>
    public void Set(float value) =>
        _handle.Use(handle => NativeMethods.pamoja_ramp_set(handle, value));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
