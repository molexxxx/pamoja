using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A decoded SHT3x status register. Mirrors &lt;c&gt;PamojaSht3xStatus&lt;/c&gt; in
/// &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSht3xStatus
{
    /// <summary>The 16-bit status word the flags were read from.</summary>
    public ushort Bits;
    /// <summary>1 when at least one alert condition is pending.</summary>
    public byte AlertPending;
    /// <summary>1 while the on-die heater is running.</summary>
    public byte HeaterOn;
    /// <summary>1 when a humidity tracking alert is set.</summary>
    public byte HumidityTrackingAlert;
    /// <summary>1 when a temperature tracking alert is set.</summary>
    public byte TemperatureTrackingAlert;
    /// <summary>1 when the part has reset since the flag was last cleared.</summary>
    public byte ResetDetected;
    /// <summary>1 when the last command could not be processed.</summary>
    public byte CommandFailed;
    /// <summary>1 when the last write failed its checksum.</summary>
    public byte WriteChecksumFailed;
}
