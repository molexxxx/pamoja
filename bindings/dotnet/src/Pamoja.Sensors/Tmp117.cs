using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Texas Instruments TMP117 high-accuracy thermometer. Every call goes straight to
/// the pamoja C ABI, which decodes exactly what the manufacturer's datasheet specifies.
/// </summary>
public static class Tmp117
{
    /// <summary>Converts a raw TMP117 temperature register to nano-degrees Celsius.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static long NanoCelsius(short raw) =>
        NativeMethods.pamoja_tmp117_nano_celsius(raw);

    /// <summary>Converts a raw TMP117 temperature register to micro-degrees Celsius.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static int MicroCelsius(short raw) =>
        NativeMethods.pamoja_tmp117_micro_celsius(raw);

    /// <summary>Converts a raw TMP117 temperature register to degrees Celsius.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float Celsius(short raw) =>
        NativeMethods.pamoja_tmp117_celsius(raw);

    /// <summary>Builds the TMP117 temperature register that decodes to a temperature.</summary>
    /// <param name="microCelsius">The micro celsius.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static short RawFromMicroCelsius(int microCelsius) =>
        NativeMethods.pamoja_tmp117_raw_from_micro_celsius(microCelsius);

    /// <summary>Builds the TMP117 temperature register that decodes to a temperature in Celsius.</summary>
    /// <param name="celsius">The celsius.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static short RawFromCelsius(float celsius) =>
        NativeMethods.pamoja_tmp117_raw_from_celsius(celsius);

    /// <summary>Builds the two bytes a TMP117 sends for a temperature register.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] TemperatureBytes(short raw)
    {
        byte[] bytes = new byte[NativeMethods.Tmp117RegisterLen];
        Status.ThrowIfError(NativeMethods.pamoja_tmp117_temperature_bytes(raw, bytes));
        return bytes;
    }

    /// <summary>Reads the two bytes a TMP117 sends for a temperature register.</summary>
    /// <param name="bytes">The bytes.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static short TemperatureFromBytes(ReadOnlySpan<byte> bytes)
    {
        short value;
        Status.ThrowIfError(NativeMethods.pamoja_tmp117_temperature_from_bytes(bytes, (nuint)bytes.Length, out value));
        return value;
    }

    /// <summary>Reads the device identifier out of a TMP117 device-ID register.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort DeviceId(ushort raw) =>
        NativeMethods.pamoja_tmp117_device_id(raw);

    /// <summary>Reads the die revision out of a TMP117 device-ID register.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static byte Revision(ushort raw) =>
        NativeMethods.pamoja_tmp117_revision(raw);

    /// <summary>Reports whether a TMP117 configuration register flags a high alert.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool HighAlert(ushort config) =>
        NativeMethods.pamoja_tmp117_high_alert(config);

    /// <summary>Reports whether a TMP117 configuration register flags a low alert.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool LowAlert(ushort config) =>
        NativeMethods.pamoja_tmp117_low_alert(config);

    /// <summary>Reports whether a TMP117 configuration register says a result is ready.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool DataReady(ushort config) =>
        NativeMethods.pamoja_tmp117_data_ready(config);

    /// <summary>Reports whether a TMP117 configuration register says an EEPROM write is running.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool EepromBusy(ushort config) =>
        NativeMethods.pamoja_tmp117_eeprom_busy(config);

    /// <summary>Reports whether a TMP117 EEPROM unlock register says a write is running.</summary>
    /// <param name="unlock">The unlock.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool EepromUnlockBusy(ushort unlock) =>
        NativeMethods.pamoja_tmp117_eeprom_unlock_busy(unlock);

    /// <summary>Assembles the 16-bit TMP117 configuration register value.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort ConfigBits(PamojaTmp117Config config) =>
        NativeMethods.pamoja_tmp117_config_bits(config);

    /// <summary>Parses a 16-bit TMP117 configuration register value.</summary>
    /// <param name="bits">The bits.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaTmp117Config ConfigFromBits(ushort bits)
    {
        PamojaTmp117Config value;
        Status.ThrowIfError(NativeMethods.pamoja_tmp117_config_from_bits(bits, out value));
        return value;
    }

    /// <summary>Returns how many conversions a TMP117 averaging code folds into one result.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static byte AveragingConversions(byte code) =>
        NativeMethods.pamoja_tmp117_averaging_conversions(code);

    /// <summary>Returns how long a TMP117 averaging code takes to convert.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint AveragingMicros(byte code) =>
        NativeMethods.pamoja_tmp117_averaging_micros(code);

    /// <summary>Returns the nominal cycle a TMP117 conversion-cycle code selects.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint CycleNominalMicros(byte code) =>
        NativeMethods.pamoja_tmp117_cycle_nominal_micros(code);

    /// <summary>Returns how often a TMP117 updates its result for a cycle and averaging code.</summary>
    /// <param name="cycle">The cycle.</param>
    /// <param name="averaging">The averaging.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint CycleMicros(byte cycle, byte averaging) =>
        NativeMethods.pamoja_tmp117_cycle_micros(cycle, averaging);
}
