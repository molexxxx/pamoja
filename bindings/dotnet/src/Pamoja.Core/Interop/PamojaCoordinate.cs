using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// A latitude and longitude in degrees, mirroring <c>PamojaCoordinate</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaCoordinate
{
    /// <summary>Degrees north of the equator, negative for south.</summary>
    public double Latitude;

    /// <summary>Degrees east of the prime meridian, negative for west.</summary>
    public double Longitude;
}
