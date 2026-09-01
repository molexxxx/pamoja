using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>The stateless helpers from the pamoja kit.</summary>
/// <remarks>
/// The stateful helpers are their own types (<see cref="Smoother"/>,
/// <see cref="Pid"/>, <see cref="Thermostat"/> and the rest); what remains here is
/// the arithmetic that needs no memory of previous calls.
/// </remarks>
public static class Kit
{
    /// <summary>Suppresses movement within a band, so noise does not act.</summary>
    /// <param name="value">The latest reading.</param>
    /// <param name="center">The value the band sits around.</param>
    /// <param name="width">The full width of the band.</param>
    /// <returns>
    /// <paramref name="center"/> while the reading is inside the band, and
    /// otherwise the reading shifted toward the centre by half the band width, so
    /// the output is continuous.
    /// </returns>
    public static float Deadband(float value, float center, float width) =>
        NativeMethods.pamoja_kit_deadband(value, center, width);

    /// <summary>Returns the great-circle distance between two coordinates.</summary>
    /// <param name="origin">The coordinate to measure from.</param>
    /// <param name="destination">The coordinate to measure to.</param>
    /// <returns>The distance in metres.</returns>
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
