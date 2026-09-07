using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// An OPT3001 configuration register, field by field. Mirrors
/// &lt;c&gt;PamojaOpt3001Config&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaOpt3001Config
{
    /// <summary>The full-scale range number, 0..=11, or 12 to set the range automatically.</summary>
    public byte RangeNumber;
    /// <summary>1 makes a conversion take 800 ms rather than 100 ms.</summary>
    public byte LongConversion;
    /// <summary>The mode code: 0 shutdown, 1 single shot, 2 continuous.</summary>
    public byte Mode;
    /// <summary>1 when the last result overflowed its range.</summary>
    public byte Overflow;
    /// <summary>1 when a conversion has completed since the register was last read.</summary>
    public byte ConversionReady;
    /// <summary>1 when the result went above the high limit.</summary>
    public byte FlagHigh;
    /// <summary>1 when the result went below the low limit.</summary>
    public byte FlagLow;
    /// <summary>1 latches the INT pin until the configuration register is read.</summary>
    public byte LatchedWindow;
    /// <summary>1 makes the INT pin active high.</summary>
    public byte ActiveHigh;
    /// <summary>1 makes the limit registers carry a mantissa alone, without an exponent.</summary>
    public byte MaskExponent;
    /// <summary>The fault-count code, 0..=3, for one, two, four, or eight faults.</summary>
    public byte FaultCount;
}
