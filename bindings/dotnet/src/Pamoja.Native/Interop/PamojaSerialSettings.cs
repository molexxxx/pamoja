using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>A serial port's speed and character format, as the C ABI carries it.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSerialSettings
{
    /// <summary>The speed, in bits a second.</summary>
    public uint Baud;

    /// <summary>One of the <c>Parity</c> codes on <see cref="NativeMethods"/>.</summary>
    public byte Parity;

    /// <summary>1 or 2.</summary>
    public byte StopBits;
}
