using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Texas Instruments INA219 current, voltage, and power monitor, and a driver for one on an
/// I2C bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet, decoded exactly as the manufacturer
/// specifies, and <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives one part over an <see cref="I2cBus"/>: it resets the part, writes the
/// configuration, programs the calibration for the shunt and the current it is sized for, and
/// reads the calibration back, the identity check a part with no id register allows. Each
/// <see cref="Measure"/> triggers one shunt and bus conversion. Nothing is sent until
/// <see cref="Init"/> or the first <see cref="Measure"/>.
/// </para>
/// </remarks>
public sealed class Ina219 : IDisposable
{
    /// <summary>The address with A1 and A0 tied to ground; the pins add to it.</summary>
    public const byte BaseAddress = 0x40;

    /// <summary>The configuration register's power-on value.</summary>
    public const ushort ConfigReset = NativeMethods.Ina219ConfigReset;

    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part at <paramref name="address"/> on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <param name="address"><see cref="BaseAddress"/> plus what A1 and A0 add.</param>
    /// <param name="shuntMilliohms">The shunt resistance in milliohms; the common breakout's is 100.</param>
    /// <param name="maxMicroamps">The largest current the shunt will carry, which sets the finest current step.</param>
    /// <param name="currentLsbMicroamps">A current step to use instead, such as a round 100, or 0 for the finest.</param>
    /// <param name="config">The range, gain, and converter settings, or null for <see cref="Ina219Config.PowerOn"/>.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Ina219(
        I2cBus bus,
        byte address,
        uint shuntMilliohms = 100,
        uint maxMicroamps = 3_200_000,
        uint currentLsbMicroamps = 0,
        Ina219Config? config = null)
    {
        ArgumentNullException.ThrowIfNull(bus);
        var settings = new PamojaIna219Settings
        {
            ShuntMilliohms = shuntMilliohms,
            MaxMicroamps = maxMicroamps,
            CurrentLsbMicroamps = currentLsbMicroamps,
            Config = (config ?? Ina219Config.PowerOn).ToNative(),
        };
        IntPtr sensor = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held =>
            NativeMethods.pamoja_ina219_new(held, address, settings, out sensor)));
        _handle = new NativeHandle(sensor, NativeMethods.pamoja_ina219_free);
    }

    /// <summary>What an address pin is tied to, as the code <see cref="Address"/> takes.</summary>
    public enum AddressPin : byte
    {
        /// <summary>Tied to GND.</summary>
        Ground = 0,

        /// <summary>Tied to VS+.</summary>
        Supply = 1,

        /// <summary>Tied to SDA.</summary>
        Sda = 2,

        /// <summary>Tied to SCL.</summary>
        Scl = 3,
    }

    /// <summary>The bus-voltage range, as its register code.</summary>
    public enum BusRange : byte
    {
        /// <summary>0 to 16 V.</summary>
        V16 = 0,

        /// <summary>0 to 32 V, the reset setting.</summary>
        V32 = 1,
    }

    /// <summary>The shunt gain and the shunt-voltage range it gives, as its register code.</summary>
    public enum Gain : byte
    {
        /// <summary>Gain 1, 40 mV either side of zero.</summary>
        Div1 = 0,

        /// <summary>Gain 1/2, 80 mV.</summary>
        Div2 = 1,

        /// <summary>Gain 1/4, 160 mV.</summary>
        Div4 = 2,

        /// <summary>Gain 1/8, 320 mV, the reset setting.</summary>
        Div8 = 3,
    }

    /// <summary>A converter's resolution, or the samples it averages at 12 bits, as its register code.</summary>
    public enum Adc : byte
    {
        /// <summary>9 bits, 84 us.</summary>
        Bits9 = 0b0000,

        /// <summary>10 bits, 148 us.</summary>
        Bits10 = 0b0001,

        /// <summary>11 bits, 276 us.</summary>
        Bits11 = 0b0010,

        /// <summary>12 bits, 532 us, the reset setting.</summary>
        Bits12 = 0b0011,

        /// <summary>2 samples averaged, 1.06 ms.</summary>
        Samples2 = 0b1001,

        /// <summary>4 samples averaged, 2.13 ms.</summary>
        Samples4 = 0b1010,

        /// <summary>8 samples averaged, 4.26 ms.</summary>
        Samples8 = 0b1011,

        /// <summary>16 samples averaged, 8.51 ms.</summary>
        Samples16 = 0b1100,

        /// <summary>32 samples averaged, 17.02 ms.</summary>
        Samples32 = 0b1101,

        /// <summary>64 samples averaged, 34.05 ms.</summary>
        Samples64 = 0b1110,

        /// <summary>128 samples averaged, 68.10 ms.</summary>
        Samples128 = 0b1111,
    }

    /// <summary>The operating mode, as its register code.</summary>
    public enum Mode : byte
    {
        /// <summary>No conversions, lowest power.</summary>
        PowerDown = 0,

        /// <summary>One shunt conversion.</summary>
        ShuntTriggered = 1,

        /// <summary>One bus conversion.</summary>
        BusTriggered = 2,

        /// <summary>One shunt and one bus conversion.</summary>
        ShuntAndBusTriggered = 3,

        /// <summary>The converter disabled.</summary>
        AdcOff = 4,

        /// <summary>Shunt conversions back to back.</summary>
        ShuntContinuous = 5,

        /// <summary>Bus conversions back to back.</summary>
        BusContinuous = 6,

        /// <summary>Shunt and bus conversions back to back, the reset setting.</summary>
        ShuntAndBusContinuous = 7,
    }

    /// <summary>The current step the driver programs, in microamps per count.</summary>
    public uint CurrentLsbMicroamps => _handle.Use(NativeMethods.pamoja_ina219_current_lsb);

    /// <summary>The calibration word the driver programs.</summary>
    public ushort CalibrationWord => _handle.Use(NativeMethods.pamoja_ina219_calibration_word);

    /// <summary>Resets the part and programs the configuration and calibration.</summary>
    /// <exception cref="PamojaException">Nothing answered, or the calibration did not read back.</exception>
    public void Init() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_ina219_init));

    /// <summary>Triggers one shunt and bus conversion and reads every result, initializing the part first if needed.</summary>
    /// <returns>The registers as read, and what they mean.</returns>
    /// <exception cref="PamojaException">As <see cref="Init"/>, and when the conversion-ready flag never sets.</exception>
    public Ina219Reading Measure()
    {
        PamojaIna219Reading reading = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_ina219_measure(sensor, out reading)));
        return Ina219Reading.From(reading);
    }

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

    /// <summary>Returns the 7-bit address the A1 and A0 pins select, from Table 1 of the datasheet.</summary>
    /// <param name="a1">What the A1 pin is tied to.</param>
    /// <param name="a0">What the A0 pin is tied to.</param>
    /// <returns>The address, 0x40 to 0x4F.</returns>
    /// <exception cref="PamojaException">A pin code is not one of the four levels.</exception>
    public static byte Address(AddressPin a1, AddressPin a0)
    {
        Status.ThrowIfError(NativeMethods.pamoja_ina219_address((byte)a1, (byte)a0, out byte address));
        return address;
    }

    /// <summary>Assembles the configuration register value.</summary>
    /// <param name="config">The settings.</param>
    /// <returns>The register value to write.</returns>
    public static ushort ConfigBits(Ina219Config config) =>
        NativeMethods.pamoja_ina219_config_bits(config.ToNative());

    /// <summary>Parses a configuration register value.</summary>
    /// <param name="bits">The register value, as read from the part.</param>
    /// <returns>The settings. Every value decodes.</returns>
    public static Ina219Config ConfigFromBits(ushort bits)
    {
        Status.ThrowIfError(NativeMethods.pamoja_ina219_config_from_bits(bits, out PamojaIna219Config config));
        return Ina219Config.From(config);
    }

    /// <summary>Returns how long one conversion cycle takes: the shunt and bus conversions the mode runs.</summary>
    /// <param name="config">The settings.</param>
    /// <returns>The time in microseconds.</returns>
    public static uint ConversionMicros(Ina219Config config) =>
        NativeMethods.pamoja_ina219_conversion_micros(config.ToNative());

    /// <summary>Returns how long one conversion takes at a converter setting.</summary>
    /// <param name="adc">The setting.</param>
    /// <returns>The time in microseconds, from the datasheet's table.</returns>
    public static uint AdcConversionMicros(Adc adc) =>
        NativeMethods.pamoja_ina219_adc_conversion_micros((byte)adc);

    /// <summary>Returns the shunt-voltage range a gain selects.</summary>
    /// <param name="gain">The gain.</param>
    /// <returns>The range in millivolts either side of zero.</returns>
    public static ushort GainRangeMillivolts(Gain gain) =>
        NativeMethods.pamoja_ina219_gain_range_millivolts((byte)gain);

    /// <summary>Computes the calibration register for a shunt and resolution.</summary>
    /// <param name="currentLsbMicroamps">The microamps per count wanted.</param>
    /// <param name="shuntMilliohms">The shunt resistor value.</param>
    /// <returns>The register value to write.</returns>
    public static ushort Calibration(uint currentLsbMicroamps, uint shuntMilliohms) =>
        NativeMethods.pamoja_ina219_calibration(currentLsbMicroamps, shuntMilliohms);

    /// <summary>Returns the smallest resolution that still covers a maximum.</summary>
    /// <param name="maxExpectedMicroamps">The largest current measured.</param>
    /// <returns>The minimum current LSB in microamps.</returns>
    public static uint MinimumCurrentLsbMicroamps(uint maxExpectedMicroamps) =>
        NativeMethods.pamoja_ina219_minimum_current_lsb_microamps(maxExpectedMicroamps);

    /// <summary>Builds the shunt-voltage register a monitor reports for a shunt voltage.</summary>
    /// <remarks>
    /// The inverse of <see cref="ShuntMicrovolts"/>, so a node can be written and tested
    /// against what a monitor sends without one attached.
    /// </remarks>
    /// <param name="microvolts">The shunt voltage in microvolts.</param>
    /// <returns>The signed register value, at 10 uV per count.</returns>
    public static short ShuntRegister(int microvolts) =>
        NativeMethods.pamoja_ina219_shunt_register(microvolts);

    /// <summary>Builds the bus-voltage register a monitor reports for a bus voltage.</summary>
    /// <param name="millivolts">The bus voltage in millivolts.</param>
    /// <returns>The register value, with the conversion-ready flag set.</returns>
    public static ushort BusRegister(uint millivolts) =>
        NativeMethods.pamoja_ina219_bus_register(millivolts);

    /// <summary>Builds the current register a monitor reports for a current.</summary>
    /// <param name="microamps">The current in microamps.</param>
    /// <param name="currentLsbMicroamps">The current LSB the calibration was set for.</param>
    /// <returns>The signed register value.</returns>
    public static short CurrentRegister(int microamps, uint currentLsbMicroamps) =>
        NativeMethods.pamoja_ina219_current_register(microamps, currentLsbMicroamps);

    /// <summary>Builds the power register a monitor reports for a power.</summary>
    /// <param name="microwatts">The power in microwatts.</param>
    /// <param name="currentLsbMicroamps">The current LSB the calibration was set for.</param>
    /// <returns>The register value.</returns>
    public static ushort PowerRegister(uint microwatts, uint currentLsbMicroamps) =>
        NativeMethods.pamoja_ina219_power_register(microwatts, currentLsbMicroamps);

    /// <summary>Converts a raw shunt-voltage register to microvolts.</summary>
    /// <param name="raw">The signed register value.</param>
    /// <returns>The shunt voltage.</returns>
    public static int ShuntMicrovolts(short raw) =>
        NativeMethods.pamoja_ina219_shunt_microvolts(raw);

    /// <summary>Converts a raw bus-voltage register to millivolts.</summary>
    /// <param name="raw">The register value.</param>
    /// <returns>The bus voltage.</returns>
    public static uint BusMillivolts(ushort raw) =>
        NativeMethods.pamoja_ina219_bus_millivolts(raw);

    /// <summary>Reports whether a bus-voltage register says a conversion is ready.</summary>
    /// <param name="raw">The register value.</param>
    /// <returns>Whether the conversion-ready flag is set.</returns>
    public static bool ConversionReady(ushort raw) =>
        NativeMethods.pamoja_ina219_conversion_ready(raw);

    /// <summary>Reports whether a bus-voltage register flags a math overflow.</summary>
    /// <param name="raw">The register value.</param>
    /// <returns>
    /// Whether the current and power readings are meaningless, which means the
    /// calibration needs revisiting.
    /// </returns>
    public static bool MathOverflow(ushort raw) =>
        NativeMethods.pamoja_ina219_math_overflow(raw);

    /// <summary>Converts a raw current register to microamps.</summary>
    /// <param name="raw">The signed register value.</param>
    /// <param name="currentLsbMicroamps">The resolution the calibration selected.</param>
    /// <returns>The current.</returns>
    public static int CurrentMicroamps(short raw, uint currentLsbMicroamps) =>
        NativeMethods.pamoja_ina219_current_microamps(raw, currentLsbMicroamps);

    /// <summary>Converts a raw power register to microwatts.</summary>
    /// <param name="raw">The register value.</param>
    /// <param name="currentLsbMicroamps">The resolution the calibration selected.</param>
    /// <returns>The power. The power LSB is fixed at twenty times the current LSB.</returns>
    public static uint PowerMicrowatts(ushort raw, uint currentLsbMicroamps) =>
        NativeMethods.pamoja_ina219_power_microwatts(raw, currentLsbMicroamps);

    /// <summary>An INA219 that is not there, for a bus with nothing plugged in.</summary>
    /// <remarks>
    /// A monitor's current and power registers count in steps the calibration sets, so
    /// <see cref="Reporting"/> takes the same shunt and largest current a driver is given, and
    /// <see cref="Part"/> is the common breakout carrying 500 mA at 12 V through the 100
    /// milliohm shunt a driver starts with.
    /// </remarks>
    public static class Sim
    {
        /// <summary>The shunt <see cref="Part"/> sits across, in milliohms.</summary>
        public const uint ShuntMilliohms = 100;

        /// <summary>The largest current <see cref="Part"/> is sized for, in microamps.</summary>
        public const uint MaxMicroamps = 3_200_000;

        /// <summary>The bus voltage <see cref="Part"/> reports, in millivolts.</summary>
        public const uint BusMillivolts = 12_000;

        /// <summary>The current <see cref="Part"/> reports, in microamps.</summary>
        public const int Microamps = 500_000;

        /// <summary>Makes a part carrying <see cref="Microamps"/> at <see cref="BusMillivolts"/>.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Part(byte address) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_ina219_sim_part(address),
                NativeMethods.pamoja_i2c_part_free,
                "simulated INA219"));

        /// <summary>Makes a part that reads what it is asked to.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <param name="shuntMilliohms">The shunt, as the driver is given it.</param>
        /// <param name="maxMicroamps">The largest current, as the driver is given it.</param>
        /// <param name="busMillivolts">The bus voltage it reports.</param>
        /// <param name="microamps">The current it reports; negative flows the other way through the shunt.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Reporting(
            byte address,
            uint shuntMilliohms,
            uint maxMicroamps,
            uint busMillivolts,
            int microamps) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_ina219_sim_reporting(address, shuntMilliohms, maxMicroamps, busMillivolts, microamps),
                NativeMethods.pamoja_i2c_part_free,
                "simulated INA219"));
    }
}
