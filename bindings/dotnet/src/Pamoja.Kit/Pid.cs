using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Holds a value at a setpoint by trading off present, past, and predicted error.</summary>
/// <remarks>This is the controller behind holding a temperature, a pressure, or a speed steady against a load that keeps changing.</remarks>
public sealed class Pid : IDisposable
{
    private readonly NativeHandle _handle;

    private Pid(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_pid_free, "PID controller");
    }

    /// <summary>Creates a controller with the given gains and no output limits.</summary>
    /// <param name="kp">The proportional gain, reacting to present error.</param>
    /// <param name="ki">The integral gain, correcting accumulated error.</param>
    /// <param name="kd">The derivative gain, damping predicted error.</param>
    public Pid(float kp, float ki, float kd)
        : this(NativeMethods.pamoja_pid_new(kp, ki, kd))
    {
    }

    /// <summary>Creates a controller whose output is clamped to a range.</summary>
    /// <param name="kp">The proportional gain, reacting to present error.</param>
    /// <param name="ki">The integral gain, correcting accumulated error.</param>
    /// <param name="kd">The derivative gain, damping predicted error.</param>
    /// <param name="min">The lowest output the controller may command.</param>
    /// <param name="max">The highest output the controller may command.</param>
    /// <returns>The clamped controller.</returns>
    public static Pid WithLimits(float kp, float ki, float kd, float min, float max) =>
        new(NativeMethods.pamoja_pid_new_with_limits(kp, ki, kd, min, max));

    /// <summary>Advances the controller by one step.</summary>
    /// <param name="setpoint">The value being held.</param>
    /// <param name="measurement">The latest measured value.</param>
    /// <param name="dt">Seconds since the previous step.</param>
    /// <returns>The control output.</returns>
    public float Update(float setpoint, float measurement, float dt) =>
        _handle.Use(handle => NativeMethods.pamoja_pid_update(handle, setpoint, measurement, dt));

    /// <summary>Clears the accumulated integral and last error.</summary>
    public void Reset() =>
        _handle.Use(NativeMethods.pamoja_pid_reset);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
