using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// What a frame says about itself before any key is involved, mirroring
/// <c>PamojaLorawanHeader</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// <see cref="IsData"/> is <c>1</c> when <see cref="DevAddr"/> and
/// <see cref="Fcnt"/> are meaningful, and <see cref="HasFport"/> when
/// <see cref="Fport"/> is. Nothing here is authenticated.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanHeader
{
    /// <summary>The length of the still-encrypted payload, in bytes.</summary>
    public nuint PayloadLen;

    /// <summary>The device address, meaningful only when addressed.</summary>
    public uint DevAddr;

    /// <summary>The low 16 bits of the frame counter.</summary>
    public ushort Fcnt;

    /// <summary>What kind of message the frame is.</summary>
    public PamojaLorawanMessageType MessageType;

    /// <summary>The port the frame was sent on.</summary>
    public byte Fport;

    /// <summary><c>1</c> for a data frame, <c>0</c> for a join frame.</summary>
    public byte IsData;

    /// <summary><c>1</c> when the frame carries a port.</summary>
    public byte HasFport;

    /// <summary><c>1</c> when the frame asks to be acknowledged.</summary>
    public byte Confirmed;

    /// <summary><c>1</c> when the frame takes part in adaptive data rate.</summary>
    public byte Adr;

    /// <summary><c>1</c> when the frame acknowledges the last confirmed one.</summary>
    public byte Ack;

    /// <summary><c>1</c> when the network has more downlink data waiting.</summary>
    public byte FPending;

    /// <summary>How many bytes of frame options the header carries.</summary>
    public byte FoptsLen;
}
