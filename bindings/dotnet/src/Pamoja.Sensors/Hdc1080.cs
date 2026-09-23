using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Texas Instruments HDC1080 humidity and temperature sensor, and a driver for one on an
/// I2C bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet, decoded exactly as the manufacturer
/// specifies, and <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives the part over an <see cref="I2cBus"/>, measuring temperature then
/// humidity from one trigger, as the datasheet's sequential mode does. The part has one
/// address, <see cref="Address"/>. Nothing is sent until <see cref="Init"/> or the first
/// <see cref="Measure"/>.
/// </para>
/// </remarks>
public sealed class Hdc1080 : IDisposable
{
    /// <summary>The one address the part answers to.</summary>
    public const byte Address = 0x40;

    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <param name="temperature">The temperature resolution, which sets its conversion time.</param>
    /// <param name="humidity">The humidity resolution, which sets its conversion time.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Hdc1080(
        I2cBus bus,
        TemperatureResolution temperature = TemperatureResolution.Bits14,
        HumidityResolution humidity = HumidityResolution.Bits14)
    {
        ArgumentNullException.ThrowIfNull(bus);
        var settings = new PamojaHdc1080Settings
        {
            TemperatureResolutionBits = (byte)temperature,
            HumidityResolutionBits = (byte)humidity,
        };
        IntPtr sensor = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held => NativeMethods.pamoja_hdc1080_new(held, settings, out sensor)));
        _handle = new NativeHandle(sensor, NativeMethods.pamoja_hdc1080_free);
    }

    /// <summary>The temperature channel's resolution, as its bit count.</summary>
    public enum TemperatureResolution : byte
    {
        /// <summary>11 bits, 3.65 ms.</summary>
        Bits11 = 11,

        /// <summary>14 bits, 6.35 ms.</summary>
        Bits14 = 14,
    }

    /// <summary>The humidity channel's resolution, as its bit count.</summary>
    public enum HumidityResolution : byte
    {
        /// <summary>8 bits, 2.5 ms.</summary>
        Bits8 = 8,

        /// <summary>11 bits, 3.85 ms.</summary>
        Bits11 = 11,

        /// <summary>14 bits, 6.5 ms.</summary>
        Bits14 = 14,
    }

    /// <summary>The configuration the driver writes.</summary>
    public PamojaHdc1080Config Configuration
    {
        get
        {
            PamojaHdc1080Config config = default;
            Status.ThrowIfError(_handle.Use(sensor =>
                NativeMethods.pamoja_hdc1080_configuration(sensor, out config)));
            return config;
        }
    }

    /// <summary>Checks the part is an HDC1080 and writes the configuration.</summary>
    /// <exception cref="PamojaException">Nothing answered, or either id register is not an HDC1080's.</exception>
    public void Init() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_hdc1080_init));

    /// <summary>Triggers one acquisition of both channels and reads them, initializing the part first if needed.</summary>
    /// <returns>The temperature and humidity.</returns>
    /// <exception cref="PamojaException">
    /// As <see cref="Init"/>, and when the part does not acknowledge the read, which it refuses
    /// until its results are ready.
    /// </exception>
    public PamojaHdc1080Measurement Measure()
    {
        PamojaHdc1080Measurement measurement = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_hdc1080_measure(sensor, out measurement)));
        return measurement;
    }

    /// <summary>Switches the on-die heater, which runs only during acquisitions, on or off.</summary>
    /// <param name="on">Whether the heater runs.</param>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public void Heater(bool on) =>
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_hdc1080_heater(sensor, on)));

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

    /// <summary>Converts a raw HDC1080 temperature register to milli-degrees Celsius.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static int MilliCelsius(ushort raw) =>
        NativeMethods.pamoja_hdc1080_milli_celsius(raw);

    /// <summary>Converts a raw HDC1080 temperature register to degrees Celsius.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float Celsius(ushort raw) =>
        NativeMethods.pamoja_hdc1080_celsius(raw);

    /// <summary>Converts a raw HDC1080 humidity register to milli-percent.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint MilliPercent(ushort raw) =>
        NativeMethods.pamoja_hdc1080_milli_percent(raw);

    /// <summary>Converts a raw HDC1080 humidity register to a relative humidity percentage.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float RelativeHumidity(ushort raw) =>
        NativeMethods.pamoja_hdc1080_relative_humidity(raw);

    /// <summary>Builds the HDC1080 temperature register that decodes to a temperature.</summary>
    /// <param name="milliCelsius">The milli celsius.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort TemperatureRegister(int milliCelsius) =>
        NativeMethods.pamoja_hdc1080_temperature_register(milliCelsius);

    /// <summary>Builds the HDC1080 humidity register that decodes to a relative humidity.</summary>
    /// <param name="milliPercent">The milli percent.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort HumidityRegister(uint milliPercent) =>
        NativeMethods.pamoja_hdc1080_humidity_register(milliPercent);

    /// <summary>Joins the three HDC1080 serial-ID registers into the 40-bit serial number.</summary>
    /// <param name="high">The high.</param>
    /// <param name="mid">The mid.</param>
    /// <param name="low">The low.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ulong SerialId(ushort high, ushort mid, ushort low) =>
        NativeMethods.pamoja_hdc1080_serial_id(high, mid, low);

    /// <summary>Splits a serial number back into the three HDC1080 serial-ID registers.</summary>
    /// <param name="serial">The serial.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static ushort SerialIdRegisters(ulong serial)
    {
        ushort value;
        Status.ThrowIfError(NativeMethods.pamoja_hdc1080_serial_id_registers(serial, out value));
        return value;
    }

    /// <summary>Parses the four bytes an HDC1080 sequential read returns.</summary>
    /// <param name="bytes">The bytes.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaHdc1080Measurement ParseMeasurement(ReadOnlySpan<byte> bytes)
    {
        PamojaHdc1080Measurement value;
        Status.ThrowIfError(NativeMethods.pamoja_hdc1080_parse_measurement(bytes, (nuint)bytes.Length, out value));
        return value;
    }

    /// <summary>Builds the HDC1080 measurement a sensor reporting these physical values would send.</summary>
    /// <param name="milliCelsius">The milli celsius.</param>
    /// <param name="milliPercent">The milli percent.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaHdc1080Measurement MeasurementFromPhysical(int milliCelsius, uint milliPercent)
    {
        PamojaHdc1080Measurement value;
        Status.ThrowIfError(NativeMethods.pamoja_hdc1080_measurement_from_physical(milliCelsius, milliPercent, out value));
        return value;
    }

    /// <summary>Builds the four bytes an HDC1080 sends for a pair of raw registers.</summary>
    /// <param name="temperatureRaw">The temperature raw.</param>
    /// <param name="humidityRaw">The humidity raw.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] MeasurementBytes(ushort temperatureRaw, ushort humidityRaw)
    {
        byte[] bytes = new byte[NativeMethods.Hdc1080MeasurementLen];
        Status.ThrowIfError(NativeMethods.pamoja_hdc1080_measurement_bytes(temperatureRaw, humidityRaw, bytes));
        return bytes;
    }

    /// <summary>Parses an HDC1080 configuration register value.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaHdc1080Config ConfigFromRegister(ushort raw)
    {
        PamojaHdc1080Config value;
        Status.ThrowIfError(NativeMethods.pamoja_hdc1080_config_from_register(raw, out value));
        return value;
    }

    /// <summary>Assembles an HDC1080 configuration register value.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static ushort ConfigToRegister(PamojaHdc1080Config config)
    {
        ushort value;
        Status.ThrowIfError(NativeMethods.pamoja_hdc1080_config_to_register(config, out value));
        return value;
    }

    /// <summary>Returns how long to wait after triggering an HDC1080 in a configuration.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static uint ConversionTimeMicros(PamojaHdc1080Config config)
    {
        uint value;
        Status.ThrowIfError(NativeMethods.pamoja_hdc1080_conversion_time_micros(config, out value));
        return value;
    }

    /// <summary>Returns how long an HDC1080 temperature conversion takes at a resolution.</summary>
    /// <param name="bits">The bits.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static uint TemperatureConversionMicros(byte bits)
    {
        uint value;
        Status.ThrowIfError(NativeMethods.pamoja_hdc1080_temperature_conversion_micros(bits, out value));
        return value;
    }

    /// <summary>Returns how long an HDC1080 humidity conversion takes at a resolution.</summary>
    /// <param name="bits">The bits.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static uint HumidityConversionMicros(byte bits)
    {
        uint value;
        Status.ThrowIfError(NativeMethods.pamoja_hdc1080_humidity_conversion_micros(bits, out value));
        return value;
    }

    /// <summary>An HDC1080 that is not there, for a bus with nothing plugged in.</summary>
    public static class Sim
    {
        /// <summary>The temperature <see cref="Part"/> reports.</summary>
        public const float Celsius = 22.5f;

        /// <summary>The relative humidity <see cref="Part"/> reports, as a percentage.</summary>
        public const float RelativeHumidity = 45.0f;

        /// <summary>Makes a part reading <see cref="Celsius"/> and <see cref="RelativeHumidity"/>.</summary>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Part() =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_hdc1080_sim_part(),
                NativeMethods.pamoja_i2c_part_free,
                "simulated HDC1080"));

        /// <summary>Makes a part that reads what it is asked to.</summary>
        /// <param name="celsius">The temperature it reports.</param>
        /// <param name="relativeHumidity">The humidity it reports, as a percentage.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Reporting(float celsius, float relativeHumidity) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_hdc1080_sim_reporting(celsius, relativeHumidity),
                NativeMethods.pamoja_i2c_part_free,
                "simulated HDC1080"));
    }
}
