using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Texas Instruments OPT3001 ambient light sensor. Every call goes straight to the
/// pamoja C ABI, which decodes exactly what the manufacturer's datasheet specifies.
/// </summary>
public static class Opt3001
{
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
}
