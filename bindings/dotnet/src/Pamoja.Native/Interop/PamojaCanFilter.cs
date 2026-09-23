using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>A CAN receive filter, as the C ABI carries it.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaCanFilter
{
    /// <summary>The identifier to match.</summary>
    public uint Id;

    /// <summary>The identifier bits that have to match.</summary>
    public uint Mask;

    /// <summary>1 for an extended 29-bit identifier, 0 for a standard one.</summary>
    public byte Extended;
}
