using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>A compensated BME280 reading.</summary>
public sealed class Bme280Measurement
{
    /// <summary>Creates a reading from the compensated values.</summary>
    /// <param name="celsius">The temperature in degrees Celsius.</param>
    /// <param name="pascals">The pressure in pascals.</param>
    /// <param name="hectopascals">The pressure in hectopascals.</param>
    /// <param name="relativeHumidityPercent">The relative humidity as a percentage.</param>
    internal Bme280Measurement(
        float celsius,
        uint pascals,
        float hectopascals,
        float relativeHumidityPercent)
    {
        Celsius = celsius;
        Pascals = pascals;
        Hectopascals = hectopascals;
        RelativeHumidityPercent = relativeHumidityPercent;
    }

    /// <summary>The temperature in degrees Celsius.</summary>
    public float Celsius { get; }

    /// <summary>The pressure in pascals.</summary>
    public uint Pascals { get; }

    /// <summary>The pressure in hectopascals, as a barometer is usually quoted.</summary>
    public float Hectopascals { get; }

    /// <summary>The relative humidity as a percentage.</summary>
    public float RelativeHumidityPercent { get; }
}

/// <summary>A decoded DS18B20 scratchpad.</summary>
public sealed class Ds18b20Reading
{
    /// <summary>Creates a reading from the decoded scratchpad fields.</summary>
    /// <param name="rawTemperature">The raw temperature register.</param>
    /// <param name="microCelsius">The temperature in micro-degrees Celsius.</param>
    /// <param name="alarmHigh">The high alarm threshold.</param>
    /// <param name="alarmLow">The low alarm threshold.</param>
    /// <param name="resolutionBits">The configured resolution in bits.</param>
    internal Ds18b20Reading(
        short rawTemperature,
        int microCelsius,
        sbyte alarmHigh,
        sbyte alarmLow,
        byte resolutionBits)
    {
        RawTemperature = rawTemperature;
        MicroCelsius = microCelsius;
        AlarmHigh = alarmHigh;
        AlarmLow = alarmLow;
        ResolutionBits = resolutionBits;
    }

    /// <summary>The raw temperature register, 1/16 degree Celsius per count.</summary>
    public short RawTemperature { get; }

    /// <summary>The temperature in micro-degrees Celsius, exact in integers.</summary>
    public int MicroCelsius { get; }

    /// <summary>The temperature in degrees Celsius.</summary>
    public float Celsius => MicroCelsius / 1_000_000f;

    /// <summary>The high alarm threshold in whole degrees Celsius.</summary>
    public sbyte AlarmHigh { get; }

    /// <summary>The low alarm threshold in whole degrees Celsius.</summary>
    public sbyte AlarmLow { get; }

    /// <summary>The configured resolution in bits: 9, 10, 11, or 12.</summary>
    public byte ResolutionBits { get; }
}

/// <summary>An ADS1115 configuration register, field by field.</summary>
/// <remarks>
/// The multi-way settings carry the code the datasheet prints; the single-bit
/// settings are booleans named for the state they select. The defaults are the
/// part's own reset state.
/// </remarks>
public sealed class Ads1115Config
{
    /// <summary>Whether writing this starts a single conversion.</summary>
    public bool StartConversion { get; init; } = true;

    /// <summary>The input multiplexer code, 0 to 7.</summary>
    public byte Mux { get; init; }

    /// <summary>The gain code, 0 to 7, which sets the full-scale range.</summary>
    public byte Pga { get; init; } = 2;

    /// <summary>Whether to convert once per request rather than continuously.</summary>
    public bool SingleShot { get; init; } = true;

    /// <summary>The data rate code, 0 to 7.</summary>
    public byte DataRate { get; init; } = 4;

    /// <summary>Whether to use the window comparator rather than the traditional one.</summary>
    public bool WindowComparator { get; init; }

    /// <summary>Whether the ALERT/RDY pin is active high.</summary>
    public bool ComparatorActiveHigh { get; init; }

    /// <summary>Whether the comparator latches until the conversion is read.</summary>
    public bool ComparatorLatching { get; init; }

    /// <summary>The comparator queue code, 0 to 3, where 3 disables it.</summary>
    public byte ComparatorQueue { get; init; } = 3;

    /// <summary>Converts to the flat struct the C ABI takes.</summary>
    /// <returns>The interop representation.</returns>
    internal PamojaAds1115Config ToNative() => new()
    {
        StartConversion = StartConversion ? (byte)1 : (byte)0,
        Mux = Mux,
        Pga = Pga,
        SingleShot = SingleShot ? (byte)1 : (byte)0,
        DataRate = DataRate,
        WindowComparator = WindowComparator ? (byte)1 : (byte)0,
        ComparatorActiveHigh = ComparatorActiveHigh ? (byte)1 : (byte)0,
        ComparatorLatching = ComparatorLatching ? (byte)1 : (byte)0,
        ComparatorQueue = ComparatorQueue,
    };

