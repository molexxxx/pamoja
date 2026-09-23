using Pamoja.Gpio;
using Pamoja.Hal;

namespace Pamoja.Actuators;

/// <summary>
/// A four-wire stepper driven coil by coil, through a transistor array such as a ULN2003 or an
/// H-bridge, as <c>pamoja_actuators::stepper::FourWire</c> drives one in Rust.
/// </summary>
/// <remarks>
/// Each step energizes the coils the drive pattern names, coil A for the pattern's high bit
/// down to coil D for its low bit, then waits the step interval. The position counts steps
/// from where the motor was when the driver was built, forward positive. A coil line is any
/// <see cref="IOutputLine"/>: a <see cref="GpioLine"/> on a board, or a
/// <see cref="PinScript"/> that records every level.
/// </remarks>
/// <typeparam name="TLine">The coil lines.</typeparam>
/// <example>
/// <code>
/// var coils = (new PinScript(), new PinScript(), new PinScript(), new PinScript());
/// using var motor = new FourWire&lt;PinScript&gt;(coils, StepDrive.FullStep, delay: new DelayLog());
/// motor.Steps(2);
/// // motor.Position is 2
/// </code>
/// </example>
public sealed class FourWire<TLine> : IDisposable
    where TLine : IOutputLine
{
    private readonly (TLine A, TLine B, TLine C, TLine D) _coils;
    private readonly Stepper _sequence;
    private readonly IDelay _delay;

    /// <summary>Wraps the four coil lines, without driving them.</summary>
    /// <param name="coils">The lines for coils A, B, C, and D, in the motor's phase order.</param>
    /// <param name="drive">The coil pattern to step through.</param>
    /// <param name="stepMicros">The pause after each step, which sets the speed.</param>
    /// <param name="delay">
    /// What waits: a <see cref="SleepDelay"/> unless given, or a <see cref="DelayLog"/> to run
    /// with nothing plugged in.
    /// </param>
    /// <exception cref="PamojaException">The native sequencer could not be created.</exception>
    public FourWire(
        (TLine A, TLine B, TLine C, TLine D) coils,
        StepDrive drive,
        uint stepMicros = Stepper.DefaultStepMicros,
        IDelay? delay = null)
    {
        _coils = coils;
        _sequence = new Stepper(drive);
        _delay = delay ?? new SleepDelay();
        Drive = drive;
        StepMicros = stepMicros;
    }

    /// <summary>The step count since the driver was built, forward positive.</summary>
    public int Position { get; private set; }

    /// <summary>The coil pattern in use.</summary>
    public StepDrive Drive { get; }

    /// <summary>The pause after each step, in microseconds.</summary>
    public uint StepMicros { get; }

    /// <summary>Takes one step and waits the step interval.</summary>
    /// <param name="direction">Which way to step.</param>
    /// <exception cref="Exception">Whatever a coil line throws when it cannot be driven.</exception>
    public void Step(StepDirection direction)
    {
        Energize(_sequence.Step(direction));
        Position += direction == StepDirection.Forward ? 1 : -1;
        _delay.DelayMicros(StepMicros);
    }

    /// <summary>Takes <paramref name="count"/> steps, backward when negative.</summary>
    /// <param name="count">The signed number of steps.</param>
    /// <exception cref="Exception">
    /// Whatever a coil line throws; the steps already taken stay counted.
    /// </exception>
    public void Steps(int count)
    {
        StepDirection direction = count < 0 ? StepDirection.Backward : StepDirection.Forward;
        for (long taken = 0; taken < Math.Abs((long)count); taken++)
        {
            Step(direction);
        }
    }

    /// <summary>Drops every coil, so the motor holds nothing and draws nothing.</summary>
    /// <exception cref="Exception">Whatever a coil line throws when it cannot be driven.</exception>
    public void Idle() => Energize(0);

    /// <summary>Hands the coil lines back, for a test to read what was driven or a program to reuse them.</summary>
    /// <returns>The lines for coils A, B, C, and D.</returns>
    public (TLine A, TLine B, TLine C, TLine D) Release() => _coils;

    /// <summary>Releases the native sequencer. The coil lines stay the caller's.</summary>
    public void Dispose() => _sequence.Dispose();

    private void Energize(byte coils)
    {
        _coils.A.Drive(Level(coils, 0b1000));
        _coils.B.Drive(Level(coils, 0b0100));
        _coils.C.Drive(Level(coils, 0b0010));
        _coils.D.Drive(Level(coils, 0b0001));
    }

    private static PinLevel Level(byte coils, byte bit) =>
        (coils & bit) != 0 ? PinLevel.High : PinLevel.Low;
}

