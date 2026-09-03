namespace Pamoja.Can;

/// <summary>A CAN frame: an identifier, its flags, and its payload.</summary>
public sealed class CanFrame
{
    /// <summary>Creates a frame from the fields the native core reported.</summary>
    /// <param name="id">The arbitration identifier, already masked to width.</param>
    /// <param name="extended">Whether the identifier is a 29-bit extended one.</param>
    /// <param name="fd">Whether this is a CAN-FD frame rather than classic CAN 2.0.</param>
    /// <param name="remote">Whether this is a remote transmission request.</param>
    /// <param name="length">The data length, which a remote frame only requests.</param>
    /// <param name="dlc">The data length code as it appears on the wire.</param>
    /// <param name="data">The payload, empty for a remote frame.</param>
    internal CanFrame(
        uint id,
        bool extended,
        bool fd,
        bool remote,
        int length,
        byte dlc,
        byte[] data)
    {
        Id = id;
        Extended = extended;
        Fd = fd;
        Remote = remote;
        Length = length;
        Dlc = dlc;
        Data = data;
    }

    /// <summary>The arbitration identifier, already masked to 11 or 29 bits.</summary>
    public uint Id { get; }

    /// <summary>Whether the identifier is a 29-bit extended one.</summary>
    public bool Extended { get; }

    /// <summary>Whether this is a CAN-FD frame rather than classic CAN 2.0.</summary>
    public bool Fd { get; }

    /// <summary>Whether this is a remote transmission request, which carries no payload.</summary>
    public bool Remote { get; }

    /// <summary>
    /// The data length: the payload length, or the length a remote frame requests
    /// without carrying it.
    /// </summary>
    public int Length { get; }

    /// <summary>The data length code as it appears on the wire.</summary>
    public byte Dlc { get; }

    /// <summary>The payload, empty for a remote frame.</summary>
    public byte[] Data { get; }
}

/// <summary>The fields J1939 packs into an extended CAN identifier.</summary>
public sealed class J1939Message
{
    /// <summary>Creates a message from the fields the native core decoded.</summary>
    /// <param name="pgn">The parameter group number.</param>
    /// <param name="priority">The message priority, 0 (highest) to 7.</param>
    /// <param name="source">The address of the sending node.</param>
    /// <param name="pduFormat">The PDU format byte of the parameter group.</param>
    /// <param name="destination">The destination address, or <c>null</c> for a broadcast.</param>
    /// <param name="broadcast">Whether the message is a broadcast.</param>
    internal J1939Message(
        uint pgn,
        byte priority,
        byte source,
        byte pduFormat,
        byte? destination,
        bool broadcast)
    {
        Pgn = pgn;
        Priority = priority;
        Source = source;
        PduFormat = pduFormat;
        Destination = destination;
        Broadcast = broadcast;
    }

    /// <summary>The parameter group number, which names what the message carries.</summary>
    public uint Pgn { get; }

    /// <summary>The message priority, 0 (highest) to 7.</summary>
    public byte Priority { get; }

    /// <summary>The source address: the node that sent the message.</summary>
    public byte Source { get; }

    /// <summary>The PDU format byte of the parameter group.</summary>
    public byte PduFormat { get; }

    /// <summary>
    /// The destination address for an addressed (PDU1) message, or <c>null</c> for a
    /// broadcast (PDU2) one.
    /// </summary>
    public byte? Destination { get; }

    /// <summary>Whether the message is a broadcast.</summary>
    public bool Broadcast { get; }
}
