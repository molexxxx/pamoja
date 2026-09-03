namespace Pamoja.Native.Interop;

/// <summary>
/// What to do with a packet bound for a given node, mirroring
/// <c>PamojaForward</c> in <c>pamoja.h</c>.
/// </summary>
public enum PamojaForward
{
    /// <summary>The packet is for this node; hand it to the application.</summary>
    Deliver = 0,

    /// <summary>A route is known; unicast the packet to the next hop.</summary>
    Relay = 1,

    /// <summary>No route is known; fall back to flooding the packet.</summary>
    Flood = 2,
}
