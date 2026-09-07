using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A BMP280 ctrl_meas register, field by field. Mirrors
/// &lt;c&gt;PamojaBmp280CtrlMeas&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBmp280CtrlMeas
{
    /// <summary>The temperature oversampling code, 0..=5, where 0 skips the measurement.</summary>
    public byte Temperature;
    /// <summary>The pressure oversampling code, 0..=5, where 0 skips the measurement.</summary>
    public byte Pressure;
    /// <summary>The power mode code: 0 sleep, 1 forced, 3 normal.</summary>
    public byte Mode;
}
