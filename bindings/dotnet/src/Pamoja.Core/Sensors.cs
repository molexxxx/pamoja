using Pamoja.Core.Interop;

namespace Pamoja.Core;

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
        PamojaCore.ThrowIfError(NativeMethods.pamoja_bme280_calibration_new(
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
            PamojaCore.ThrowIfError(NativeMethods.pamoja_bme280_compensate(
                handle, copy, (nuint)copy.Length, out PamojaBme280Measurement produced));
            return produced;
        });
        return new Bme280Measurement(
            reading.Celsius, reading.Pascals, reading.Hectopascals, reading.RelativeHumidityPercent);
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>A Bosch BME280 temperature, pressure, and humidity sensor.</summary>
public static class Bme280
{
    /// <summary>The address a BME280 answers on with its SDO pin low.</summary>
    public const byte AddressPrimary = 0x76;

    /// <summary>The address it answers on with SDO high.</summary>
    public const byte AddressSecondary = 0x77;

    /// <summary>The value its chip-ID register reads, which confirms the part.</summary>
    public const byte ChipId = 0x60;
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
        PamojaCore.ThrowIfError(NativeMethods.pamoja_ds18b20_parse_scratchpad(
            bytes, (nuint)bytes.Length, out PamojaDs18b20Reading reading));
        return new Ds18b20Reading(
            reading.RawTemperature,
            reading.MicroCelsius,
            reading.AlarmHigh,
            reading.AlarmLow,
            reading.ResolutionBits);
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
        PamojaCore.ThrowIfError(NativeMethods.pamoja_ds18b20_config_byte(bits, out byte value));
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
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_ds18b20_step_micro_celsius(bits, out uint value));
        return value;
    }

    /// <summary>Returns how long a conversion may take at a resolution.</summary>
    /// <param name="bits">The resolution in bits.</param>
    /// <returns>The datasheet's worst case, in microseconds.</returns>
    /// <exception cref="PamojaException">The resolution is not one the part offers.</exception>
    public static uint MaxConversionMicros(byte bits)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_ds18b20_max_conversion_micros(bits, out uint value));
        return value;
    }
}

/// <summary>A TI INA219 current, voltage, and power monitor.</summary>
public static class Ina219
{
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
}

/// <summary>A TI ADS1115 16-bit analogue-to-digital converter.</summary>
public static class Ads1115
{
    /// <summary>The value the configuration register reads after a reset.</summary>
    public const ushort ConfigReset = 0x8583;

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
        PamojaCore.ThrowIfError(
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
}