/// <summary>
/// A stepper behind a step and direction driver chip such as an A4988 or a DRV8825, as
/// <c>pamoja_actuators::stepper::StepDir</c> drives one in Rust.
/// </summary>
/// <remarks>
/// Each step sets the direction line, high for forward, and holds it for the pulse width,
/// since the chip reads the direction on the step line's rising edge and needs it settled
/// first. Then it pulses the step line high for the pulse width, brings it low, and waits the
/// step interval. Microstepping, current limiting, and enable are the chip's own pins and
/// settings, outside this driver.
/// </remarks>
/// <typeparam name="TLine">The step and direction lines.</typeparam>
/// <example>
/// <code>
/// var motor = new StepDir&lt;PinScript&gt;(new PinScript(), new PinScript(), delay: new DelayLog());
/// motor.Steps(-1);
/// // motor.Position is -1
/// </code>
/// </example>
public sealed class StepDir<TLine>
    where TLine : IOutputLine
{
    private readonly TLine _step;
    private readonly TLine _direction;
    private readonly IDelay _delay;

    /// <summary>Wraps the step and direction lines, without driving them.</summary>
    /// <param name="step">The line the chip counts rising edges on.</param>
    /// <param name="direction">The line the chip reads the direction from; high is forward.</param>
    /// <param name="pulseMicros">
    /// How long the direction line is held before the step line rises, and the step line is
    /// then held high.
    /// </param>
    /// <param name="stepMicros">The pause after each step, which sets the speed.</param>
    /// <param name="delay">
    /// What waits: a <see cref="SleepDelay"/> unless given, or a <see cref="DelayLog"/> to run
    /// with nothing plugged in.
    /// </param>
    public StepDir(
        TLine step,
        TLine direction,
        uint pulseMicros = Stepper.DefaultPulseMicros,
        uint stepMicros = Stepper.DefaultStepMicros,
        IDelay? delay = null)
    {
        _step = step;
        _direction = direction;
        _delay = delay ?? new SleepDelay();
        PulseMicros = pulseMicros;
        StepMicros = stepMicros;
    }

    /// <summary>The step count since the driver was built, forward positive.</summary>
    public int Position { get; private set; }

    /// <summary>How long the direction is held before each step pulse, and the pulse itself, in microseconds.</summary>
    public uint PulseMicros { get; }

    /// <summary>The pause after each step, in microseconds.</summary>
    public uint StepMicros { get; }

    /// <summary>Takes one step and waits the step interval.</summary>
    /// <param name="direction">Which way to step.</param>
    /// <exception cref="Exception">Whatever a line throws when it cannot be driven.</exception>
    public void Step(StepDirection direction)
    {
        bool forward = direction == StepDirection.Forward;
        _direction.Drive(forward ? PinLevel.High : PinLevel.Low);
        _delay.DelayMicros(PulseMicros);
        _step.Drive(PinLevel.High);
        _delay.DelayMicros(PulseMicros);
        _step.Drive(PinLevel.Low);
        Position += forward ? 1 : -1;
        _delay.DelayMicros(StepMicros);
    }

    /// <summary>Takes <paramref name="count"/> steps, backward when negative.</summary>
    /// <param name="count">The signed number of steps.</param>
    /// <exception cref="Exception">
    /// Whatever a line throws; the steps already taken stay counted.
    /// </exception>
    public void Steps(int count)
    {
        StepDirection direction = count < 0 ? StepDirection.Backward : StepDirection.Forward;
        for (long taken = 0; taken < Math.Abs((long)count); taken++)
        {
            Step(direction);
        }
    }

    /// <summary>Hands the lines back.</summary>
    /// <returns>The step line, then the direction line.</returns>
    public (TLine Step, TLine Direction) Release() => (_step, _direction);
}
