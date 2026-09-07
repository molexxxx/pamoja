using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A decoded SCD4x measurement frame. Mirrors &lt;c&gt;PamojaScd4xMeasurement&lt;/c&gt;
/// in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaScd4xMeasurement
{
    /// <summary>The carbon dioxide concentration in parts per million.</summary>
    public ushort Co2Ppm;
    /// <summary>The raw temperature word.</summary>
    public ushort TemperatureRaw;
    /// <summary>The raw humidity word.</summary>
    public ushort HumidityRaw;
    /// <summary>The temperature in milli-degrees Celsius, exact in integer arithmetic.</summary>
    public int MilliCelsius;
    /// <summary>The temperature in degrees Celsius.</summary>
    public float Celsius;
    /// <summary>The relative humidity in milli-percent.</summary>
    public uint HumidityMilliPercent;
    /// <summary>The relative humidity as a percentage.</summary>
    public float RelativeHumidityPercent;
}
