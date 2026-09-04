using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A compensated BME280 reading, mirroring <c>PamojaBme280Measurement</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBme280Measurement
{
    /// <summary>The temperature in degrees Celsius.</summary>
    public float Celsius;

    /// <summary>The pressure in pascals.</summary>
    public uint Pascals;

    /// <summary>The pressure in hectopascals, as a barometer is usually quoted.</summary>
    public float Hectopascals;

    /// <summary>The relative humidity as a percentage.</summary>
    public float RelativeHumidityPercent;
}
