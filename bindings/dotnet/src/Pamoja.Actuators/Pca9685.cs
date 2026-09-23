using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Actuators;

/// <summary>
/// An NXP PCA9685 16-channel PWM controller, for servos, LEDs, and valves, and a driver for one
/// on an I2C bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet: its registers, the prescale arithmetic, and
/// <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives one part over an <see cref="I2cBus"/>. It programs the prescale for its
/// frequency with the oscillator asleep, because the part only takes a prescale then, wakes
/// it, waits the 500 us it needs, and loads a channel's four registers in one transfer.
/// Nothing is sent until <see cref="Init"/> or the first channel is loaded.
/// </para>
/// </remarks>
public sealed class Pca9685 : IDisposable
{
    /// <summary>The address it answers at with its six address pins low.</summary>
    public const byte DefaultAddress = NativeMethods.Pca9685DefaultAddress;

    /// <summary>The part's internal oscillator frequency, in hertz.</summary>
    public const uint InternalOscHz = 25_000_000;

    /// <summary>How many channels it drives.</summary>
    public const byte Channels = 16;

    /// <summary>How many counts each period is divided into.</summary>
    public const ushort Counts = 4096;

    /// <summary>How long the oscillator takes to run once woken, in microseconds.</summary>
    public const uint OscillatorStartupMicros = NativeMethods.Pca9685OscillatorStartupMicros;

    /// <summary>MODE1 at power-up: asleep, answering the All Call address.</summary>
    public const byte Mode1Reset = NativeMethods.Pca9685Mode1Reset;

    /// <summary>MODE2 at power-up: totem-pole outputs.</summary>
    public const byte Mode2Reset = NativeMethods.Pca9685Mode2Reset;

    /// <summary>PRE_SCALE at power-up: 200 Hz on the internal oscillator.</summary>
    public const byte PreScaleReset = NativeMethods.Pca9685PreScaleReset;

