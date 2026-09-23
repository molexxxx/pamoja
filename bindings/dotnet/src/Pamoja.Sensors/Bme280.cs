using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>A BME280 <c>ctrl_meas</c> register, field by field.</summary>
/// <param name="Temperature">The temperature oversampling.</param>
/// <param name="Pressure">The pressure oversampling.</param>
/// <param name="Mode">The power mode.</param>
public readonly record struct Bme280CtrlMeas(
    Bme280.Oversampling Temperature,
    Bme280.Oversampling Pressure,
    Bme280.Mode Mode);

/// <summary>A BME280 <c>config</c> register, field by field.</summary>
/// <param name="Standby">The normal-mode standby period.</param>
/// <param name="Filter">The IIR filter.</param>
/// <param name="Spi3Wire">Whether the 3-wire SPI interface is enabled.</param>
public readonly record struct Bme280Config(
    Bme280.Standby Standby,
    Bme280.Filter Filter,
    bool Spi3Wire);

/// <summary>
/// A Bosch BME280 temperature, pressure, and humidity sensor, and a driver for one on an I2C
/// bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet: its addresses, registers, setting codes, and
/// timing, and <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives one part over an <see cref="I2cBus"/>, measuring on demand in forced
/// mode. Nothing is sent until <see cref="Init"/> or the first <see cref="Measure"/>. The
/// driver holds its own share of the bus, so the bus may be disposed once the driver is built,
/// and it waits as the datasheet asks, sleeping only when a real part is on the other end.
/// </para>
/// </remarks>
public sealed class Bme280 : IDisposable
{
    /// <summary>The address a BME280 answers on with its SDO pin low.</summary>
    public const byte AddressPrimary = 0x76;

    /// <summary>The address it answers on with SDO high.</summary>
    public const byte AddressSecondary = 0x77;

    /// <summary>The value its chip-ID register reads, which tells it from a BMP280.</summary>
    public const byte ChipId = 0x60;

    /// <summary>The word written to the reset register to restart the part.</summary>
    public const byte ResetWord = 0xB6;

    /// <summary>How long the part takes to start after a reset, in microseconds.</summary>
    public const uint StartupMicros = 2_000;

    /// <summary>How many bytes the temperature and pressure calibration block holds.</summary>
    public const int CalibrationTempPressLength = NativeMethods.Bme280CalibrationTempPressLen;

    /// <summary>How many bytes the humidity calibration block holds.</summary>
    public const int CalibrationHumidityLength = NativeMethods.Bme280CalibrationHumidityLen;

