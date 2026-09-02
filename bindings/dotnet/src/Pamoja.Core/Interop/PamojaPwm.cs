using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// A PCA9685 channel's four register bytes, mirroring <c>PamojaPwm</c> in
/// <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// The field order matches the channel's four consecutive registers, so the whole
/// struct can be written in one bus transaction.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaPwm
{
    /// <summary>The low byte of the count at which the output goes high.</summary>
    public byte OnLow;

    /// <summary>The high byte of that count; bit 4 is the full-on flag.</summary>
    public byte OnHigh;

    /// <summary>The low byte of the count at which the output goes low.</summary>
    public byte OffLow;

    /// <summary>The high byte of that count; bit 4 is the full-off flag.</summary>
    public byte OffHigh;
}
