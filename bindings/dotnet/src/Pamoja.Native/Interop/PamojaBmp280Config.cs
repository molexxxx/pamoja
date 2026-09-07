using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A BMP280 config register, field by field. Mirrors
/// &lt;c&gt;PamojaBmp280Config&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBmp280Config
{
    /// <summary>The normal-mode standby code, 0..=7.</summary>
    public byte Standby;
    /// <summary>The IIR filter code, 0..=7.</summary>
    public byte Filter;
    /// <summary>1 enables the 3-wire SPI interface.</summary>
    public byte Spi3wire;
}