    /// <summary>How many bytes one measurement burst holds.</summary>
    public const int DataLength = NativeMethods.Bme280MeasurementLen;

    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part at <paramref name="address"/> on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <param name="address"><see cref="AddressPrimary"/> with SDO low, <see cref="AddressSecondary"/> with SDO high.</param>
    /// <param name="temperature">The temperature oversampling.</param>
    /// <param name="pressure">The pressure oversampling.</param>
    /// <param name="humidity">The humidity oversampling.</param>
    /// <param name="filter">The IIR filter.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Bme280(
        I2cBus bus,
        byte address,
        Oversampling temperature = Oversampling.X1,
        Oversampling pressure = Oversampling.X1,
        Oversampling humidity = Oversampling.X1,
        Filter filter = Filter.Off)
    {
        ArgumentNullException.ThrowIfNull(bus);
        var settings = new PamojaBme280Settings
        {
            Temperature = (byte)temperature,
            Pressure = (byte)pressure,
            Humidity = (byte)humidity,
            Filter = (byte)filter,
        };
        IntPtr sensor = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held =>
            NativeMethods.pamoja_bme280_new(held, address, settings, out sensor)));
        _handle = new NativeHandle(sensor, NativeMethods.pamoja_bme280_free);
    }

    /// <summary>How many samples a measurement averages, as its register code.</summary>
    public enum Oversampling : byte
    {
        /// <summary>The measurement is skipped.</summary>
        Skipped = 0,

        /// <summary>One sample.</summary>
        X1 = 1,

        /// <summary>Two samples.</summary>
        X2 = 2,

        /// <summary>Four samples.</summary>
        X4 = 3,

        /// <summary>Eight samples.</summary>
        X8 = 4,

        /// <summary>Sixteen samples.</summary>
        X16 = 5,
    }

    /// <summary>A power mode, as its register code.</summary>
    public enum Mode : byte
    {
        /// <summary>No measurements; the power-on default.</summary>
        Sleep = 0,

        /// <summary>One measurement, then back to sleep.</summary>
        Forced = 1,

        /// <summary>Measurements on a cycle, a standby period apart.</summary>
        Normal = 3,
    }

    /// <summary>The IIR filter that smooths pressure and temperature, as its register code.</summary>
    public enum Filter : byte
    {
        /// <summary>No filtering.</summary>
        Off = 0,

        /// <summary>Coefficient 2.</summary>
        X2 = 1,

        /// <summary>Coefficient 4.</summary>
        X4 = 2,

        /// <summary>Coefficient 8.</summary>
        X8 = 3,

        /// <summary>Coefficient 16.</summary>
        X16 = 4,
    }

    /// <summary>The period the part rests between measurements in normal mode, as its register code.</summary>
    public enum Standby : byte
    {
        /// <summary>0.5 ms.</summary>
        Ms0_5 = 0,

        /// <summary>62.5 ms.</summary>
        Ms62_5 = 1,

        /// <summary>125 ms.</summary>
        Ms125 = 2,

        /// <summary>250 ms.</summary>
        Ms250 = 3,

        /// <summary>500 ms.</summary>
        Ms500 = 4,

        /// <summary>1000 ms.</summary>
        Ms1000 = 5,

        /// <summary>10 ms.</summary>
        Ms10 = 6,

        /// <summary>20 ms.</summary>
        Ms20 = 7,
    }

    /// <summary>Resets the part, checks it is a BME280, reads its calibration, and writes the settings.</summary>
    /// <remarks>
    /// The order is the datasheet's: the reset word, the start-up time, a wait for the
    /// calibration image to load, the chip id, the two calibration blocks, then
    /// <c>config</c>, <c>ctrl_hum</c>, and <c>ctrl_meas</c>, leaving the part asleep.
    /// </remarks>
    /// <exception cref="PamojaException">
    /// Nothing answered at the address, another part did, or the calibration never finished
    /// loading.
    /// </exception>
    public void Init() =>
        Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_bme280_init));

    /// <summary>Runs one forced measurement and compensates it, initializing the part first if needed.</summary>
    /// <returns>The reading.</returns>
    /// <exception cref="PamojaException">
    /// As <see cref="Init"/>, and when the part is still measuring after the datasheet's time.
    /// </exception>
    public Bme280Measurement Measure()
    {
        PamojaBme280Measurement reading = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_bme280_measure(sensor, out reading)));
        return new Bme280Measurement(
            reading.Celsius,
            reading.Pascals,
            reading.Hectopascals,
            reading.RelativeHumidityPercent);
    }

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

    /// <summary>Reports whether a status register says a conversion is running.</summary>
    /// <param name="status">The status register.</param>
    /// <returns>Whether a measurement is in progress.</returns>
    public static bool Measuring(byte status) => NativeMethods.pamoja_bme280_measuring(status);

    /// <summary>Reports whether a status register says the calibration image is loading.</summary>
    /// <param name="status">The status register.</param>
    /// <returns>Whether the calibration is still being copied.</returns>
    public static bool ImageUpdating(byte status) => NativeMethods.pamoja_bme280_image_updating(status);

    /// <summary>Assembles a <c>ctrl_meas</c> register value.</summary>
    /// <param name="ctrl">The oversampling and the power mode.</param>
    /// <returns>The register value to write.</returns>
    public static byte CtrlMeasBits(Bme280CtrlMeas ctrl) =>
        NativeMethods.pamoja_bme280_ctrl_meas_bits(new PamojaBme280CtrlMeas
        {
            Temperature = (byte)ctrl.Temperature,
            Pressure = (byte)ctrl.Pressure,
            Mode = (byte)ctrl.Mode,
        });

    /// <summary>Parses a <c>ctrl_meas</c> register value.</summary>
    /// <param name="bits">The register value, as read from the part.</param>
    /// <returns>The oversampling and the power mode.</returns>
    public static Bme280CtrlMeas CtrlMeasFromBits(byte bits)
    {
        Status.ThrowIfError(NativeMethods.pamoja_bme280_ctrl_meas_from_bits(bits, out PamojaBme280CtrlMeas ctrl));
        return new Bme280CtrlMeas((Oversampling)ctrl.Temperature, (Oversampling)ctrl.Pressure, (Mode)ctrl.Mode);
    }

    /// <summary>Assembles a <c>ctrl_hum</c> register value, which takes effect only after the next <c>ctrl_meas</c> write.</summary>
    /// <param name="humidity">The humidity oversampling.</param>
    /// <returns>The register value to write.</returns>
    public static byte CtrlHumBits(Oversampling humidity) =>
        NativeMethods.pamoja_bme280_ctrl_hum_bits((byte)humidity);

    /// <summary>Parses a <c>ctrl_hum</c> register value.</summary>
    /// <param name="bits">The register value, as read from the part.</param>
    /// <returns>The humidity oversampling.</returns>
    public static Oversampling CtrlHumFromBits(byte bits) =>
        (Oversampling)NativeMethods.pamoja_bme280_ctrl_hum_from_bits(bits);

    /// <summary>Assembles a <c>config</c> register value.</summary>
    /// <param name="config">The standby period, filter, and interface settings.</param>
    /// <returns>The register value to write.</returns>
    public static byte ConfigBits(Bme280Config config) =>
        NativeMethods.pamoja_bme280_config_bits(new PamojaBme280Config
        {
            Standby = (byte)config.Standby,
            Filter = (byte)config.Filter,
            Spi3Wire = config.Spi3Wire ? (byte)1 : (byte)0,
        });

    /// <summary>Parses a <c>config</c> register value.</summary>
    /// <param name="bits">The register value, as read from the part.</param>
    /// <returns>The standby period, filter, and interface settings.</returns>
    public static Bme280Config ConfigFromBits(byte bits)
    {
        Status.ThrowIfError(NativeMethods.pamoja_bme280_config_from_bits(bits, out PamojaBme280Config config));
        return new Bme280Config((Standby)config.Standby, (Filter)config.Filter, config.Spi3Wire != 0);
    }

    /// <summary>Returns how many samples an oversampling setting averages.</summary>
    /// <param name="oversampling">The setting.</param>
    /// <returns>The factor, or 0 when the measurement is skipped.</returns>
    public static byte OversamplingFactor(Oversampling oversampling) =>
        NativeMethods.pamoja_bme280_oversampling_factor((byte)oversampling);

    /// <summary>Returns the period a standby setting rests for in normal mode.</summary>
    /// <param name="standby">The setting.</param>
    /// <returns>The period in microseconds.</returns>
    public static uint StandbyMicros(Standby standby) =>
        NativeMethods.pamoja_bme280_standby_micros((byte)standby);

    /// <summary>Returns the IIR coefficient a filter setting selects.</summary>
    /// <param name="filter">The setting.</param>
    /// <returns>The coefficient, or 0 when the filter is off.</returns>
    public static byte FilterCoefficient(Filter filter) =>
        NativeMethods.pamoja_bme280_filter_coefficient((byte)filter);

    /// <summary>Returns the longest one measurement can take, which is how long a driver waits after forcing one.</summary>
    /// <param name="temperature">The temperature oversampling.</param>
    /// <param name="pressure">The pressure oversampling.</param>
    /// <param name="humidity">The humidity oversampling.</param>
    /// <returns>The datasheet's maximum in microseconds.</returns>
    public static uint MaxMeasurementMicros(Oversampling temperature, Oversampling pressure, Oversampling humidity) =>
        NativeMethods.pamoja_bme280_max_measurement_micros((byte)temperature, (byte)pressure, (byte)humidity);

    /// <summary>Returns the typical time one measurement takes.</summary>
    /// <param name="temperature">The temperature oversampling.</param>
    /// <param name="pressure">The pressure oversampling.</param>
    /// <param name="humidity">The humidity oversampling.</param>
    /// <returns>The datasheet's typical time in microseconds.</returns>
    public static uint TypicalMeasurementMicros(Oversampling temperature, Oversampling pressure, Oversampling humidity) =>
        NativeMethods.pamoja_bme280_typical_measurement_micros((byte)temperature, (byte)pressure, (byte)humidity);

    /// <summary>The registers a driver reads and writes.</summary>
    public static class Register
    {
        /// <summary>The chip-ID register.</summary>
        public const byte ChipId = 0xD0;

        /// <summary>The reset register.</summary>
        public const byte Reset = 0xE0;

        /// <summary>The first of the 26 temperature and pressure calibration bytes.</summary>
        public const byte CalibTempPress = 0x88;

        /// <summary>The first of the 7 humidity calibration bytes.</summary>
        public const byte CalibHumidity = 0xE1;

        /// <summary>The humidity control register, <c>ctrl_hum</c>.</summary>
        public const byte CtrlHum = 0xF2;

        /// <summary>The status register.</summary>
        public const byte Status = 0xF3;

        /// <summary>The measurement control register, <c>ctrl_meas</c>.</summary>
        public const byte CtrlMeas = 0xF4;

        /// <summary>The configuration register, <c>config</c>.</summary>
        public const byte Config = 0xF5;

        /// <summary>The first of the 8 data bytes a burst read covers.</summary>
        public const byte Data = 0xF7;
    }

    /// <summary>A BME280 that is not there, for a bus with nothing plugged in.</summary>
    public static class Sim
    {
        /// <summary>The status a simulated part reports when it is neither measuring nor loading.</summary>
        public const byte StatusIdle = 0x00;

        /// <summary>
        /// Makes a part holding a real BME280's calibration and one measurement it took, which
        /// compensate to 20.44 C, 848.05 hPa, and 44.65 %.
        /// </summary>
        /// <param name="address">The address it answers to.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static I2cPart Part(byte address) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_bme280_sim_part(address),
                NativeMethods.pamoja_i2c_part_free,
                "simulated BME280"));

        /// <summary>Makes a part that reads what it is asked to, to within what its converter can represent.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <param name="celsius">The temperature it reports.</param>
        /// <param name="hectopascals">The pressure it reports.</param>
        /// <param name="relativeHumidity">The humidity it reports, as a percentage.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static I2cPart Reporting(byte address, float celsius, float hectopascals, float relativeHumidity) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_bme280_sim_reporting(address, celsius, hectopascals, relativeHumidity),
                NativeMethods.pamoja_i2c_part_free,
                "simulated BME280"));

        /// <summary>The 26-byte temperature and pressure calibration block a simulated part holds.</summary>
        /// <returns>The block.</returns>
        public static byte[] Calibration()
        {
            byte[] tempPress = new byte[CalibrationTempPressLength];
            byte[] humidity = new byte[CalibrationHumidityLength];
            Status.ThrowIfError(NativeMethods.pamoja_bme280_sim_calibration(tempPress, humidity));
            return tempPress;
        }

        /// <summary>The 7-byte humidity calibration block a simulated part holds.</summary>
        /// <returns>The block.</returns>
        public static byte[] CalibrationHumidity()
        {
            byte[] tempPress = new byte[CalibrationTempPressLength];
            byte[] humidity = new byte[CalibrationHumidityLength];
            Status.ThrowIfError(NativeMethods.pamoja_bme280_sim_calibration(tempPress, humidity));
            return humidity;
        }

        /// <summary>The eight data registers a simulated part holds: one measurement a real part took.</summary>
        /// <returns>The registers.</returns>
        public static byte[] Burst()
        {
            byte[] burst = new byte[DataLength];
            Status.ThrowIfError(NativeMethods.pamoja_bme280_sim_burst(burst));
            return burst;
        }

        /// <summary>Builds the eight data registers that compensate to a reading against the simulated calibration.</summary>
        /// <param name="celsius">The temperature.</param>
        /// <param name="hectopascals">The pressure.</param>
        /// <param name="relativeHumidity">The humidity, as a percentage.</param>
        /// <returns>The bytes a burst read would return.</returns>
        public static byte[] BurstFor(float celsius, float hectopascals, float relativeHumidity)
        {
            byte[] burst = new byte[DataLength];
            Status.ThrowIfError(NativeMethods.pamoja_bme280_sim_burst_for(celsius, hectopascals, relativeHumidity, burst));
            return burst;
        }
    }
}
