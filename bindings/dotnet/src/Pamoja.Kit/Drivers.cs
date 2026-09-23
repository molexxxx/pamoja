using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>A hobby servo's pulse widths across its travel.</summary>
/// <param name="MinUs">The pulse width at zero degrees, in microseconds.</param>
/// <param name="MaxUs">The pulse width at full travel, in microseconds.</param>
/// <param name="RangeDeg">The full travel in degrees; its magnitude is used.</param>
public readonly record struct ServoMap(ushort MinUs, ushort MaxUs, float RangeDeg)
{
    /// <summary>Gets the standard hobby servo: 1000 to 2000 microseconds over 180 degrees.</summary>
    public static ServoMap Standard
    {
        get
        {
            PamojaServoMap standard = NativeMethods.pamoja_servo_map_standard();
            return new ServoMap(standard.MinUs, standard.MaxUs, standard.RangeDeg);
        }
    }

    /// <summary>Returns the pulse width that sets the servo to an angle.</summary>
    /// <param name="angleDeg">The angle in degrees, held to the servo's travel.</param>
    /// <returns>The pulse width in microseconds, or 0, no pulse, for an angle that is not a number.</returns>
    public ushort Pulse(float angleDeg) => NativeMethods.pamoja_servo_map_pulse(ToNative(), angleDeg);

    /// <summary>Returns the angle a pulse width sets.</summary>
    /// <param name="pulseUs">The pulse width in microseconds, held to the servo's range.</param>
    /// <returns>The angle in degrees, or 0 for a servo whose pulse range is empty.</returns>
    public float Angle(ushort pulseUs) => NativeMethods.pamoja_servo_map_angle(ToNative(), pulseUs);

    private PamojaServoMap ToNative() => new() { MinUs = MinUs, MaxUs = MaxUs, RangeDeg = RangeDeg };
}

/// <summary>An electronic speed controller's pulse widths from full reverse to full forward.</summary>
/// <param name="MinUs">The pulse width at full reverse, in microseconds.</param>
/// <param name="NeutralUs">The pulse width at rest, in microseconds.</param>
/// <param name="MaxUs">The pulse width at full forward, in microseconds.</param>
public readonly record struct Esc(ushort MinUs, ushort NeutralUs, ushort MaxUs)
{
    /// <summary>Gets the common reversible controller: 1000, 1500, and 2000 microseconds.</summary>
    public static Esc Bidirectional
    {
        get
        {
            PamojaEsc reversible = NativeMethods.pamoja_esc_bidirectional();
            return new Esc(reversible.MinUs, reversible.NeutralUs, reversible.MaxUs);
        }
    }

    /// <summary>Returns the pulse width for a throttle.</summary>
    /// <param name="throttle">The demand from -1, full reverse, to 1, full forward, held to that range.</param>
    /// <returns>The pulse width in microseconds, or the neutral one for a throttle that is not a number.</returns>
    public ushort Pulse(float throttle) =>
        NativeMethods.pamoja_esc_pulse(new PamojaEsc { MinUs = MinUs, NeutralUs = NeutralUs, MaxUs = MaxUs }, throttle);
}

/// <summary>Counts a quadrature encoder's steps from its A and B channels.</summary>
public sealed class Quadrature : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a decoder seeded with the channel levels it reads now.</summary>
    /// <param name="a">The A channel's level now.</param>
    /// <param name="b">The B channel's level now.</param>
    /// <remarks>Seeding with the real levels keeps the first reading from counting a step that did not happen.</remarks>
    public Quadrature(bool a = false, bool b = false)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_quadrature_new(a, b), NativeMethods.pamoja_quadrature_free, "quadrature decoder", serialized: true);
    }

    /// <summary>Gets the signed count of steps so far.</summary>
    public long Count => _handle.Use(NativeMethods.pamoja_quadrature_count);

    /// <summary>Feeds the channel levels read now.</summary>
    /// <param name="a">The A channel's level.</param>
    /// <param name="b">The B channel's level.</param>
    /// <returns>1 or -1 for a step in either direction, or 0 for no change or a jump past a step.</returns>
    public int Update(bool a, bool b) => _handle.Use(handle => NativeMethods.pamoja_quadrature_update(handle, a, b));

    /// <summary>Sets the count back to 0, keeping the channel state last read.</summary>
    public void Reset() => _handle.Use(NativeMethods.pamoja_quadrature_reset);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Turns encoder steps into the distance and speed of the wheel they turn with.</summary>
/// <param name="CountsPerRev">Steps per wheel revolution; its magnitude is used.</param>
/// <param name="WheelRadius">The wheel radius in meters; its magnitude is used.</param>
public readonly record struct QuadratureScale(float CountsPerRev, float WheelRadius)
{
    /// <summary>Returns the distance a wheel rolled for a step count.</summary>
    /// <param name="count">The signed step count.</param>
    /// <returns>The distance in meters, or 0 for a scale with no resolution.</returns>
    public float Distance(long count) => NativeMethods.pamoja_quadrature_scale_distance(ToNative(), count);

    /// <summary>Returns a wheel's speed from the steps counted over a time step.</summary>
    /// <param name="deltaCount">The steps counted during the step.</param>
    /// <param name="dt">The length of the step.</param>
    /// <returns>The speed in meters per second, or 0 when <paramref name="dt"/> is 0.</returns>
    public float Velocity(long deltaCount, float dt) =>
        NativeMethods.pamoja_quadrature_scale_velocity(ToNative(), deltaCount, dt);

    private PamojaQuadratureScale ToNative() => new() { CountsPerRev = CountsPerRev, WheelRadius = WheelRadius };
}