    /// <summary>The smallest value the part loads into PRE_SCALE.</summary>
    public const byte PreScaleMin = NativeMethods.Pca9685PreScaleMin;

    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part at <paramref name="address"/> on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <param name="address">The address its A5 to A0 pins select, <see cref="DefaultAddress"/> with all six low.</param>
    /// <param name="frequencyHz">The PWM frequency every channel shares; 50 for hobby servos.</param>
    /// <param name="oscillatorHz">The clock the prescaler divides, for a board that drives EXTCLK.</param>
    /// <param name="outputs">How the outputs are wired, or null for <see cref="Pca9685Outputs.PowerOn"/>.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Pca9685(
        I2cBus bus,
        byte address = DefaultAddress,
        uint frequencyHz = NativeMethods.Pca9685DefaultFrequencyHz,
        uint oscillatorHz = InternalOscHz,
        Pca9685Outputs? outputs = null)
    {
        ArgumentNullException.ThrowIfNull(bus);
        var settings = new PamojaPca9685Settings
        {
            FrequencyHz = frequencyHz,
            OscillatorHz = oscillatorHz,
            Outputs = (outputs ?? Pca9685Outputs.PowerOn).ToNative(),
        };
        IntPtr driver = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held =>
            NativeMethods.pamoja_pca9685_new(held, address, settings, out driver)));
        _handle = new NativeHandle(driver, NativeMethods.pamoja_pca9685_free);
    }

    /// <summary>The prescale value the driver writes for its frequency.</summary>
    public byte Prescale => _handle.Use(NativeMethods.pamoja_pca9685_prescale);

    /// <summary>The frequency the part runs at once the prescaler has rounded the one asked for, in hertz.</summary>
    public float Frequency => _handle.Use(NativeMethods.pamoja_pca9685_frequency);

    /// <summary>Programs the prescale and the output wiring, wakes the oscillator, and restarts the channels.</summary>
    /// <exception cref="PamojaException">The bus failed.</exception>
    public void Init() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_pca9685_init));

    /// <summary>Loads one channel, initializing the part first if it has not been.</summary>
    /// <param name="channel">The output, 0 to 15.</param>
    /// <param name="pwm">The four register bytes the <see cref="Pwm"/> builders make.</param>
    /// <exception cref="PamojaException">The channel is past 15, the setting is not four bytes, or the bus failed.</exception>
    public void SetChannel(byte channel, ReadOnlySpan<byte> pwm)
    {
        PamojaPwm setting = Pwm.Native(pwm);
        Status.ThrowIfError(_handle.Use(driver =>
            NativeMethods.pamoja_pca9685_set_channel(driver, channel, setting)));
    }

    /// <summary>Loads every channel with the same four bytes in one transfer.</summary>
    /// <param name="pwm">The four register bytes the <see cref="Pwm"/> builders make.</param>
    /// <exception cref="PamojaException">The setting is not four bytes, or the bus failed.</exception>
    public void SetAll(ReadOnlySpan<byte> pwm)
    {
        PamojaPwm setting = Pwm.Native(pwm);
        Status.ThrowIfError(_handle.Use(driver => NativeMethods.pamoja_pca9685_set_all(driver, setting)));
    }

    /// <summary>Stops the oscillator; the channels keep their settings.</summary>
    /// <exception cref="PamojaException">The bus failed.</exception>
    public void Sleep() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_pca9685_sleep));

    /// <summary>Wakes the oscillator, waits the 500 us it needs, and restarts the channels.</summary>
    /// <exception cref="PamojaException">The bus failed.</exception>
    public void Wake() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_pca9685_wake));

    /// <summary>Sends the general-call software reset to every PCA9685 on the bus.</summary>
    /// <remarks>It goes to address <c>0x00</c>, which nothing on a simulated bus answers.</remarks>
    /// <exception cref="PamojaException">The bus failed, or nothing acknowledged the general call.</exception>
    public void SoftwareReset() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_pca9685_software_reset));

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

    /// <summary>Returns the first of a channel's four consecutive registers.</summary>
    /// <param name="channel">The channel, 0 to 15.</param>
    /// <returns>The register address.</returns>
    /// <exception cref="PamojaException">The channel is beyond the part.</exception>
    public static byte ChannelRegister(byte channel)
    {
        Status.ThrowIfError(
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

    /// <summary>The registers a driver writes and a program reads back.</summary>
    public static class Register
    {
        /// <summary>Mode register 1: restart, clock, auto-increment, sleep, and the addresses answered.</summary>
        public const byte Mode1 = NativeMethods.Pca9685RegisterMode1;

        /// <summary>Mode register 2: how the outputs are wired and when they change.</summary>
        public const byte Mode2 = NativeMethods.Pca9685RegisterMode2;

        /// <summary>The first of channel 0's four registers; <see cref="ChannelRegister"/> gives the rest.</summary>
        public const byte Led0OnL = NativeMethods.Pca9685RegisterLed0OnL;

        /// <summary>The first of the four registers that load every channel at once.</summary>
        public const byte AllLedOnL = NativeMethods.Pca9685RegisterAllLedOnL;

        /// <summary>The prescaler, writable only while the part sleeps.</summary>
        public const byte PreScale = NativeMethods.Pca9685RegisterPreScale;
    }

    /// <summary>MODE1's bits.</summary>
    public static class Mode1
    {
        /// <summary>Set when the part slept with a channel running; cleared by a written 1.</summary>
        public const byte Restart = NativeMethods.Pca9685Mode1Restart;

        /// <summary>The prescaler divides the EXTCLK pin rather than the oscillator.</summary>
        public const byte Extclk = NativeMethods.Pca9685Mode1Extclk;

        /// <summary>The register pointer moves on after each byte.</summary>
        public const byte AutoIncrement = NativeMethods.Pca9685Mode1AutoIncrement;

        /// <summary>The oscillator is off and PRE_SCALE takes a write.</summary>
        public const byte Sleep = NativeMethods.Pca9685Mode1Sleep;
    }

    /// <summary>A PCA9685 that is not there, for a bus with nothing plugged in.</summary>
    /// <remarks>
    /// It holds the part's power-on registers and keeps its datasheet's rules: PRE_SCALE takes
    /// a write only while the part sleeps, the register pointer moves on only with
    /// auto-increment set, and one write to the ALL_LED registers loads every channel.
    /// </remarks>
    public static class Sim
    {
        /// <summary>Makes a PCA9685 as it powers up: asleep at 200 Hz with every output off.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static I2cPart Part(byte address = DefaultAddress) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_pca9685_sim_part(address),
                NativeMethods.pamoja_i2c_part_free,
                "simulated PCA9685"));
    }
}

/// <summary>How a PCA9685's sixteen outputs are wired, the MODE2 register.</summary>
/// <param name="TotemPole">Totem-pole outputs when true, open-drain when false.</param>
/// <param name="Inverted">Invert the output logic, for a board with no external driver.</param>
/// <param name="ChangeOnAck">Change the outputs on each register write's acknowledge rather than on the stop.</param>
public readonly record struct Pca9685Outputs(bool TotemPole, bool Inverted = false, bool ChangeOnAck = false)
{
    /// <summary>The power-on wiring: totem-pole outputs, not inverted, changing on the stop.</summary>
    public static Pca9685Outputs PowerOn { get; } = new(TotemPole: true);

    /// <summary>Converts to the flat struct the C ABI takes.</summary>
    /// <returns>The interop representation.</returns>
    internal PamojaPca9685Outputs ToNative() => new()
    {
        TotemPole = TotemPole ? (byte)1 : (byte)0,
        Inverted = Inverted ? (byte)1 : (byte)0,
        ChangeOnAck = ChangeOnAck ? (byte)1 : (byte)0,
    };
}
