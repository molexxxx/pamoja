using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A decoded INA226 die-ID register. Mirrors &lt;c&gt;PamojaIna226DieId&lt;/c&gt; in
/// &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaIna226DieId
{
    /// <summary>The 12-bit device identifier.</summary>
    public ushort Device;
    /// <summary>The 4-bit die revision.</summary>
    public byte Revision;
}
