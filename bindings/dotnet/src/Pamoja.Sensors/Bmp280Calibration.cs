using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>A compensated BMP280 reading.</summary>
public sealed class Bmp280Reading
{
    /// <summary>Creates a reading from the compensated values.</summary>
    /// <param name="celsius">The temperature in degrees Celsius.</param>
    /// <param name="pascals">The pressure in pascals.</param>
    /// <param name="hectopascals">The pressure in hectopascals.</param>
    internal Bmp280Reading(float celsius, uint pascals, float hectopascals)
    {
        Celsius = celsius;
        Pascals = pascals;
        Hectopascals = hectopascals;
    }

    /// <summary>The temperature in degrees Celsius.</summary>
    public float Celsius { get; }

    /// <summary>The pressure in pascals.</summary>
    public uint Pascals { get; }

    /// <summary>The pressure in hectopascals.</summary>
    public float Hectopascals { get; }
}

/// <summary>
/// A BMP280's factory calibration, read once out of the registers and reused for every
/// measurement, since the compensation the datasheet specifies needs it.
/// </summary>
public sealed class Bmp280Calibration : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Builds a calibration from the bytes read out of the registers.</summary>
    /// <param name="bytes">The 24-byte coefficient block from 0x88.</param>
    /// <exception cref="PamojaException">The block is the wrong length.</exception>
    public Bmp280Calibration(ReadOnlySpan<byte> bytes)
    {
        Status.ThrowIfError(NativeMethods.pamoja_bmp280_calibration_new(
            bytes,
            (nuint)bytes.Length,
            out IntPtr calibration));
        _handle = NativeHandle.Create(
            calibration, NativeMethods.pamoja_bmp280_calibration_free, "BMP280 calibration");
    }

    /// <summary>Turns a six-byte burst read into a compensated reading.</summary>
    /// <param name="measurement">The six measurement registers from 0xF7.</param>
    /// <returns>The compensated reading.</returns>
    /// <exception cref="PamojaException">The measurement is not six bytes.</exception>
    public Bmp280Reading Compensate(ReadOnlySpan<byte> measurement)
    {
        byte[] copy = measurement.ToArray();
        PamojaBmp280Reading reading = _handle.Use(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_bmp280_compensate(
                handle, copy, (nuint)copy.Length, out PamojaBmp280Reading produced));
            return produced;
        });
        return new Bmp280Reading(reading.Celsius, reading.Pascals, reading.Hectopascals);
    }

    /// <summary>Rebuilds the register bytes the coefficients were read from.</summary>
    /// <returns>The 24 coefficient bytes in register order.</returns>
    /// <exception cref="PamojaException">The calibration has been disposed.</exception>
    public byte[] ToBytes()
    {
        byte[] bytes = new byte[NativeMethods.Bmp280CalibrationLen];
        _handle.Use(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_bmp280_calibration_to_bytes(handle, bytes));
            return 0;
        });
        return bytes;
    }

    /// <summary>Reads the individual coefficients the compensation is computed from.</summary>
    /// <returns>The coefficients as the datasheet names them.</returns>
    /// <exception cref="PamojaException">The calibration has been disposed.</exception>
    public PamojaBmp280Coefficients Coefficients() =>
        _handle.Use(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_bmp280_calibration_coefficients(
                handle, out PamojaBmp280Coefficients coefficients));
            return coefficients;
        });

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
