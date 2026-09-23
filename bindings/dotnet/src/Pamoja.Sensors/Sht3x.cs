using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Sensirion SHT3x-DIS humidity and temperature sensor, and a driver for one on an I2C bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet, decoded exactly as the manufacturer
/// specifies, and <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives one part over an <see cref="I2cBus"/>, measuring on demand in
/// single-shot mode. The part has no id register, so <see cref="Init"/> soft-resets it and
/// reads its status: a status word whose checksum holds is what confirms an SHT3x answers.
/// Nothing is sent until <see cref="Init"/> or the first <see cref="Measure"/>.
/// </para>
/// </remarks>
public sealed class Sht3x : IDisposable
{
    /// <summary>The address with ADDR low.</summary>
    public const byte AddressA = 0x44;

    /// <summary>The address with ADDR high.</summary>
    public const byte AddressB = 0x45;

    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part at <paramref name="address"/> on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <param name="address"><see cref="AddressA"/> with ADDR low, <see cref="AddressB"/> with ADDR high.</param>
    /// <param name="repeatability">How repeatable each measurement is, against how long it takes.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Sht3x(I2cBus bus, byte address, Repeatability repeatability = Repeatability.High)
    {
        ArgumentNullException.ThrowIfNull(bus);
        IntPtr sensor = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held =>
            NativeMethods.pamoja_sht3x_new(held, address, (byte)repeatability, out sensor)));
        _handle = new NativeHandle(sensor, NativeMethods.pamoja_sht3x_free);
    }

    /// <summary>How repeatable a measurement is, which trades noise against time and energy.</summary>
    public enum Repeatability : byte
    {
        /// <summary>Low repeatability, 4 ms at most.</summary>
        Low = 0,

        /// <summary>Medium repeatability, 6 ms at most.</summary>
        Medium = 1,

        /// <summary>High repeatability, 15 ms at most; the driver's default.</summary>
        High = 2,
    }

    /// <summary>The status register as it was last read, or null before it has been.</summary>
    public PamojaSht3xStatus? LastStatus =>
        _handle.Use(sensor => NativeMethods.pamoja_sht3x_last_status(sensor, out PamojaSht3xStatus status)
            ? status
            : (PamojaSht3xStatus?)null);

    /// <summary>Soft-resets the part and reads its status.</summary>
    /// <exception cref="PamojaException">Nothing answered, or the status word failed its checksum.</exception>
    public void Init() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_sht3x_init));

    /// <summary>Runs one single-shot measurement, initializing the part first if needed.</summary>
    /// <returns>The checksum-checked temperature and humidity.</returns>
    /// <exception cref="PamojaException">As <see cref="Init"/>, and when a data word fails its checksum.</exception>
    public PamojaSht3xMeasurement Measure()
    {
        PamojaSht3xMeasurement measurement = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_sht3x_measure(sensor, out measurement)));
        return measurement;
    }

    /// <summary>Reads the status register.</summary>
    /// <returns>The status, which <see cref="LastStatus"/> keeps as well.</returns>
    /// <exception cref="PamojaException">The transfer failed, or the word failed its checksum.</exception>
    public PamojaSht3xStatus ReadStatus()
    {
        PamojaSht3xStatus status = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_sht3x_read_status(sensor, out status)));
        return status;
    }

    /// <summary>Switches the plausibility-check heater on, initializing the part first if needed.</summary>
    /// <exception cref="PamojaException">As <see cref="Init"/>.</exception>
    public void HeaterOn() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_sht3x_heater_on));

    /// <summary>Switches the heater off, which is its state after any reset.</summary>
    /// <exception cref="PamojaException">As <see cref="Init"/>.</exception>
    public void HeaterOff() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_sht3x_heater_off));

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

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

    /// <summary>An SHT3x that is not there, for a bus with nothing plugged in.</summary>
    /// <remarks>
    /// It takes Sensirion's 16-bit commands: every single-shot measurement and a periodic fetch
    /// answer with the reading and its checksums, the status command with the status a part
    /// reports after a reset, and a reset, the heater, and the rest with nothing.
    /// </remarks>
    public static class Sim
    {
        /// <summary>The temperature <see cref="Part"/> reports.</summary>
        public const float Celsius = 22.5f;

        /// <summary>The relative humidity <see cref="Part"/> reports, as a percentage.</summary>
        public const float RelativeHumidity = 45.0f;

        /// <summary>Makes a part reading <see cref="Celsius"/> and <see cref="RelativeHumidity"/>.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static CommandPart Part(byte address) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_sht3x_sim_part(address),
                NativeMethods.pamoja_i2c_part_free,
                "simulated SHT3x"));

        /// <summary>Makes a part that reads what it is asked to.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <param name="celsius">The temperature it reports.</param>
        /// <param name="relativeHumidity">The humidity it reports, as a percentage.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static CommandPart Reporting(byte address, float celsius, float relativeHumidity) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_sht3x_sim_reporting(address, celsius, relativeHumidity),
                NativeMethods.pamoja_i2c_part_free,
                "simulated SHT3x"));
    }
}
