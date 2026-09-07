using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Texas Instruments HDC1080 humidity and temperature sensor. Every call goes
/// straight to the pamoja C ABI, which decodes exactly what the manufacturer's
/// datasheet specifies.
/// </summary>
public static class Hdc1080
{
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
}
