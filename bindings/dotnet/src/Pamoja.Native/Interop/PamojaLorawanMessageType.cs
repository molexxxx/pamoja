namespace Pamoja.Native.Interop;

/// <summary>
/// What kind of message a frame is, mirroring <c>PamojaLorawanMessageType</c> in
/// <c>pamoja.h</c>.
/// </summary>
public enum PamojaLorawanMessageType
{
    /// <summary>A device asking to join a network.</summary>
    JoinRequest = 0,

    /// <summary>A network admitting a device.</summary>
    JoinAccept = 1,

    /// <summary>Data from a device that does not need acknowledging.</summary>
    UnconfirmedUp = 2,

    /// <summary>Data from a device that asks to be acknowledged.</summary>
    ConfirmedUp = 3,

    /// <summary>Data to a device that does not need acknowledging.</summary>
    UnconfirmedDown = 4,

    /// <summary>Data to a device that asks to be acknowledged.</summary>
    ConfirmedDown = 5,
}