    /// <summary>Builds a configuration from the flat struct the C ABI returns.</summary>
    /// <param name="native">The interop representation.</param>
    /// <returns>The configuration.</returns>
    internal static Ads1115Config FromNative(PamojaAds1115Config native) => new()
    {
        StartConversion = native.StartConversion != 0,
        Mux = native.Mux,
        Pga = native.Pga,
        SingleShot = native.SingleShot != 0,
        DataRate = native.DataRate,
        WindowComparator = native.WindowComparator != 0,
        ComparatorActiveHigh = native.ComparatorActiveHigh != 0,
        ComparatorLatching = native.ComparatorLatching != 0,
        ComparatorQueue = native.ComparatorQueue,
    };
}

/// <summary>A BME280's factory calibration, read once and reused for every measurement.</summary>
public sealed class Bme280Calibration : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Builds a calibration from the bytes read out of the registers.</summary>
    /// <param name="tempPress">The 26-byte temperature and pressure block.</param>
    /// <param name="humidity">The 7-byte humidity block.</param>
    /// <exception cref="PamojaException">Either block is the wrong length.</exception>
    public Bme280Calibration(ReadOnlySpan<byte> tempPress, ReadOnlySpan<byte> humidity)
    {
        Status.ThrowIfError(NativeMethods.pamoja_bme280_calibration_new(
            tempPress,
            (nuint)tempPress.Length,
            humidity,
            (nuint)humidity.Length,
            out IntPtr calibration));
        _handle = NativeHandle.Create(
            calibration, NativeMethods.pamoja_bme280_calibration_free, "BME280 calibration");
    }

    /// <summary>Turns an eight-byte burst read into a compensated reading.</summary>
    /// <param name="measurement">The eight measurement registers.</param>
    /// <returns>The compensated reading.</returns>
    /// <exception cref="PamojaException">The measurement is not eight bytes.</exception>
    public Bme280Measurement Compensate(ReadOnlySpan<byte> measurement)
    {
        byte[] copy = measurement.ToArray();
        PamojaBme280Measurement reading = _handle.Use(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_bme280_compensate(
                handle, copy, (nuint)copy.Length, out PamojaBme280Measurement produced));
            return produced;
        });
        return new Bme280Measurement(
            reading.Celsius, reading.Pascals, reading.Hectopascals, reading.RelativeHumidityPercent);
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>A Maxim DS18B20 1-Wire thermometer.</summary>
public static class Ds18b20
{
    /// <summary>The 1-Wire family code that identifies a DS18B20 on the bus.</summary>
    public const byte FamilyCode = 0x28;

    /// <summary>Parses and CRC-checks a nine-byte scratchpad.</summary>
    /// <param name="bytes">The scratchpad as the device sent it, the ninth its CRC.</param>
    /// <returns>The decoded reading.</returns>
    /// <exception cref="PamojaException">
    /// The CRC does not match, which means the read was corrupted on the bus and
    /// should be repeated.
    /// </exception>
    public static Ds18b20Reading ParseScratchpad(ReadOnlySpan<byte> bytes)
    {
        Status.ThrowIfError(NativeMethods.pamoja_ds18b20_parse_scratchpad(
            bytes, (nuint)bytes.Length, out PamojaDs18b20Reading reading));
        return new Ds18b20Reading(
            reading.RawTemperature,
            reading.MicroCelsius,
            reading.AlarmHigh,
            reading.AlarmLow,
            reading.ResolutionBits);
    }

    /// <summary>Builds the nine bytes a part in the given state puts on the bus.</summary>
    /// <remarks>
    /// This is the inverse of <see cref="ParseScratchpad"/>, so a node can be written and
    /// tested against what a thermometer sends without one attached.
    /// </remarks>
    /// <param name="celsius">The temperature the part is reading.</param>
    /// <param name="resolutionBits">The resolution it is configured for, 9 to 12.</param>
    /// <param name="alarmHigh">The high alarm threshold in whole degrees Celsius.</param>
    /// <param name="alarmLow">The low alarm threshold in whole degrees Celsius.</param>
    /// <returns>The nine scratchpad bytes in transmission order, CRC last.</returns>
    /// <exception cref="PamojaException">The resolution is not one the part offers.</exception>
    public static byte[] BuildScratchpad(
        float celsius,
        byte resolutionBits,
        sbyte alarmHigh,
        sbyte alarmLow)
    {
        byte[] bytes = new byte[9];
        Status.ThrowIfError(NativeMethods.pamoja_ds18b20_build_scratchpad(
            celsius, resolutionBits, alarmHigh, alarmLow, bytes));
        return bytes;
    }

