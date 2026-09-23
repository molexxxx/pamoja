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

    /// <summary>Computes roll and pitch from a three-axis accelerometer at rest.</summary>
    /// <param name="ax">Acceleration along the forward axis, in any unit.</param>
    /// <param name="ay">Acceleration along the right axis, in the same unit.</param>
    /// <param name="az">Acceleration along the up axis, in the same unit.</param>
    /// <returns>The tilt in degrees; only the ratios between axes set it.</returns>
    public static Tilt TiltFromAccel(double ax, double ay, double az)
    {
        PamojaTilt tilt = NativeMethods.pamoja_imu_tilt_from_accel(ax, ay, az);
        return new Tilt(tilt.Roll, tilt.Pitch);
    }

    /// <summary>Computes the dew point from the air temperature and relative humidity.</summary>
    /// <param name="celsius">The air temperature, in degrees Celsius.</param>
    /// <param name="humidityPercent">The relative humidity, in percent.</param>
    /// <returns>The dew point, in degrees Celsius.</returns>
    public static double DewPoint(double celsius, double humidityPercent) =>
        NativeMethods.pamoja_weather_dew_point(celsius, humidityPercent);

    /// <summary>Returns the transform from a serial arm's base to its tool.</summary>
    /// <param name="joints">The arm's joints, base first.</param>
    /// <returns>The composed transform, or the identity for an arm with no joints.</returns>
    public static Transform ForwardKinematics(params DhParameters[] joints)
    {
        ArgumentNullException.ThrowIfNull(joints);
        PamojaDhParameters[] native = Array.ConvertAll(joints, joint => joint.ToNative());
        return new Transform(NativeMethods.pamoja_forward_kinematics(native, (nuint)native.Length));
    }

    /// <summary>Cuts forward and sideways motion when an obstacle is within the stopping distance, keeping the turn.</summary>
    /// <param name="twist">The requested motion.</param>
    /// <param name="rangeM">The nearest range ahead, in meters; one that is not a number counts as an obstacle.</param>
    /// <param name="stopDistanceM">The range at or below which motion is cut; its magnitude is used.</param>
    /// <returns><paramref name="twist"/> while the way is clear, or it with no forward or sideways speed.</returns>
    public static Twist ObstacleStop(Twist twist, float rangeM, float stopDistanceM) =>
        Twist.From(NativeMethods.pamoja_obstacle_stop(twist.ToNative(), rangeM, stopDistanceM));
}
