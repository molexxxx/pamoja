using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The fields J1939 packs into an extended CAN identifier, mirroring
/// <c>PamojaJ1939Id</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// <see cref="Addressed"/> is <c>1</c> for a PDU1 message, where
/// <see cref="Destination"/> names the node the message is for, and <c>0</c> for a
/// PDU2 broadcast, where it carries no meaning.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaJ1939Id
{
    /// <summary>The parameter group number, which names what the message carries.</summary>
    public uint Pgn;

    /// <summary>The message priority, 0 (highest) to 7.</summary>
    public byte Priority;

    /// <summary>The source address: the node that sent the message.</summary>
    public byte Source;

    /// <summary>The PDU format byte of the parameter group.</summary>
    public byte PduFormat;

    /// <summary>The destination address, meaningful only when addressed.</summary>
    public byte Destination;

    /// <summary><c>1</c> for an addressed (PDU1) message, <c>0</c> for a broadcast.</summary>
    public byte Addressed;
}
