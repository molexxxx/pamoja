using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// What a network grants a device that joined, mirroring
/// <c>PamojaLorawanGrant</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanGrant
{
    /// <summary>A nonce this network must not reuse; low 24 bits only.</summary>
    public uint AppNonce;

    /// <summary>The network identifier; low 24 bits only.</summary>
    public uint NetId;

    /// <summary>The address to assign the device.</summary>
    public uint DevAddr;

    /// <summary>The downlink settings byte.</summary>
    public byte DlSettings;

    /// <summary>The delay before the first receive window, in seconds.</summary>
    public byte RxDelay;
}
