using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>A stepper drive pattern, trading torque, smoothness, and resolution.</summary>
public enum StepDrive
{
    /// <summary>One coil at a time: four steps, least torque and least power.</summary>
    Wave = 0,

    /// <summary>Two adjacent coils at a time: four steps, most torque.</summary>
    FullStep = 1,

    /// <summary>Alternating one and two coils: eight steps, double resolution.</summary>
    HalfStep = 2,
}

/// <summary>Which way to step a motor.</summary>
public enum StepDirection
{
    /// <summary>Advance the sequence, turning the shaft one way.</summary>
    Forward = 0,

    /// <summary>Reverse the sequence, turning the shaft the other way.</summary>
    Backward = 1,
}

/// <summary>An NXP PCA9685 16-channel PWM controller, for servos, LEDs, and valves.</summary>
public static class Pca9685
{
    /// <summary>The part's internal oscillator frequency, in hertz.</summary>
    public const uint InternalOscHz = 25_000_000;

    /// <summary>How many channels it drives.</summary>
    public const byte Channels = 16;

    /// <summary>How many counts each period is divided into.</summary>
    public const ushort Counts = 4096;

    /// <summary>Returns the first of a channel's four consecutive registers.</summary>
    /// <param name="channel">The channel, 0 to 15.</param>
    /// <returns>The register address.</returns>
    /// <exception cref="PamojaException">The channel is beyond the part.</exception>
    public static byte ChannelRegister(byte channel)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_pca9685_channel_register(channel, out byte register));
        return register;
    }

    /// <summary>Returns the prescale value that sets an update rate.</summary>
    /// <param name="updateRateHz">The PWM frequency wanted.</param>
    /// <param name="oscHz">The oscillator frequency, usually <see cref="InternalOscHz"/>.</param>
    /// <returns>The prescale register value.</returns>
    public static byte PrescaleForFrequency(uint updateRateHz, uint oscHz = InternalOscHz) =>
        NativeMethods.pamoja_pca9685_prescale_for_frequency(updateRateHz, oscHz);

    /// <summary>Returns the update rate a prescale value produces.</summary>
    /// <param name="prescale">The prescale register value.</param>
    /// <param name="oscHz">The oscillator frequency, usually <see cref="InternalOscHz"/>.</param>
    /// <returns>The frequency in hertz.</returns>
    public static float FrequencyForPrescale(byte prescale, uint oscHz = InternalOscHz) =>
        NativeMethods.pamoja_pca9685_frequency_for_prescale(prescale, oscHz);
}

/// <summary>The four register bytes for one PCA9685 channel.</summary>
/// <remarks>
/// Each call returns them in the channel's own register order, so they can be
/// written in a single bus transaction.
/// </remarks>
public static class Pwm
{
    /// <summary>Builds a setting from explicit on and off counts.</summary>
    /// <param name="on">The count at which the output goes high.</param>
    /// <param name="off">The count at which it goes low.</param>
    /// <returns>The four register bytes; counts are masked to 12 bits.</returns>
    public static byte[] FromCounts(ushort on, ushort off) =>
        Bytes(NativeMethods.pamoja_pwm_from_counts(on, off));

    /// <summary>Builds a setting with no phase delay: on at count 0, off at <paramref name="off"/>.</summary>
    /// <param name="off">The count at which the output goes low, which sets the duty.</param>
    /// <returns>The four register bytes.</returns>
    public static byte[] Duty(ushort off) => Bytes(NativeMethods.pamoja_pwm_duty(off));

    /// <summary>Builds the setting that drives a hobby servo to a pulse width.</summary>
    /// <param name="pulseMicros">
    /// The high-pulse width in microseconds. Typical travel is about 1000 to 2000.
    /// </param>
    /// <param name="updateRateHz">The PWM frequency the controller is set to.</param>
    /// <returns>The four register bytes.</returns>
    public static byte[] Servo(uint pulseMicros, uint updateRateHz = 50) =>
        Bytes(NativeMethods.pamoja_pwm_servo(pulseMicros, updateRateHz));

    /// <summary>The setting that holds a channel continuously high.</summary>
    /// <returns>The four register bytes.</returns>
    public static byte[] FullOn() => Bytes(NativeMethods.pamoja_pwm_full_on());

    /// <summary>The setting that holds a channel continuously low, the power-on state.</summary>
    /// <returns>
    /// The four register bytes. This is not the same as a zero duty, which still
    /// glitches high for one count.
    /// </returns>
    public static byte[] FullOff() => Bytes(NativeMethods.pamoja_pwm_full_off());

    /// <summary>Lays a native setting out in register order.</summary>
    /// <param name="pwm">The setting the native call produced.</param>
    /// <returns>The four bytes.</returns>
    private static byte[] Bytes(PamojaPwm pwm) =>
        [pwm.OnLow, pwm.OnHigh, pwm.OffLow, pwm.OffHigh];
}

/// <summary>A stepper motor's place in its drive sequence, and how far it has turned.</summary>
/// <example>
/// <code>
/// using var motor = new Stepper(StepDrive.HalfStep);
/// byte coils = motor.Step(StepDirection.Forward);
/// </code>
/// </example>
public sealed class Stepper : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a stepper at the start of a pattern, with its position at zero.</summary>
    /// <param name="drive">The coil pattern to walk.</param>
    /// <exception cref="PamojaException">The native stepper could not be created.</exception>
    public Stepper(StepDrive drive)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_stepper_new((PamojaStepDrive)drive),
            NativeMethods.pamoja_stepper_free,
            "stepper");
    }

    /// <summary>The coil pattern currently held, without advancing.</summary>
    public byte Coils => _handle.Use(NativeMethods.pamoja_stepper_coils);

    /// <summary>How many steps have been taken, signed by direction.</summary>
    public int Steps => _handle.Use(NativeMethods.pamoja_stepper_steps);

    /// <summary>Returns how many steps one electrical cycle of a pattern takes.</summary>
    /// <param name="drive">The coil pattern.</param>
    /// <returns><c>4</c> for wave and full-step, <c>8</c> for half-step.</returns>
    public static int StepCount(StepDrive drive) =>
        checked((int)NativeMethods.pamoja_stepper_step_count((PamojaStepDrive)drive));

    /// <summary>Returns how many steps a rotation of an angle takes on a motor.</summary>
    /// <param name="degrees">The angle to turn through.</param>
    /// <param name="stepsPerRevolution">The motor's steps per full revolution.</param>
    /// <returns>The step count, negative for a negative angle.</returns>
    public static int StepsForDegrees(float degrees, uint stepsPerRevolution) =>
        NativeMethods.pamoja_stepper_steps_for_degrees(degrees, stepsPerRevolution);

    /// <summary>Advances one step and returns the four-bit coil pattern to apply.</summary>
    /// <param name="direction">Which way to turn.</param>
    /// <returns>
    /// The coil pattern; the most significant of the four bits is the first coil.
    /// </returns>
    public byte Step(StepDirection direction) =>
        _handle.Use(handle => NativeMethods.pamoja_stepper_step(handle, (PamojaStepDirection)direction));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
