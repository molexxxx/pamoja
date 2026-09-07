using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Sensirion SCD40 or SCD41 carbon dioxide sensor. Every call goes straight to the
/// pamoja C ABI, which decodes exactly what the manufacturer's datasheet specifies.
/// </summary>
public static class Scd4x
{
    /// <summary>Computes the CRC-8 an SCD4x appends to every data word.</summary>
    /// <param name="data">The data.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static byte Crc(ReadOnlySpan<byte> data) =>
        NativeMethods.pamoja_scd4x_crc(data, (nuint)data.Length);

    /// <summary>Reads a CRC-checked three-byte SCD4x word frame.</summary>
    /// <param name="frame">The frame.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static ushort Word(ReadOnlySpan<byte> frame)
    {
        ushort value;
        Status.ThrowIfError(NativeMethods.pamoja_scd4x_word(frame, (nuint)frame.Length, out value));
        return value;
    }

    /// <summary>Builds the three bytes an SCD4x sends for a word: the word then its CRC.</summary>
    /// <param name="value">The value.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] WordFrame(ushort value)
    {
        byte[] bytes = new byte[NativeMethods.Scd4xWordLen];
        Status.ThrowIfError(NativeMethods.pamoja_scd4x_word_frame(value, bytes));
        return bytes;
    }

    /// <summary>Builds the two bytes that address an SCD4x command, most significant first.</summary>
    /// <param name="command">The command.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] CommandFrame(ushort command)
    {
        byte[] bytes = new byte[NativeMethods.Scd4xCommandLen];
        Status.ThrowIfError(NativeMethods.pamoja_scd4x_command_frame(command, bytes));
        return bytes;
    }

    /// <summary>Builds the five bytes that write a word to an SCD4x: command, word, CRC.</summary>
    /// <param name="command">The command.</param>
    /// <param name="value">The value.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] WriteFrame(ushort command, ushort value)
    {
        byte[] bytes = new byte[NativeMethods.Scd4xWriteLen];
        Status.ThrowIfError(NativeMethods.pamoja_scd4x_write_frame(command, value, bytes));
        return bytes;
    }

    /// <summary>Returns how long an SCD4x command may take before its result can be read.</summary>
    /// <param name="command">The command.</param>
    /// <returns>The value, or null where the part defines none.</returns>
    public static ushort? MaxDurationMs(ushort command)
    {
        ushort value;
        return NativeMethods.pamoja_scd4x_max_duration_ms(command, out value) ? value : null;
    }

    /// <summary>Reports whether an SCD4x accepts a command while it is measuring.</summary>
    /// <param name="command">The command.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool AllowedDuringMeasurement(ushort command) =>
        NativeMethods.pamoja_scd4x_allowed_during_measurement(command);

    /// <summary>Parses and CRC-checks a nine-byte SCD4x measurement frame.</summary>
    /// <param name="frame">The frame.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaScd4xMeasurement ParseMeasurement(ReadOnlySpan<byte> frame)
    {
        PamojaScd4xMeasurement value;
        Status.ThrowIfError(NativeMethods.pamoja_scd4x_parse_measurement(frame, (nuint)frame.Length, out value));
        return value;
    }

    /// <summary>Builds the SCD4x measurement a sensor reporting these physical values would send.</summary>
    /// <param name="co2Ppm">The co2 ppm.</param>
    /// <param name="milliCelsius">The milli celsius.</param>
    /// <param name="humidityMilliPercent">The humidity milli percent.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaScd4xMeasurement MeasurementFromPhysical(ushort co2Ppm, int milliCelsius, uint humidityMilliPercent)
    {
        PamojaScd4xMeasurement value;
        Status.ThrowIfError(NativeMethods.pamoja_scd4x_measurement_from_physical(co2Ppm, milliCelsius, humidityMilliPercent, out value));
        return value;
    }

    /// <summary>Builds the nine bytes an SCD4x sends for a set of raw words, each CRC included.</summary>
    /// <param name="co2Ppm">The co2 ppm.</param>
    /// <param name="temperatureRaw">The temperature raw.</param>
    /// <param name="humidityRaw">The humidity raw.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] MeasurementBytes(ushort co2Ppm, ushort temperatureRaw, ushort humidityRaw)
    {
        byte[] bytes = new byte[NativeMethods.Scd4xMeasurementLen];
        Status.ThrowIfError(NativeMethods.pamoja_scd4x_measurement_bytes(co2Ppm, temperatureRaw, humidityRaw, bytes));
        return bytes;
    }

    /// <summary>Converts a raw SCD4x temperature word to milli-degrees Celsius.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static int MilliCelsius(ushort raw) =>
        NativeMethods.pamoja_scd4x_milli_celsius(raw);

