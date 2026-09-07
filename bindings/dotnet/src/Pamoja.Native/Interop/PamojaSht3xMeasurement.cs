using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A decoded SHT3x temperature and humidity pair. Mirrors
/// &lt;c&gt;PamojaSht3xMeasurement&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSht3xMeasurement
{
    /// <summary>The raw temperature word.</summary>
    public ushort TemperatureRaw;
    /// <summary>The raw humidity word.</summary>
    public ushort HumidityRaw;
    /// <summary>The temperature in milli-degrees Celsius, exact in integer arithmetic.</summary>
    public int MilliCelsius;
    /// <summary>The temperature in degrees Celsius.</summary>
    public float Celsius;
    /// <summary>The temperature in milli-degrees Fahrenheit.</summary>
    public int MilliFahrenheit;
    /// <summary>The temperature in degrees Fahrenheit.</summary>
    public float Fahrenheit;
    /// <summary>The relative humidity in milli-percent.</summary>
    public uint MilliPercent;
    /// <summary>The relative humidity as a percentage.</summary>
    public float RelativeHumidity;
}
