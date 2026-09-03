using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A validated I2C device address, mirroring <c>PamojaI2cAddress</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaI2cAddress
{
    /// <summary>The address itself, without the read/write bit.</summary>
    public ushort Value;

    /// <summary><c>1</c> for a 10-bit address, <c>0</c> for a 7-bit one.</summary>
    public byte TenBit;
}
