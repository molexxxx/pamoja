using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// An INA226 Mask/Enable register, field by field. Mirrors
/// &lt;c&gt;PamojaIna226MaskEnable&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaIna226MaskEnable
{
    /// <summary>1 alerts when the shunt voltage exceeds the limit.</summary>
    public byte ShuntOverLimit;
    /// <summary>1 alerts when the shunt voltage drops below the limit.</summary>
    public byte ShuntUnderLimit;
    /// <summary>1 alerts when the bus voltage exceeds the limit.</summary>
    public byte BusOverLimit;
    /// <summary>1 alerts when the bus voltage drops below the limit.</summary>
    public byte BusUnderLimit;
    /// <summary>1 alerts when the power exceeds the limit.</summary>
    public byte PowerOverLimit;
    /// <summary>1 also alerts when a conversion completes.</summary>
    public byte ConversionReady;
    /// <summary>1 when the selected limit function caused the last alert.</summary>
    public byte AlertFunctionFlag;
    /// <summary>1 when every conversion and multiplication has completed.</summary>
    public byte ConversionReadyFlag;
    /// <summary>1 when an arithmetic overflow left current and power invalid.</summary>
    public byte MathOverflow;
    /// <summary>1 makes the alert pin active high.</summary>
    public byte AlertActiveHigh;
    /// <summary>1 latches the alert pin until this register is read.</summary>
    public byte AlertLatch;
}
