using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A BMP280's per-chip trimming coefficients, as they sit in its registers. Mirrors
/// &lt;c&gt;PamojaBmp280Coefficients&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBmp280Coefficients
{
    /// <summary>The dig_T1 coefficient.</summary>
    public ushort DigT1;
    /// <summary>The dig_T2 coefficient.</summary>
    public short DigT2;
    /// <summary>The dig_T3 coefficient.</summary>
    public short DigT3;
    /// <summary>The dig_P1 coefficient.</summary>
    public ushort DigP1;
    /// <summary>The dig_P2 coefficient.</summary>
    public short DigP2;
    /// <summary>The dig_P3 coefficient.</summary>
    public short DigP3;
    /// <summary>The dig_P4 coefficient.</summary>
    public short DigP4;
    /// <summary>The dig_P5 coefficient.</summary>
    public short DigP5;
    /// <summary>The dig_P6 coefficient.</summary>
    public short DigP6;
    /// <summary>The dig_P7 coefficient.</summary>
    public short DigP7;
    /// <summary>The dig_P8 coefficient.</summary>
    public short DigP8;
    /// <summary>The dig_P9 coefficient.</summary>
    public short DigP9;
}
