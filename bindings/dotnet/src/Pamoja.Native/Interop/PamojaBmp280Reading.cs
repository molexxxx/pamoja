using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A compensated BMP280 reading. Mirrors &lt;c&gt;PamojaBmp280Reading&lt;/c&gt; in
/// &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBmp280Reading
{
    /// <summary>The temperature in degrees Celsius.</summary>
    public float Celsius;
    /// <summary>The pressure in pascals.</summary>
    public uint Pascals;
    /// <summary>The pressure in hectopascals, the unit a barometer is usually quoted in.</summary>
    public float Hectopascals;
}
