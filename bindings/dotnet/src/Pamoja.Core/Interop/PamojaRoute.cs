using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// A learned way to reach one node, mirroring <c>PamojaRoute</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaRoute
{
    /// <summary>The node this route reaches.</summary>
    public uint Dst;

    /// <summary>The neighbour to send a packet to on the way there.</summary>
    public uint NextHop;

    /// <summary>What the route costs, usually in hops.</summary>
    public ushort Cost;
}
