using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Switches a load on and off around a setpoint, with hysteresis to stop chatter.</summary>
/// <remarks>Without a dead band a load switches rapidly around the setpoint and wears out; the hysteresis is what makes on/off control safe for a compressor or a pump.</remarks>
public sealed class Thermostat : IDisposable
{
    private readonly NativeHandle _handle;

    private Thermostat(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_thermostat_free, "thermostat");
    }

    /// <summary>Creates a cooling thermostat, which switches on when the reading rises.</summary>
    /// <param name="setpoint">The value being held.</param>
    /// <param name="hysteresis">How far past the setpoint before switching.</param>
    /// <returns>The thermostat.</returns>
    public static Thermostat Cooling(float setpoint, float hysteresis) =>
        new(NativeMethods.pamoja_thermostat_cooling(setpoint, hysteresis));

    /// <summary>Creates a heating thermostat, which switches on when the reading falls.</summary>
    /// <param name="setpoint">The value being held.</param>
    /// <param name="hysteresis">How far past the setpoint before switching.</param>
    /// <returns>The thermostat.</returns>
    public static Thermostat Heating(float setpoint, float hysteresis) =>
        new(NativeMethods.pamoja_thermostat_heating(setpoint, hysteresis));

    /// <summary>Feeds a reading in and reports whether the load should be on.</summary>
    /// <param name="reading">The latest measured value.</param>
    /// <returns><c>true</c> while the load should run.</returns>
    public bool Update(float reading) =>
        _handle.Use(handle => NativeMethods.pamoja_thermostat_update(handle, reading));

    /// <summary>Gets whether the load should currently be on.</summary>
    public bool IsOn =>
        _handle.Use(NativeMethods.pamoja_thermostat_is_on);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
