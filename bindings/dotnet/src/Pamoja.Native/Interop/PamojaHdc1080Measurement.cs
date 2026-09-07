using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A decoded HDC1080 temperature and humidity pair. Mirrors
/// &lt;c&gt;PamojaHdc1080Measurement&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaHdc1080Measurement
{
    /// <summary>The raw temperature register.</summary>
    public ushort TemperatureRaw;
    /// <summary>The raw humidity register.</summary>
    public ushort HumidityRaw;
    /// <summary>The temperature in milli-degrees Celsius, exact in integer arithmetic.</summary>
    public int MilliCelsius;
    /// <summary>The temperature in degrees Celsius.</summary>
    public float Celsius;
    /// <summary>The relative humidity in milli-percent.</summary>
    public uint MilliPercent;
    /// <summary>The relative humidity as a percentage.</summary>
    public float RelativeHumidity;
}
