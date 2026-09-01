using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>Turns a raw sensor count into the units the reading is actually in.</summary>
/// <remarks>A sensor reports counts, not litres or degrees; a calibration is the mapping between them, either stated outright or fitted through two known points.</remarks>
public sealed class Calibration : IDisposable
{
    private readonly NativeHandle _handle;

    private Calibration(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_calibration_free, "calibration");
    }

    /// <summary>Creates a calibration applying <c>raw * scale + offset</c>.</summary>
    /// <param name="scale">The multiplier applied to the raw reading.</param>
    /// <param name="offset">The constant added after scaling.</param>
    /// <returns>The calibration.</returns>
    public static Calibration Linear(float scale, float offset) =>
        new(NativeMethods.pamoja_calibration_linear(scale, offset));

    /// <summary>Creates a calibration fitted through two known reference points.</summary>
    /// <param name="rawLow">The raw reading at the low reference point.</param>
    /// <param name="valueLow">The real value at the low reference point.</param>
    /// <param name="rawHigh">The raw reading at the high reference point.</param>
    /// <param name="valueHigh">The real value at the high reference point.</param>
    /// <returns>The calibration.</returns>
    public static Calibration TwoPoint(
        float rawLow,
        float valueLow,
        float rawHigh,
        float valueHigh) =>
        new(NativeMethods.pamoja_calibration_two_point(
            rawLow, valueLow, rawHigh, valueHigh));

    /// <summary>Converts a raw reading into calibrated units.</summary>
    /// <param name="raw">The raw sensor reading.</param>
    /// <returns>The reading in real units.</returns>
    public float Apply(float raw) =>
        _handle.Use(handle => NativeMethods.pamoja_calibration_apply(handle, raw));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
