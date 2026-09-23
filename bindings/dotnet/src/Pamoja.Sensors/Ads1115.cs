using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Texas Instruments ADS1115 16-bit analog-to-digital converter, and a driver for one on an
/// I2C bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet, decoded exactly as the manufacturer
/// specifies, and <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives one part over an <see cref="I2cBus"/>, converting one input on demand
/// and leaving the part powered down between conversions. The part has no id register, so
/// <see cref="Init"/> writes the configuration and reads it back. Nothing is sent until
/// <see cref="Init"/> or the first <see cref="Sample"/>.
/// </para>
/// </remarks>
public sealed class Ads1115 : IDisposable
{
    /// <summary>The address with ADDR tied to ground.</summary>
    public const byte AddressGnd = 0x48;

    /// <summary>The address with ADDR tied to VDD.</summary>
    public const byte AddressVdd = 0x49;

    /// <summary>The address with ADDR tied to SDA.</summary>
    public const byte AddressSda = 0x4A;

    /// <summary>The address with ADDR tied to SCL.</summary>
    public const byte AddressScl = 0x4B;

    /// <summary>The value the configuration register reads after a reset.</summary>
    public const ushort ConfigReset = 0x8583;

    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part at <paramref name="address"/> on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <param name="address">One of the <c>Address</c> constants, by where ADDR is tied.</param>
    /// <param name="input">The input the part converts.</param>
    /// <param name="gain">The full-scale range, and with it the size of one count.</param>
    /// <param name="rate">The data rate, and with it the conversion time and the noise.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Ads1115(
        I2cBus bus,
        byte address,
        Mux input = Mux.Ain0Ain1,
        Pga gain = Pga.Fsr2_048,
        DataRate rate = DataRate.Sps128)
    {
        ArgumentNullException.ThrowIfNull(bus);
        var settings = new PamojaAds1115Settings
        {
            Mux = (byte)input,
            Pga = (byte)gain,
            DataRate = (byte)rate,
        };
        IntPtr sensor = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held =>
            NativeMethods.pamoja_ads1115_new(held, address, settings, out sensor)));
        _handle = new NativeHandle(sensor, NativeMethods.pamoja_ads1115_free);
    }

    /// <summary>The input multiplexer setting, as its register code.</summary>
    public enum Mux : byte
    {
        /// <summary>AIN0 against AIN1, the reset setting.</summary>
        Ain0Ain1 = 0,

        /// <summary>AIN0 against AIN3.</summary>
        Ain0Ain3 = 1,

        /// <summary>AIN1 against AIN3.</summary>
        Ain1Ain3 = 2,

        /// <summary>AIN2 against AIN3.</summary>
        Ain2Ain3 = 3,

        /// <summary>AIN0 against ground.</summary>
        Ain0Gnd = 4,

        /// <summary>AIN1 against ground.</summary>
        Ain1Gnd = 5,

        /// <summary>AIN2 against ground.</summary>
        Ain2Gnd = 6,

        /// <summary>AIN3 against ground.</summary>
        Ain3Gnd = 7,
    }

    /// <summary>The full-scale range, as its register code.</summary>
    public enum Pga : byte
    {
        /// <summary>6.144 V either side of zero.</summary>
        Fsr6_144 = 0,

        /// <summary>4.096 V.</summary>
        Fsr4_096 = 1,

        /// <summary>2.048 V, the reset setting.</summary>
        Fsr2_048 = 2,

        /// <summary>1.024 V.</summary>
        Fsr1_024 = 3,

        /// <summary>0.512 V.</summary>
        Fsr0_512 = 4,

        /// <summary>0.256 V.</summary>
        Fsr0_256 = 5,
    }

    /// <summary>The data rate, as its register code.</summary>
    public enum DataRate : byte
    {
        /// <summary>8 samples per second.</summary>
        Sps8 = 0,

        /// <summary>16 samples per second.</summary>
        Sps16 = 1,

        /// <summary>32 samples per second.</summary>
        Sps32 = 2,

        /// <summary>64 samples per second.</summary>
        Sps64 = 3,

        /// <summary>128 samples per second, the reset setting.</summary>
        Sps128 = 4,

        /// <summary>250 samples per second.</summary>
        Sps250 = 5,

        /// <summary>475 samples per second.</summary>
        Sps475 = 6,

        /// <summary>860 samples per second.</summary>
        Sps860 = 7,
    }

    /// <summary>The configuration the driver writes.</summary>
    public Ads1115Config Config
    {
        get
        {
            PamojaAds1115Config config = default;
            Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_ads1115_config(sensor, out config)));
            return Ads1115Config.FromNative(config);
        }
    }

    /// <summary>Writes the input, range, and data rate, and reads the configuration back.</summary>
    /// <exception cref="PamojaException">Nothing answered, or the configuration read back differently.</exception>
    public void Init() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_ads1115_init));

    /// <summary>Runs one conversion of the configured input, initializing the part first if needed.</summary>
    /// <returns>The conversion, with the range it ran at.</returns>
    /// <exception cref="PamojaException">As <see cref="Init"/>, and when the conversion never finishes.</exception>
    public Ads1115Sample Sample()
    {
        PamojaAds1115Sample sample = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_ads1115_sample(sensor, out sample)));
        return Ads1115Sample.From(sample);
    }

    /// <summary>Converts another input once, leaving the configured input as it was.</summary>
    /// <param name="input">The input for this one conversion.</param>
    /// <returns>The conversion.</returns>
    /// <exception cref="PamojaException">As <see cref="Sample"/>.</exception>
    public Ads1115Sample SampleInput(Mux input)
    {
        PamojaAds1115Sample sample = default;
        Status.ThrowIfError(_handle.Use(sensor =>
            NativeMethods.pamoja_ads1115_sample_input(sensor, (byte)input, out sample)));
        return Ads1115Sample.From(sample);
    }

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

    /// <summary>Assembles the 16-bit configuration register value.</summary>
    /// <param name="config">The settings to encode.</param>
    /// <returns>The register value to write, most significant bit first.</returns>
    public static ushort ConfigBits(Ads1115Config config)
    {
        ArgumentNullException.ThrowIfNull(config);
        return NativeMethods.pamoja_ads1115_config_bits(config.ToNative());
    }

    /// <summary>Parses a 16-bit configuration register value.</summary>
    /// <param name="bits">The register value, as read from the device.</param>
    /// <returns>The decoded settings. Every value decodes, so this never throws.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static Ads1115Config ConfigFromBits(ushort bits)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_ads1115_config_from_bits(bits, out PamojaAds1115Config config));
        return Ads1115Config.FromNative(config);
    }

    /// <summary>Returns the full-scale range a gain code selects.</summary>
    /// <param name="pga">The gain code, 0 to 7.</param>
    /// <returns>The full scale in microvolts.</returns>
    public static uint FullScaleMicrovolts(byte pga) =>
        NativeMethods.pamoja_ads1115_full_scale_microvolts(pga);

    /// <summary>Returns the sample rate a data-rate code selects.</summary>
    /// <param name="dataRate">The data-rate code, 0 to 7.</param>
    /// <returns>The rate in samples per second.</returns>
    public static ushort SamplesPerSecond(byte dataRate) =>
        NativeMethods.pamoja_ads1115_samples_per_second(dataRate);

    /// <summary>Returns how long a conversion takes at a data rate: one period plus the datasheet's ten percent variation.</summary>
    /// <param name="rate">The data rate.</param>
    /// <returns>The time in microseconds.</returns>
    public static uint ConversionMicros(DataRate rate) =>
        NativeMethods.pamoja_ads1115_conversion_micros((byte)rate);

    /// <summary>Converts a raw conversion result to nanovolts.</summary>
    /// <param name="pga">The gain the conversion was taken at.</param>
    /// <param name="raw">The signed conversion register value.</param>
    /// <returns>The measured voltage, exact at every gain setting.</returns>
    public static long ToNanovolts(byte pga, short raw) =>
        NativeMethods.pamoja_ads1115_to_nanovolts(pga, raw);

    /// <summary>Converts a raw conversion result to volts.</summary>
    /// <param name="pga">The gain the conversion was taken at.</param>
    /// <param name="raw">The signed conversion register value.</param>
    /// <returns>The measured voltage.</returns>
    public static float ToVolts(byte pga, short raw) =>
        NativeMethods.pamoja_ads1115_to_volts(pga, raw);

    /// <summary>An ADS1115 that is not there, for a bus with nothing plugged in.</summary>
    /// <remarks>
    /// A conversion comes back as a count of the range the gain selects, so
    /// <see cref="Reporting"/> takes the same gain a driver is given. The part keeps whatever
    /// configuration a driver writes, so a conversion a driver starts reads as finished.
    /// </remarks>
    public static class Sim
    {
        /// <summary>The voltage <see cref="Part"/> reports: half a 3.3 V supply.</summary>
        public const float Volts = 1.65f;

        /// <summary>Makes a part reading <see cref="Volts"/> at the range a driver starts with.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Part(byte address) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_ads1115_sim_part(address),
                NativeMethods.pamoja_i2c_part_free,
                "simulated ADS1115"));

        /// <summary>Makes a part that reads what it is asked to at the range a driver converts at.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <param name="gain">The range the driver converts at.</param>
        /// <param name="volts">The voltage it reports, held to the range.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Reporting(byte address, Pga gain, float volts) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_ads1115_sim_reporting(address, (byte)gain, volts),
                NativeMethods.pamoja_i2c_part_free,
                "simulated ADS1115"));
    }
}
