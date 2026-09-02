using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// The header flags a sender sets on a data frame, mirroring
/// <c>PamojaLorawanFlags</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// Each field is <c>1</c> for on and <c>0</c> for off. <see cref="FPending"/>
/// applies to a downlink only and is ignored when encoding an uplink.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanFlags
{
    /// <summary>Ask the far end to acknowledge this frame.</summary>
    public byte Confirmed;

    /// <summary>Mark the frame as taking part in adaptive data rate.</summary>
    public byte Adr;

    /// <summary>Acknowledge the last confirmed frame from the far end.</summary>
    public byte Ack;

    /// <summary>Tell the device more downlink data is waiting.</summary>
    public byte FPending;
}
