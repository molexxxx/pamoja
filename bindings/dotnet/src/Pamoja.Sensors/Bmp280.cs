using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Bosch BMP280 pressure and temperature sensor. Every call goes straight to the
/// pamoja C ABI, which decodes exactly what the manufacturer's datasheet specifies.
/// </summary>
public static class Bmp280
{
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
}
