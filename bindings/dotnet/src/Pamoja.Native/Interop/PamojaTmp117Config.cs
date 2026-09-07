using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A TMP117 configuration register, field by field. Mirrors
/// &lt;c&gt;PamojaTmp117Config&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaTmp117Config
{
    /// <summary>1 when a result went above the high limit.</summary>
    public byte HighAlert;
    /// <summary>1 when a result went below the low limit.</summary>
    public byte LowAlert;
    /// <summary>1 when a conversion has completed since the register was last read.</summary>
    public byte DataReady;
    /// <summary>1 while an EEPROM write is still in progress.</summary>
    public byte EepromBusy;
    /// <summary>The conversion-mode code: 0 continuous, 1 shutdown, 3 one-shot.</summary>
    public byte Mode;
    /// <summary>The conversion-cycle code, 0..=7.</summary>
    public byte Cycle;
    /// <summary>The averaging code, 0..=3.</summary>
    public byte Averaging;
    /// <summary>1 makes the limits a therm hysteresis band rather than alerts.</summary>
    public byte ThermMode;
    /// <summary>1 makes the ALERT pin active high.</summary>
    public byte AlertActiveHigh;
    /// <summary>1 makes the ALERT pin reflect data ready rather than the alert flags.</summary>
    public byte AlertPinDataReady;
    /// <summary>1 triggers a software reset when this register is written.</summary>
    public byte SoftReset;
}
