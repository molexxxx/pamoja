using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Texas Instruments TMP117 high-accuracy thermometer, and a driver for one on an I2C bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet, decoded exactly as the manufacturer
/// specifies, and <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives one part over an <see cref="I2cBus"/>, converting on demand in one-shot
/// mode and leaving the part powered down between conversions. Nothing is sent until
/// <see cref="Init"/> or the first <see cref="Measure"/>. The driver holds its own share of the
/// bus, so the bus may be disposed once the driver is built.
/// </para>
/// </remarks>
public sealed class Tmp117 : IDisposable
{
    /// <summary>The address with ADD0 tied to ground.</summary>
    public const byte AddressAdd0Gnd = 0x48;

    /// <summary>The address with ADD0 tied to V+.</summary>
    public const byte AddressAdd0Vplus = 0x49;

    /// <summary>The address with ADD0 tied to SDA.</summary>
    public const byte AddressAdd0Sda = 0x4A;

    /// <summary>The address with ADD0 tied to SCL.</summary>
    public const byte AddressAdd0Scl = 0x4B;

    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part at <paramref name="address"/> on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <param name="address">One of the <c>AddressAdd0</c> constants, by where ADD0 is tied.</param>
    /// <param name="averaging">How many conversions are averaged into each result.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Tmp117(I2cBus bus, byte address, Averaging averaging = Averaging.X8)
    {
        ArgumentNullException.ThrowIfNull(bus);
        IntPtr sensor = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held =>
            NativeMethods.pamoja_tmp117_new(held, address, (byte)averaging, out sensor)));
        _handle = new NativeHandle(sensor, NativeMethods.pamoja_tmp117_free);
    }

    /// <summary>How many conversions are averaged into one result, as its register code.</summary>
    public enum Averaging : byte
    {
        /// <summary>No averaging: each result is one 15.5 ms conversion.</summary>
        None = 0,

        /// <summary>Eight conversions, the factory setting.</summary>
        X8 = 1,

        /// <summary>Thirty-two conversions.</summary>
        X32 = 2,

        /// <summary>Sixty-four conversions.</summary>
        X64 = 3,
    }

    /// <summary>The silicon revision read at initialization, or null before it.</summary>
    public byte? SiliconRevision =>
        _handle.Use(sensor => NativeMethods.pamoja_tmp117_silicon_revision(sensor, out byte revision)
            ? revision
            : (byte?)null);

    /// <summary>Checks the part is a TMP117, waits for its EEPROM, and writes the settings with the part in shutdown.</summary>
    /// <exception cref="PamojaException">
    /// Nothing answered, the device id is not a TMP117's, or the EEPROM never reported ready.
    /// </exception>
    public void Init() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_tmp117_init));

    /// <summary>Runs one conversion and reads the temperature, initializing the part first if needed.</summary>
    /// <returns>The temperature.</returns>
    /// <exception cref="PamojaException">As <see cref="Init"/>, and when the conversion never finishes.</exception>
    public Tmp117Reading Measure()
    {
        PamojaTmp117Reading reading = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_tmp117_measure(sensor, out reading)));
        return Tmp117Reading.From(reading);
    }

    /// <summary>Writes the high and low limits the part compares each result against.</summary>
    /// <param name="highCelsius">The high limit; the factory value is 192 C.</param>
    /// <param name="lowCelsius">The low limit; the factory value is -256 C.</param>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public void SetAlertLimits(float highCelsius, float lowCelsius) =>
        Status.ThrowIfError(_handle.Use(sensor =>
            NativeMethods.pamoja_tmp117_set_alert_limits(sensor, highCelsius, lowCelsius)));

    /// <summary>Reads the alert flags, including results the driver's own reads saw since the last call.</summary>
    /// <returns>Whether a result was above the high limit or below the low limit.</returns>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public Tmp117Alerts Alerts()
    {
        PamojaTmp117Alerts alerts = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_tmp117_alerts(sensor, out alerts)));
        return new Tmp117Alerts(alerts.High != 0, alerts.Low != 0);
    }

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

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

    /// <summary>A TMP117 that is not there, for a bus with nothing plugged in.</summary>
    /// <remarks>
    /// Its configuration register keeps the flags the part sets for itself whatever a driver
    /// writes, with the data-ready flag set, so every conversion reads as finished.
    /// </remarks>
    public static class Sim
    {
        /// <summary>The temperature <see cref="Part"/> reports.</summary>
        public const float Celsius = 21.25f;

        /// <summary>Makes a part reading <see cref="Celsius"/>.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Part(byte address) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_tmp117_sim_part(address),
                NativeMethods.pamoja_i2c_part_free,
                "simulated TMP117"));

        /// <summary>Makes a part that reads what it is asked to.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <param name="celsius">The temperature it reports, which lands on the nearest 7.8125 millidegrees.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Reporting(byte address, float celsius) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_tmp117_sim_reporting(address, celsius),
                NativeMethods.pamoja_i2c_part_free,
                "simulated TMP117"));
    }
}
