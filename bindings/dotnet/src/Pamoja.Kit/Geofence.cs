using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Keeps a tracked point inside an area, and notices when it leaves.</summary>
/// <remarks>
/// A fence is a centre and a radius. Feeding it successive fixes reports whether
/// each is inside or outside and, crucially, the single fix that crossed, so an
/// alert fires once on the crossing rather than on every fix while away.
/// </remarks>
/// <example>
/// <code>
/// using var pen = new Geofence(new Coordinate(-1.2921, 36.8219), 50.0);
/// pen.Update(new Coordinate(-1.2921, 36.8219)); // Boundary.Inside
/// pen.Update(new Coordinate(-1.2930, 36.8219)); // Boundary.Exited
/// </code>
/// </example>
public sealed class Geofence : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a circular fence around a centre fix.</summary>
    /// <param name="center">The centre of the fence.</param>
    /// <param name="radiusM">The fence radius, in metres.</param>
    /// <exception cref="PamojaException">The native fence could not be created.</exception>
    public Geofence(Coordinate center, double radiusM)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_geofence_new(ToNative(center), radiusM),
            NativeMethods.pamoja_geofence_free,
            "geofence");
    }

    /// <summary>Feeds a fix in and reports where it sits, including a crossing.</summary>
    /// <param name="point">The latest fix.</param>
    /// <returns>The boundary state for this fix.</returns>
    public Boundary Update(Coordinate point) =>
        (Boundary)_handle.Use(handle =>
            NativeMethods.pamoja_geofence_update(handle, ToNative(point)));

    /// <summary>Reports whether a fix lies inside, without recording a crossing.</summary>
    /// <param name="point">The fix to test.</param>
    /// <returns><c>true</c> if the fix is inside the fence.</returns>
    public bool Contains(Coordinate point) =>
        _handle.Use(handle => NativeMethods.pamoja_geofence_contains(handle, ToNative(point)));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Converts a coordinate into the layout the C ABI expects.</summary>
    /// <param name="value">The coordinate to convert.</param>
    /// <returns>The native coordinate.</returns>
    internal static PamojaCoordinate ToNative(Coordinate value) =>
        new() { Latitude = value.Latitude, Longitude = value.Longitude };
}
