using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Texas Instruments OPT3001 ambient light sensor, and a driver for one on an I2C bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet, decoded exactly as the manufacturer
/// specifies, and <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives one part over an <see cref="I2cBus"/>, measuring on demand in
/// single-shot mode; the part returns itself to shutdown after each conversion. Nothing is sent
/// until <see cref="Init"/> or the first <see cref="Measure"/>.
/// </para>
/// </remarks>
public sealed class Opt3001 : IDisposable
{
    /// <summary>The address with ADDR tied to ground.</summary>
    public const byte AddressGnd = 0x44;

    /// <summary>The address with ADDR tied to VDD.</summary>
    public const byte AddressVdd = 0x45;

    /// <summary>The address with ADDR tied to SDA.</summary>
    public const byte AddressSda = 0x46;

    /// <summary>The address with ADDR tied to SCL.</summary>
    public const byte AddressScl = 0x47;

    /// <summary>The range number that lets the part choose its own full-scale range.</summary>
    public const byte RangeAutomatic = 12;

    /// <summary>The highest fixed range number, 83865.60 lux full scale.</summary>
    public const byte RangeMax = 11;

    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part at <paramref name="address"/> on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <param name="address">One of the <c>Address</c> constants, by where ADDR is tied.</param>
    /// <param name="conversionTime">How long each conversion integrates.</param>
    /// <param name="range">A fixed range number up to <see cref="RangeMax"/>, or <see cref="RangeAutomatic"/>.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Opt3001(
        I2cBus bus,
        byte address,
        ConversionTime conversionTime = ConversionTime.Ms800,
        byte range = RangeAutomatic)
    {
        ArgumentNullException.ThrowIfNull(bus);
        var settings = new PamojaOpt3001Settings
        {
            LongConversion = conversionTime == ConversionTime.Ms800 ? (byte)1 : (byte)0,
            RangeNumber = range,
        };
        IntPtr sensor = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held =>
            NativeMethods.pamoja_opt3001_new(held, address, settings, out sensor)));
        _handle = new NativeHandle(sensor, NativeMethods.pamoja_opt3001_free);
    }

    /// <summary>How long each conversion integrates.</summary>
    public enum ConversionTime : byte
    {
        /// <summary>100 ms, for speed.</summary>
        Ms100 = 0,

        /// <summary>800 ms, for resolution; the part's reset setting.</summary>
        Ms800 = 1,
    }

    /// <summary>The configuration the driver writes, with the part in shutdown.</summary>
    public PamojaOpt3001Config Configuration
    {
        get
        {
            PamojaOpt3001Config config = default;
            Status.ThrowIfError(_handle.Use(sensor =>
                NativeMethods.pamoja_opt3001_configuration(sensor, out config)));
            return config;
        }
    }

    /// <summary>Checks the part is an OPT3001 and writes the settings with the part in shutdown.</summary>
    /// <exception cref="PamojaException">Nothing answered, or either id register is not an OPT3001's.</exception>
    public void Init() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_opt3001_init));

    /// <summary>Runs one conversion and reads the illuminance, initializing the part first if needed.</summary>
    /// <returns>The illuminance.</returns>
    /// <exception cref="PamojaException">As <see cref="Init"/>, and when the conversion never finishes.</exception>
    public Opt3001Reading Measure()
    {
        PamojaOpt3001Reading reading = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_opt3001_measure(sensor, out reading)));
        return new Opt3001Reading(reading.Raw, reading.MilliLux, reading.Lux);
    }

    /// <summary>Writes the low and high limits the part's interrupt pin compares each result against.</summary>
    /// <param name="lowMilliLux">The low limit, in millilux.</param>
    /// <param name="highMilliLux">The high limit, in millilux.</param>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public void SetLimits(uint lowMilliLux, uint highMilliLux) =>
        Status.ThrowIfError(_handle.Use(sensor =>
            NativeMethods.pamoja_opt3001_set_limits(sensor, lowMilliLux, highMilliLux)));

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

    /// <summary>Returns the illuminance one count carries at an OPT3001 exponent.</summary>
    /// <param name="exponent">The exponent.</param>
    /// <returns>The value, or null where the part defines none.</returns>
    public static uint? LsbMilliLux(byte exponent)
    {
        uint value;
        return NativeMethods.pamoja_opt3001_lsb_milli_lux(exponent, out value) ? value : null;
    }

    /// <summary>Returns the full scale an OPT3001 range number covers.</summary>
    /// <param name="rangeNumber">The range number.</param>
    /// <returns>The value, or null where the part defines none.</returns>
    public static uint? FullScaleMilliLux(byte rangeNumber)
    {
        uint value;
        return NativeMethods.pamoja_opt3001_full_scale_milli_lux(rangeNumber, out value) ? value : null;
    }

    /// <summary>Converts a raw OPT3001 result register to milli-lux.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint MilliLux(ushort raw) =>
        NativeMethods.pamoja_opt3001_milli_lux(raw);

    /// <summary>Converts a raw OPT3001 result register to lux.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float Lux(ushort raw) =>
        NativeMethods.pamoja_opt3001_lux(raw);

    /// <summary>Builds the OPT3001 result register that decodes to an illuminance.</summary>
    /// <param name="milliLux">The milli lux.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort RawFromMilliLux(uint milliLux) =>
        NativeMethods.pamoja_opt3001_raw_from_milli_lux(milliLux);

    /// <summary>Reads the two bytes an OPT3001 sends for a register, most significant first.</summary>
    /// <param name="bytes">The bytes.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static ushort WordFromBytes(ReadOnlySpan<byte> bytes)
    {
        ushort value;
        Status.ThrowIfError(NativeMethods.pamoja_opt3001_word_from_bytes(bytes, (nuint)bytes.Length, out value));
        return value;
    }

    /// <summary>Builds the two bytes an OPT3001 sends for a register, most significant first.</summary>
    /// <param name="word">The word.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] WordToBytes(ushort word)
    {
        byte[] bytes = new byte[NativeMethods.Opt3001RegisterLen];
        Status.ThrowIfError(NativeMethods.pamoja_opt3001_word_to_bytes(word, bytes));
        return bytes;
    }

    /// <summary>Assembles the 16-bit OPT3001 configuration register value.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort ConfigBits(PamojaOpt3001Config config) =>
        NativeMethods.pamoja_opt3001_config_bits(config);

    /// <summary>Parses a 16-bit OPT3001 configuration register value.</summary>
    /// <param name="bits">The bits.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaOpt3001Config ConfigFromBits(ushort bits)
    {
        PamojaOpt3001Config value;
        Status.ThrowIfError(NativeMethods.pamoja_opt3001_config_from_bits(bits, out value));
        return value;
    }

    /// <summary>Returns the conversion time an OPT3001 setting selects.</summary>
    /// <param name="longConversion">The long conversion.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort ConversionMillis(bool longConversion) =>
        NativeMethods.pamoja_opt3001_conversion_millis(longConversion);

    /// <summary>Returns how many consecutive faults an OPT3001 fault-count code requires.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static byte FaultCount(byte code) =>
        NativeMethods.pamoja_opt3001_fault_count(code);

    /// <summary>Reports whether an OPT3001 range number sets the full scale automatically.</summary>
    /// <param name="rangeNumber">The range number.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool IsAutomaticRange(byte rangeNumber) =>
        NativeMethods.pamoja_opt3001_is_automatic_range(rangeNumber);

    /// <summary>An OPT3001 that is not there, for a bus with nothing plugged in.</summary>
    /// <remarks>
    /// Its configuration register keeps the flags the part sets for itself whatever a driver
    /// writes, with the conversion-ready flag set, so every conversion reads as finished.
    /// </remarks>
    public static class Sim
    {
        /// <summary>The illuminance <see cref="Part"/> reports.</summary>
        public const float Lux = 380.0f;

        /// <summary>Makes a part reading <see cref="Lux"/>.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Part(byte address) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_opt3001_sim_part(address),
                NativeMethods.pamoja_i2c_part_free,
                "simulated OPT3001"));

        /// <summary>Makes a part that reads what it is asked to.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <param name="lux">The illuminance it reports, which lands on the nearest step its exponent and mantissa represent.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Reporting(byte address, float lux) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_opt3001_sim_reporting(address, lux),
                NativeMethods.pamoja_i2c_part_free,
                "simulated OPT3001"));
    }
}
