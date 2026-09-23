using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>The stateless helpers from the pamoja kit.</summary>
/// <remarks>
/// The stateful helpers are their own types (<see cref="Smoother"/>,
/// <see cref="Pid"/>, <see cref="Thermostat"/> and the rest); what remains here is
/// the arithmetic that needs no memory of previous calls.
/// </remarks>
public static class Kit
{
    /// <summary>
    /// The most readings a windowed helper (<see cref="Window"/>, <see cref="Median"/>,
    /// <see cref="Trend"/>, <see cref="Anomaly"/>) keeps, and the number it keeps unless told fewer.
    /// </summary>
    public const int WindowCapacity = NativeMethods.WindowCapacity;

    /// <summary>Holds a reading at the center while it stays within the band, so small wiggle does not act.</summary>
    /// <param name="value">The latest reading.</param>
    /// <param name="center">The value the band sits around.</param>
    /// <param name="width">
    /// How far the band reaches either side of <paramref name="center"/>; its magnitude is used.
    /// </param>
    /// <returns>
    /// <paramref name="center"/> while the reading is within <paramref name="width"/> of it,
    /// and otherwise the reading unchanged.
    /// </returns>
    public static float Deadband(float value, float center, float width) =>
        NativeMethods.pamoja_kit_deadband(value, center, width);

    /// <summary>Returns the great-circle distance between two coordinates.</summary>
    /// <param name="origin">The coordinate to measure from.</param>
    /// <param name="destination">The coordinate to measure to.</param>
    /// <returns>The distance in meters.</returns>
    public static double DistanceBetween(Coordinate origin, Coordinate destination) =>
        NativeMethods.pamoja_coordinate_distance_to(
            Geofence.ToNative(origin), Geofence.ToNative(destination));

    /// <summary>Returns the initial bearing from one coordinate to another.</summary>
    /// <param name="origin">The coordinate to measure from.</param>
    /// <param name="destination">The coordinate to measure to.</param>
    /// <returns>The bearing in degrees, clockwise from north.</returns>
    public static double BearingBetween(Coordinate origin, Coordinate destination) =>
        NativeMethods.pamoja_coordinate_bearing_to(
            Geofence.ToNative(origin), Geofence.ToNative(destination));
}
