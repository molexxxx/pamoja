using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// An ADS1115 configuration register field by field, mirroring
/// <c>PamojaAds1115Config</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// The multi-way settings carry the code the datasheet prints; the single-bit
/// settings are <c>1</c> for the state their name describes.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaAds1115Config
{
    /// <summary><c>1</c> starts a single conversion when written.</summary>
    public byte StartConversion;

    /// <summary>The input multiplexer code, 0 to 7.</summary>
    public byte Mux;

    /// <summary>The gain code, 0 to 7, which sets the full-scale range.</summary>
    public byte Pga;

    /// <summary><c>1</c> converts once per request, <c>0</c> continuously.</summary>
    public byte SingleShot;

    /// <summary>The data rate code, 0 to 7.</summary>
    public byte DataRate;

    /// <summary><c>1</c> selects the window comparator.</summary>
    public byte WindowComparator;

    /// <summary><c>1</c> makes the ALERT/RDY pin active high.</summary>
    public byte ComparatorActiveHigh;

    /// <summary><c>1</c> latches the comparator until the conversion is read.</summary>
    public byte ComparatorLatching;

    /// <summary>The comparator queue code, 0 to 3, where 3 disables it.</summary>
    public byte ComparatorQueue;
}
