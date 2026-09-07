using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The uncompensated codes a BMP280 burst read carries. Mirrors
/// &lt;c&gt;PamojaBmp280Measurement&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBmp280Measurement
{
    /// <summary>The 20-bit pressure code.</summary>
    public uint Pressure;
    /// <summary>The 20-bit temperature code.</summary>
    public uint Temperature;
    /// <summary>1 when pressure oversampling was off, so the code carries no reading.</summary>
    public byte PressureSkipped;
    /// <summary>1 when temperature oversampling was off, so the code carries no reading.</summary>
    public byte TemperatureSkipped;
}
