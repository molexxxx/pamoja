namespace Pamoja.Native.Interop;

/// <summary>
/// The direction a frame travelled, mirroring <c>PamojaLorawanDirection</c> in
/// <c>pamoja.h</c>.
/// </summary>
public enum PamojaLorawanDirection
{
    /// <summary>From an end device up to the network.</summary>
    Uplink = 0,

    /// <summary>From the network down to an end device.</summary>
    Downlink = 1,
}
