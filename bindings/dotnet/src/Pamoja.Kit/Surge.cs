using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Notices a step change between successive readings.</summary>
/// <remarks>A sudden jump usually means an event rather than drift: a burst pipe, a door opening, a load switching on.</remarks>
public sealed class Surge : IDisposable
{
    private readonly NativeHandle _handle;

    private Surge(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_surge_free, "surge detector");
    }

    /// <summary>Creates a detector for rises of at least the given size.</summary>
    /// <param name="limit">The smallest rise worth reporting.</param>
    /// <returns>The detector.</returns>
    public static Surge Rising(float limit) =>
        new(NativeMethods.pamoja_surge_rising(limit));

    /// <summary>Creates a detector for falls of at least the given size.</summary>
    /// <param name="limit">The smallest fall worth reporting.</param>
    /// <returns>The detector.</returns>
    public static Surge Falling(float limit) =>
        new(NativeMethods.pamoja_surge_falling(limit));

    /// <summary>Feeds a value in and reports a qualifying step.</summary>
    /// <param name="value">The latest reading.</param>
    /// <returns>The size of the step, or <c>null</c> when this reading did not complete one.</returns>
    public float? Update(float value) =>
        _handle.UseTry<float>((IntPtr handle, out float delta) =>
            NativeMethods.pamoja_surge_update(handle, value, out delta));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
