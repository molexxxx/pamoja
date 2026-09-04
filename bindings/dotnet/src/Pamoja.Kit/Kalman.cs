using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Estimates a true value from noisy readings.</summary>
/// <remarks>The filter trusts the model and the sensor in proportion to how noisy each is, so it settles faster than a plain average without chasing every spike.</remarks>
public sealed class Kalman : IDisposable
{
    private readonly NativeHandle _handle;

    private Kalman(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_kalman_free, "Kalman filter");
    }

    /// <summary>Creates a filter from the noise levels and a first guess.</summary>
    /// <param name="processNoise">How much the true value is expected to drift.</param>
    /// <param name="measurementNoise">How noisy the sensor is.</param>
    /// <param name="initial">The first guess at the true value.</param>
    public Kalman(float processNoise, float measurementNoise, float initial)
        : this(NativeMethods.pamoja_kalman_new(processNoise, measurementNoise, initial))
    {
    }

    /// <summary>Folds a reading in and returns the new estimate.</summary>
    /// <param name="reading">The latest raw reading.</param>
    /// <returns>The updated estimate.</returns>
    public float Update(float reading) =>
        _handle.Use(handle => NativeMethods.pamoja_kalman_update(handle, reading));

    /// <summary>Gets the current estimate.</summary>
    public float Estimate =>
        _handle.Use(NativeMethods.pamoja_kalman_estimate);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
