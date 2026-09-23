using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Bosch BMP280 pressure and temperature sensor, and a driver for one on an I2C bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet, decoded exactly as the manufacturer
/// specifies, and <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives one part over an <see cref="I2cBus"/>, measuring on demand in forced
/// mode. Nothing is sent until <see cref="Init"/> or the first <see cref="Measure"/>. The
/// driver holds its own share of the bus, so the bus may be disposed once the driver is built.
/// </para>
/// </remarks>
public sealed class Bmp280 : IDisposable
{
    /// <summary>The address a BMP280 answers on with its SDO pin low.</summary>
    public const byte AddressPrimary = 0x76;

    /// <summary>The address it answers on with SDO high.</summary>
    public const byte AddressSecondary = 0x77;

    /// <summary>The value its chip-ID register reads, which tells it from a BME280.</summary>
    public const byte ChipId = 0x58;

    /// <summary>How many trimming bytes the part holds from 0x88.</summary>
    public const int CalibrationLength = NativeMethods.Bmp280CalibrationLen;

    /// <summary>How many bytes one measurement burst holds.</summary>
    public const int DataLength = NativeMethods.Bmp280DataLen;

    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part at <paramref name="address"/> on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <param name="address"><see cref="AddressPrimary"/> with SDO low, <see cref="AddressSecondary"/> with SDO high.</param>
    /// <param name="temperature">The temperature oversampling.</param>
    /// <param name="pressure">The pressure oversampling.</param>
    /// <param name="filter">The IIR filter's three-bit code, 0 for the filter off, written as given.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Bmp280(
        I2cBus bus,
        byte address,
        Oversampling temperature = Oversampling.X1,
        Oversampling pressure = Oversampling.X1,
        byte filter = 0)
    {
        ArgumentNullException.ThrowIfNull(bus);
        var settings = new PamojaBmp280Settings
        {
            Temperature = (byte)temperature,
            Pressure = (byte)pressure,
            Filter = filter,
        };
        IntPtr sensor = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held =>
            NativeMethods.pamoja_bmp280_new(held, address, settings, out sensor)));
        _handle = new NativeHandle(sensor, NativeMethods.pamoja_bmp280_free);
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

    /// <summary>The trimming coefficients read at initialization, or null before it.</summary>
    public PamojaBmp280Coefficients? Coefficients =>
        _handle.Use(sensor => NativeMethods.pamoja_bmp280_coefficients(sensor, out PamojaBmp280Coefficients coefficients)
            ? coefficients
            : (PamojaBmp280Coefficients?)null);

    /// <summary>Resets the part, checks it is a BMP280, reads its trimming, and writes the settings.</summary>
    /// <exception cref="PamojaException">
    /// Nothing answered at the address, another part did, or the trimming never finished loading.
    /// </exception>
    public void Init() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_bmp280_init));

    /// <summary>Runs one forced measurement and compensates it, initializing the part first if needed.</summary>
    /// <returns>The reading.</returns>
    /// <exception cref="PamojaException">
    /// As <see cref="Init"/>, and when the part is still measuring after the datasheet's time.
    /// </exception>
    public Bmp280Reading Measure()
    {
        PamojaBmp280Reading reading = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_bmp280_measure(sensor, out reading)));
        return new Bmp280Reading(reading.Celsius, reading.Pascals, reading.Hectopascals);
    }

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

    /// <summary>Unpacks the six data bytes a BMP280 burst read returns.</summary>
    /// <param name="data">The data.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaBmp280Measurement ParseMeasurement(ReadOnlySpan<byte> data)
    {
        PamojaBmp280Measurement value;
        Status.ThrowIfError(NativeMethods.pamoja_bmp280_parse_measurement(data, (nuint)data.Length, out value));
        return value;
    }

    /// <summary>Builds the six data bytes a BMP280 holding these codes would return.</summary>
    /// <param name="pressure">The pressure.</param>
    /// <param name="temperature">The temperature.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte[] MeasurementBytes(uint pressure, uint temperature)
    {
        byte[] bytes = new byte[NativeMethods.Bmp280DataLen];
        Status.ThrowIfError(NativeMethods.pamoja_bmp280_measurement_bytes(pressure, temperature, bytes));
        return bytes;
    }

    /// <summary>Reports whether a BMP280 status byte says a conversion is running.</summary>
    /// <param name="status">The status.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool Measuring(byte status) =>
        NativeMethods.pamoja_bmp280_measuring(status);

    /// <summary>Reports whether a BMP280 status byte says the calibration image is loading.</summary>
    /// <param name="status">The status.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool ImageUpdating(byte status) =>
        NativeMethods.pamoja_bmp280_image_updating(status);

    /// <summary>Assembles a BMP280 ctrl_meas register value.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static byte CtrlMeasBits(PamojaBmp280CtrlMeas config) =>
        NativeMethods.pamoja_bmp280_ctrl_meas_bits(config);

    /// <summary>Parses a BMP280 ctrl_meas register value.</summary>
    /// <param name="bits">The bits.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaBmp280CtrlMeas CtrlMeasFromBits(byte bits)
    {
        PamojaBmp280CtrlMeas value;
        Status.ThrowIfError(NativeMethods.pamoja_bmp280_ctrl_meas_from_bits(bits, out value));
        return value;
    }

    /// <summary>Assembles a BMP280 config register value.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static byte ConfigBits(PamojaBmp280Config config) =>
        NativeMethods.pamoja_bmp280_config_bits(config);

    /// <summary>Parses a BMP280 config register value.</summary>
    /// <param name="bits">The bits.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaBmp280Config ConfigFromBits(byte bits)
    {
        PamojaBmp280Config value;
        Status.ThrowIfError(NativeMethods.pamoja_bmp280_config_from_bits(bits, out value));
        return value;
    }

    /// <summary>Returns how many samples a BMP280 oversampling code averages.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static byte OversamplingFactor(byte code) =>
        NativeMethods.pamoja_bmp280_oversampling_factor(code);

    /// <summary>Returns the normal-mode standby period a BMP280 code selects.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint StandbyMicros(byte code) =>
        NativeMethods.pamoja_bmp280_standby_micros(code);

    /// <summary>A BMP280 that is not there, for a bus with nothing plugged in.</summary>
    /// <remarks>
    /// A BMP280 is a BME280 without humidity, with the same temperature and pressure registers
    /// and trimming layout, so <see cref="Part"/> holds the temperature and pressure half of a
    /// real part and reads 20.44 C and 848.05 hPa.
    /// </remarks>
    public static class Sim
    {
        /// <summary>The status a simulated part reports when it is neither measuring nor loading.</summary>
        public const byte StatusIdle = 0x00;

        /// <summary>Makes a part holding a real part's trimming and one measurement it took.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static I2cPart Part(byte address) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_bmp280_sim_part(address),
                NativeMethods.pamoja_i2c_part_free,
                "simulated BMP280"));

        /// <summary>Makes a part that reads what it is asked to, within a hundredth of a degree and of a hectopascal.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <param name="celsius">The temperature it reports.</param>
        /// <param name="hectopascals">The pressure it reports.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static I2cPart Reporting(byte address, float celsius, float hectopascals) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_bmp280_sim_reporting(address, celsius, hectopascals),
                NativeMethods.pamoja_i2c_part_free,
                "simulated BMP280"));

        /// <summary>The 24 trimming bytes a simulated part holds.</summary>
        /// <returns>The block.</returns>
        public static byte[] Calibration()
        {
            byte[] calibration = new byte[CalibrationLength];
            Status.ThrowIfError(NativeMethods.pamoja_bmp280_sim_calibration(calibration));
            return calibration;
        }

        /// <summary>The six data registers a simulated part holds: one measurement a real part took.</summary>
        /// <returns>The registers.</returns>
        public static byte[] Burst()
        {
            byte[] burst = new byte[DataLength];
            Status.ThrowIfError(NativeMethods.pamoja_bmp280_sim_burst(burst));
            return burst;
        }

        /// <summary>Builds the six data registers that compensate to a reading against the simulated trimming.</summary>
        /// <param name="celsius">The temperature.</param>
        /// <param name="hectopascals">The pressure.</param>
        /// <returns>The bytes a burst read would return.</returns>
        public static byte[] BurstFor(float celsius, float hectopascals)
        {
            byte[] burst = new byte[DataLength];
            Status.ThrowIfError(NativeMethods.pamoja_bmp280_sim_burst_for(celsius, hectopascals, burst));
            return burst;
        }
    }
}
