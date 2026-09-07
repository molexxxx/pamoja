using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// An HDC1080 configuration register, field by field. Mirrors
/// &lt;c&gt;PamojaHdc1080Config&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaHdc1080Config
{
    /// <summary>1 resets the part when this register is written.</summary>
    public byte SoftwareReset;
    /// <summary>1 runs the on-die heater during measurements.</summary>
    public byte Heater;
    /// <summary>1 acquires temperature and humidity from one trigger.</summary>
    public byte Sequential;
    /// <summary>1 when the supply has dropped below 2.8 V, which the part reports back.</summary>
    public byte BatteryLow;
    /// <summary>The temperature resolution in bits: 14 or 11.</summary>
    public byte TemperatureResolutionBits;
    /// <summary>The humidity resolution in bits: 14, 11, or 8.</summary>
    public byte HumidityResolutionBits;
}