    /// <summary>Converts a raw SCD4x temperature word to degrees Celsius.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float Celsius(ushort raw) =>
        NativeMethods.pamoja_scd4x_celsius(raw);

    /// <summary>Builds the SCD4x temperature word that decodes to a temperature.</summary>
    /// <param name="milliCelsius">The milli celsius.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort TemperatureRaw(int milliCelsius) =>
        NativeMethods.pamoja_scd4x_temperature_raw(milliCelsius);

    /// <summary>Converts a raw SCD4x humidity word to milli-percent.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint HumidityMilliPercent(ushort raw) =>
        NativeMethods.pamoja_scd4x_humidity_milli_percent(raw);

    /// <summary>Converts a raw SCD4x humidity word to a relative humidity percentage.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float RelativeHumidityPercent(ushort raw) =>
        NativeMethods.pamoja_scd4x_relative_humidity_percent(raw);

    /// <summary>Builds the SCD4x humidity word that decodes to a relative humidity.</summary>
    /// <param name="milliPercent">The milli percent.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort HumidityRaw(uint milliPercent) =>
        NativeMethods.pamoja_scd4x_humidity_raw(milliPercent);

    /// <summary>Reports whether an SCD4x data-ready word says a fresh result is waiting.</summary>
    /// <param name="word">The word.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool DataReady(ushort word) =>
        NativeMethods.pamoja_scd4x_data_ready(word);

    /// <summary>Builds the SCD4x temperature-offset word for an offset.</summary>
    /// <param name="milliCelsius">The milli celsius.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort TemperatureOffsetWord(uint milliCelsius) =>
        NativeMethods.pamoja_scd4x_temperature_offset_word(milliCelsius);

    /// <summary>Reads an SCD4x temperature-offset word back as an offset.</summary>
    /// <param name="word">The word.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint TemperatureOffsetMilliCelsius(ushort word) =>
        NativeMethods.pamoja_scd4x_temperature_offset_milli_celsius(word);

    /// <summary>Builds the SCD4x ambient-pressure word for a pressure.</summary>
    /// <param name="pascals">The pascals.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort AmbientPressureWord(uint pascals) =>
        NativeMethods.pamoja_scd4x_ambient_pressure_word(pascals);

    /// <summary>Reads an SCD4x ambient-pressure word back as a pressure.</summary>
    /// <param name="word">The word.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint AmbientPressurePascals(ushort word) =>
        NativeMethods.pamoja_scd4x_ambient_pressure_pascals(word);

    /// <summary>Reads the correction a forced recalibration applied.</summary>
    /// <param name="word">The word.</param>
    /// <returns>The value, or null where the part defines none.</returns>
    public static int? ForcedRecalibrationCorrectionPpm(ushort word)
    {
        int value;
        return NativeMethods.pamoja_scd4x_forced_recalibration_correction_ppm(word, out value) ? value : null;
    }

    /// <summary>Builds the word an SCD4x returns for a forced-recalibration outcome.</summary>
    /// <param name="succeeded">The succeeded.</param>
    /// <param name="correctionPpm">The correction ppm.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort ForcedRecalibrationWord(bool succeeded, int correctionPpm) =>
        NativeMethods.pamoja_scd4x_forced_recalibration_word(succeeded, correctionPpm);

    /// <summary>Reports whether an SCD4x word says automatic self-calibration is on.</summary>
    /// <param name="word">The word.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool AutomaticSelfCalibrationEnabled(ushort word) =>
        NativeMethods.pamoja_scd4x_automatic_self_calibration_enabled(word);

    /// <summary>Builds the SCD4x word that turns automatic self-calibration on or off.</summary>
    /// <param name="enabled">The enabled.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort AutomaticSelfCalibrationWord(bool enabled) =>
        NativeMethods.pamoja_scd4x_automatic_self_calibration_word(enabled);

    /// <summary>Reports whether an SCD4x self-test word says the part is healthy.</summary>
    /// <param name="word">The word.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool SelfTestPassed(ushort word) =>
        NativeMethods.pamoja_scd4x_self_test_passed(word);

    /// <summary>Reads the 48-bit serial number out of a nine-byte SCD4x frame.</summary>
    /// <param name="frame">The frame.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static ulong SerialNumber(ReadOnlySpan<byte> frame)
    {
        ulong value;
        Status.ThrowIfError(NativeMethods.pamoja_scd4x_serial_number(frame, (nuint)frame.Length, out value));
        return value;
    }

    /// <summary>Builds the nine bytes an SCD4x sends for a serial number, each CRC included.</summary>
    /// <param name="serial">The serial.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] SerialNumberFrame(ulong serial)
    {
        byte[] bytes = new byte[NativeMethods.Scd4xMeasurementLen];
        Status.ThrowIfError(NativeMethods.pamoja_scd4x_serial_number_frame(serial, bytes));
        return bytes;
    }
}
