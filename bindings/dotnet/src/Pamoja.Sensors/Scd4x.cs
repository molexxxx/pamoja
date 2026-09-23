using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Sensirion SCD40 or SCD41 carbon dioxide sensor, and a driver for one on an I2C bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet, decoded exactly as the manufacturer
/// specifies, and <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives the part over an <see cref="I2cBus"/> in periodic measurement: the part
/// produces a result every five seconds, and <see cref="Measure"/> waits for the next one. The
/// part has one address, <see cref="Address"/>. Nothing is sent until <see cref="Init"/> or the
/// first <see cref="Measure"/>.
/// </para>
/// </remarks>
public sealed class Scd4x : IDisposable
{
    /// <summary>The one address the part answers to.</summary>
    public const byte Address = 0x62;

    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Scd4x(I2cBus bus)
    {
        ArgumentNullException.ThrowIfNull(bus);
        IntPtr sensor = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held => NativeMethods.pamoja_scd4x_new(held, out sensor)));
        _handle = new NativeHandle(sensor, NativeMethods.pamoja_scd4x_free);
    }

    /// <summary>The 48-bit serial number read at initialization, or null before it.</summary>
    public ulong? Serial =>
        _handle.Use(sensor => NativeMethods.pamoja_scd4x_serial(sensor, out ulong serial)
            ? serial
            : (ulong?)null);

    /// <summary>Stops any running measurement, reads the serial number, and starts periodic measurement.</summary>
    /// <exception cref="PamojaException">Nothing answered, or the serial number failed its checksum.</exception>
    public void Init() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_scd4x_init));

    /// <summary>Waits for the next periodic result and reads it, initializing the part first if needed.</summary>
    /// <returns>The carbon dioxide, temperature, and humidity.</returns>
    /// <exception cref="PamojaException">
    /// As <see cref="Init"/>, when no result becomes ready, and when a word fails its checksum.
    /// </exception>
    public PamojaScd4xMeasurement Measure()
    {
        PamojaScd4xMeasurement measurement = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_scd4x_measure(sensor, out measurement)));
        return measurement;
    }

    /// <summary>Runs one on-demand measurement on an SCD41, which takes five seconds.</summary>
    /// <remarks>The part must not be measuring periodically: call <see cref="Stop"/> first, or use this instead of <see cref="Init"/>.</remarks>
    /// <returns>The measurement.</returns>
    /// <exception cref="PamojaException">As <see cref="Measure"/>.</exception>
    public PamojaScd4xMeasurement MeasureSingleShot()
    {
        PamojaScd4xMeasurement measurement = default;
        Status.ThrowIfError(_handle.Use(sensor =>
            NativeMethods.pamoja_scd4x_measure_single_shot(sensor, out measurement)));
        return measurement;
    }

    /// <summary>Asks the part whether a periodic result is waiting.</summary>
    /// <returns>Whether <see cref="Measure"/> would read without waiting.</returns>
    /// <exception cref="PamojaException">The transfer failed, or the status word failed its checksum.</exception>
    public bool DataReady()
    {
        bool ready = false;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_scd4x_poll_ready(sensor, out ready)));
        return ready;
    }

    /// <summary>Stops periodic measurement, after which the part takes its settings commands.</summary>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public void Stop() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_scd4x_stop));

    /// <summary>Starts periodic measurement.</summary>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public void Start() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_scd4x_start));

    /// <summary>Sets the temperature offset that compensates the part's own warmth, until power is lost.</summary>
    /// <param name="milliCelsius">The offset to subtract, in millidegrees.</param>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public void SetTemperatureOffset(uint milliCelsius) =>
        Status.ThrowIfError(_handle.Use(sensor =>
            NativeMethods.pamoja_scd4x_set_temperature_offset(sensor, milliCelsius)));

    /// <summary>Sets the altitude the part corrects its carbon dioxide reading for.</summary>
    /// <param name="meters">The altitude above sea level.</param>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public void SetSensorAltitude(ushort meters) =>
        Status.ThrowIfError(_handle.Use(sensor =>
            NativeMethods.pamoja_scd4x_set_sensor_altitude(sensor, meters)));

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

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

    /// <summary>An SCD4x that is not there, for a bus with nothing plugged in.</summary>
    /// <remarks>
    /// It answers the serial number, data-ready, and measurement commands, each word with its
    /// checksum, and always has a result waiting; starting, stopping, and the settings commands
    /// answer with nothing, as the real part's do.
    /// </remarks>
    public static class Sim
    {
        /// <summary>The carbon dioxide <see cref="Part"/> reports, in parts per million.</summary>
        public const ushort Co2Ppm = 800;

        /// <summary>The temperature <see cref="Part"/> reports.</summary>
        public const float Celsius = 22.5f;

        /// <summary>The relative humidity <see cref="Part"/> reports, as a percentage.</summary>
        public const float RelativeHumidity = 45.0f;

        /// <summary>The serial number every simulated part reports.</summary>
        public const ulong Serial = 0x0000_5A4D_0C1E_2B3F;

        /// <summary>Makes a part reading <see cref="Co2Ppm"/>, <see cref="Celsius"/>, and <see cref="RelativeHumidity"/>.</summary>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static CommandPart Part() =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_scd4x_sim_part(),
                NativeMethods.pamoja_i2c_part_free,
                "simulated SCD4x"));

        /// <summary>Makes a part that reads what it is asked to.</summary>
        /// <param name="co2Ppm">The carbon dioxide it reports, in parts per million.</param>
        /// <param name="celsius">The temperature it reports.</param>
        /// <param name="relativeHumidity">The humidity it reports, as a percentage.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static CommandPart Reporting(ushort co2Ppm, float celsius, float relativeHumidity) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_scd4x_sim_reporting(co2Ppm, celsius, relativeHumidity),
                NativeMethods.pamoja_i2c_part_free,
                "simulated SCD4x"));
    }
}
