using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Notices a step change between successive readings.</summary>
/// <remarks>A sudden jump usually means an event rather than drift: a burst pipe, a door opening, a load switching on.</remarks>
public sealed class Surge : IDisposable
{
    private readonly NativeHandle _handle;

    private Surge(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_surge_free, "surge detector", serialized: true);
    }

    /// <summary>Creates a detector for rises of more than the given size between readings.</summary>
    /// <param name="limit">The largest safe rise per reading; its magnitude is used.</param>
    /// <returns>The detector.</returns>
    public static Surge Rising(float limit) =>
        new(NativeMethods.pamoja_surge_rising(limit));

    /// <summary>Creates a detector for falls of more than the given size between readings.</summary>
    /// <param name="limit">The largest safe fall per reading; its magnitude is used.</param>
    /// <returns>The detector.</returns>
    public static Surge Falling(float limit) =>
        new(NativeMethods.pamoja_surge_falling(limit));

    /// <summary>Feeds a value in and reports a step past the limit.</summary>
    /// <param name="value">The latest reading. One that is not a finite number is ignored.</param>
    /// <returns>
    /// The size of the step, as a positive number, or <c>null</c> when the change stayed
    /// within the limit, went the other way, or this is the first reading.
    /// </returns>
    public float? Update(float value) =>
        _handle.UseTry<float>((IntPtr handle, out float delta) =>
            NativeMethods.pamoja_surge_update(handle, value, out delta));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