    /// <summary>Computes the Maxim CRC-8 a 1-Wire device checks its bytes with.</summary>
    /// <param name="data">The bytes the checksum covers.</param>
    /// <returns>The checksum.</returns>
    public static byte Crc8(ReadOnlySpan<byte> data) =>
        NativeMethods.pamoja_ds18b20_crc8(data, (nuint)data.Length);

    /// <summary>Converts a raw temperature register to micro-degrees Celsius.</summary>
    /// <param name="raw">The 16-bit two's-complement register.</param>
    /// <returns>The temperature, exact in integer arithmetic.</returns>
    public static int MicroCelsius(short raw) => NativeMethods.pamoja_ds18b20_micro_celsius(raw);

    /// <summary>Converts a raw temperature register to degrees Celsius.</summary>
    /// <param name="raw">The 16-bit two's-complement register.</param>
    /// <returns>The temperature.</returns>
    public static float Celsius(short raw) => NativeMethods.pamoja_ds18b20_celsius(raw);

    /// <summary>Returns the configuration byte that selects a resolution.</summary>
    /// <param name="bits">The resolution in bits: 9, 10, 11, or 12.</param>
    /// <returns>The byte to write to the configuration register.</returns>
    /// <exception cref="PamojaException">The resolution is not one the part offers.</exception>
    public static byte ConfigByte(byte bits)
    {
        Status.ThrowIfError(NativeMethods.pamoja_ds18b20_config_byte(bits, out byte value));
        return value;
    }

    /// <summary>Returns the resolution a configuration byte selects.</summary>
    /// <param name="configByte">The byte read from the configuration register.</param>
    /// <returns>The resolution in bits.</returns>
    public static byte ResolutionBits(byte configByte) =>
        NativeMethods.pamoja_ds18b20_resolution_bits(configByte);

    /// <summary>Returns the temperature step a resolution resolves.</summary>
    /// <param name="bits">The resolution in bits.</param>
    /// <returns>The step in micro-degrees Celsius.</returns>
    /// <exception cref="PamojaException">The resolution is not one the part offers.</exception>
    public static uint StepMicroCelsius(byte bits)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_ds18b20_step_micro_celsius(bits, out uint value));
        return value;
    }

    /// <summary>Returns how long a conversion may take at a resolution.</summary>
    /// <param name="bits">The resolution in bits.</param>
    /// <returns>The datasheet's worst case, in microseconds.</returns>
    /// <exception cref="PamojaException">The resolution is not one the part offers.</exception>
    public static uint MaxConversionMicros(byte bits)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_ds18b20_max_conversion_micros(bits, out uint value));
        return value;
    }

    /// <summary>Decodes the text the Linux kernel's <c>w1_therm</c> driver serves for a thermometer.</summary>
    /// <remarks>
    /// The <c>w1_slave</c> file holds the scratchpad in hex with the kernel's checksum verdict,
    /// then the temperature. The scratchpad is parsed and its CRC checked here as well.
    /// </remarks>
    /// <param name="text">The file's contents.</param>
    /// <returns>The decoded reading.</returns>
    /// <exception cref="PamojaException">
    /// The kernel or this decoder rejects the CRC, or the text is not in the driver's format.
    /// </exception>
    public static Ds18b20Reading ParseW1Slave(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        Status.ThrowIfError(NativeMethods.pamoja_ds18b20_parse_w1_slave(text, out PamojaDs18b20Reading reading));
        return Read(reading);
    }

    /// <summary>
    /// Renders the text the Linux kernel's <c>w1_therm</c> driver serves for a scratchpad it
    /// read cleanly, the inverse of <see cref="ParseW1Slave"/>.
    /// </summary>
    /// <param name="scratchpad">The nine scratchpad bytes, the ninth their CRC.</param>
    /// <returns>The <c>w1_slave</c> file's two lines.</returns>
    /// <exception cref="PamojaException">The bytes are not nine, or the CRC does not match.</exception>
    public static string W1SlaveText(ReadOnlySpan<byte> scratchpad)
    {
        Status.ThrowIfError(NativeMethods.pamoja_ds18b20_w1_slave_text(
            scratchpad, (nuint)scratchpad.Length, out IntPtr text));
        return OwnedString.Read(text);
    }

    /// <summary>Converts the flat struct the C ABI returns.</summary>
    /// <param name="reading">The interop representation.</param>
    /// <returns>The reading.</returns>
    internal static Ds18b20Reading Read(PamojaDs18b20Reading reading) =>
        new(reading.RawTemperature, reading.MicroCelsius, reading.AlarmHigh, reading.AlarmLow, reading.ResolutionBits);
}
