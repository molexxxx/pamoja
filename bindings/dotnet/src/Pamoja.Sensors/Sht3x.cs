using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Sensirion SHT3x-DIS humidity and temperature sensor. Every call goes straight to
/// the pamoja C ABI, which decodes exactly what the manufacturer's datasheet specifies.
/// </summary>
public static class Sht3x
{
    /// <summary>Computes the CRC-8 an SHT3x appends to every data word.</summary>
    /// <param name="data">The data.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static byte Crc(ReadOnlySpan<byte> data) =>
        NativeMethods.pamoja_sht3x_crc(data, (nuint)data.Length);

    /// <summary>Reads a CRC-checked three-byte SHT3x word frame.</summary>
    /// <param name="frame">The frame.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static ushort Word(ReadOnlySpan<byte> frame)
    {
        ushort value;
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_word(frame, (nuint)frame.Length, out value));
        return value;
    }

    /// <summary>Builds the three bytes an SHT3x sends for a word: the word then its CRC.</summary>
    /// <param name="value">The value.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] WordBytes(ushort value)
    {
        byte[] bytes = new byte[NativeMethods.Sht3xWordLen];
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_word_bytes(value, bytes));
        return bytes;
    }

    /// <summary>Parses and CRC-checks a six-byte SHT3x measurement frame.</summary>
    /// <param name="frame">The frame.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaSht3xMeasurement ParseMeasurement(ReadOnlySpan<byte> frame)
    {
        PamojaSht3xMeasurement value;
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_parse_measurement(frame, (nuint)frame.Length, out value));
        return value;
    }

    /// <summary>Builds the six bytes an SHT3x sends for a pair of raw words.</summary>
    /// <param name="temperatureRaw">The temperature raw.</param>
    /// <param name="humidityRaw">The humidity raw.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] MeasurementBytes(ushort temperatureRaw, ushort humidityRaw)
    {
        byte[] bytes = new byte[NativeMethods.Sht3xMeasurementLen];
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_measurement_bytes(temperatureRaw, humidityRaw, bytes));
        return bytes;
    }

    /// <summary>Converts a raw SHT3x temperature word to milli-degrees Celsius.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static int MilliCelsius(ushort raw) =>
        NativeMethods.pamoja_sht3x_milli_celsius(raw);

    /// <summary>Converts a raw SHT3x temperature word to degrees Celsius.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float Celsius(ushort raw) =>
        NativeMethods.pamoja_sht3x_celsius(raw);

    /// <summary>Converts a raw SHT3x temperature word to milli-degrees Fahrenheit.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static int MilliFahrenheit(ushort raw) =>
        NativeMethods.pamoja_sht3x_milli_fahrenheit(raw);

    /// <summary>Converts a raw SHT3x temperature word to degrees Fahrenheit.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float Fahrenheit(ushort raw) =>
        NativeMethods.pamoja_sht3x_fahrenheit(raw);

    /// <summary>Converts a raw SHT3x humidity word to milli-percent.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint MilliPercent(ushort raw) =>
        NativeMethods.pamoja_sht3x_milli_percent(raw);

    /// <summary>Converts a raw SHT3x humidity word to a relative humidity percentage.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float RelativeHumidity(ushort raw) =>
        NativeMethods.pamoja_sht3x_relative_humidity(raw);

    /// <summary>Builds the SHT3x temperature word that decodes to a temperature.</summary>
    /// <param name="milliCelsius">The milli celsius.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort TemperatureRawFromMilliCelsius(int milliCelsius) =>
        NativeMethods.pamoja_sht3x_temperature_raw_from_milli_celsius(milliCelsius);

    /// <summary>Builds the SHT3x temperature word that decodes to a temperature in Celsius.</summary>
    /// <param name="celsius">The celsius.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort TemperatureRawFromCelsius(float celsius) =>
        NativeMethods.pamoja_sht3x_temperature_raw_from_celsius(celsius);

    /// <summary>Builds the SHT3x temperature word that decodes to a temperature in Fahrenheit.</summary>
    /// <param name="milliFahrenheit">The milli fahrenheit.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort TemperatureRawFromMilliFahrenheit(int milliFahrenheit) =>
        NativeMethods.pamoja_sht3x_temperature_raw_from_milli_fahrenheit(milliFahrenheit);

    /// <summary>Builds the SHT3x humidity word that decodes to a relative humidity.</summary>
    /// <param name="milliPercent">The milli percent.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort HumidityRawFromMilliPercent(uint milliPercent) =>
        NativeMethods.pamoja_sht3x_humidity_raw_from_milli_percent(milliPercent);

    /// <summary>Builds the SHT3x humidity word that decodes to a relative humidity percentage.</summary>
    /// <param name="percent">The percent.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort HumidityRawFromRelativeHumidity(float percent) =>
        NativeMethods.pamoja_sht3x_humidity_raw_from_relative_humidity(percent);

    /// <summary>Parses and CRC-checks a three-byte SHT3x status frame.</summary>
    /// <param name="frame">The frame.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaSht3xStatus ParseStatus(ReadOnlySpan<byte> frame)
    {
        PamojaSht3xStatus value;
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_parse_status(frame, (nuint)frame.Length, out value));
        return value;
    }

    /// <summary>Splits an SHT3x status word into its flags.</summary>
    /// <param name="bits">The bits.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaSht3xStatus StatusFromBits(ushort bits)
    {
        PamojaSht3xStatus value;
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_status_from_bits(bits, out value));
        return value;
    }

    /// <summary>Builds the three bytes an SHT3x sends for a status word, CRC last.</summary>
    /// <param name="bits">The bits.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] StatusBytes(ushort bits)
    {
        byte[] bytes = new byte[NativeMethods.Sht3xWordLen];
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_status_bytes(bits, bytes));
        return bytes;
    }

    /// <summary>Returns the SHT3x single-shot command for a repeatability and clock mode.</summary>
    /// <param name="repeatability">The repeatability.</param>
    /// <param name="clockStretching">The clock stretching.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static ushort SingleShot(byte repeatability, bool clockStretching)
    {
        ushort value;
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_single_shot(repeatability, clockStretching, out value));
        return value;
    }

    /// <summary>Returns the SHT3x periodic-mode command for a repeatability and rate.</summary>
    /// <param name="repeatability">The repeatability.</param>
    /// <param name="rate">The rate.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static ushort Periodic(byte repeatability, byte rate)
    {
        ushort value;
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_periodic(repeatability, rate, out value));
        return value;
    }

    /// <summary>Returns how long an SHT3x measurement may take at a repeatability.</summary>
    /// <param name="repeatability">The repeatability.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static uint MaxMeasurementMicros(byte repeatability)
    {
        uint value;
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_max_measurement_micros(repeatability, out value));
        return value;
    }

    /// <summary>Returns how long an SHT3x measurement typically takes at a repeatability.</summary>
    /// <param name="repeatability">The repeatability.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static uint TypicalMeasurementMicros(byte repeatability)
    {
        uint value;
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_typical_measurement_micros(repeatability, out value));
        return value;
    }

    /// <summary>Returns the gap between SHT3x periodic measurements at a rate.</summary>
    /// <param name="rate">The rate.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static uint IntervalMicros(byte rate)
    {
        uint value;
        Status.ThrowIfError(NativeMethods.pamoja_sht3x_interval_micros(rate, out value));
        return value;
    }
}
