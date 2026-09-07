using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// An INA226 configuration register, field by field. Mirrors
/// &lt;c&gt;PamojaIna226Config&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaIna226Config
{
    /// <summary>1 resets the part when this register is written.</summary>
    public byte Reset;
    /// <summary>The averaging code, 0..=7, from 1 to 1024 samples.</summary>
    public byte Averaging;
    /// <summary>The bus-voltage conversion-time code, 0..=7.</summary>
    public byte BusConversionTime;
    /// <summary>The shunt-voltage conversion-time code, 0..=7.</summary>
    public byte ShuntConversionTime;
    /// <summary>The operating-mode code, 0..=7.</summary>
    public byte Mode;
}
