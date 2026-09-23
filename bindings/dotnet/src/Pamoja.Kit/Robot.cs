using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Tracks a robot's pose by adding up its motion.</summary>
/// <remarks>
/// Each step is integrated as the exact arc the robot drives, so a robot that drives and
/// turns at once follows its curve rather than cutting the corner. A speed, rate, time step,
/// or distance that is not a finite number is ignored.
/// </remarks>
public sealed class Odometry : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an estimate starting at a known pose.</summary>
    /// <param name="start">The starting pose, the origin facing along x unless given.</param>
    public Odometry(Pose start = default)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_odometry_new(start.ToNative()),
            NativeMethods.pamoja_odometry_free,
            "odometry",
            serialized: true);
    }

    /// <summary>Gets the pose so far.</summary>
    public Pose Pose => Pose.From(_handle.Use(NativeMethods.pamoja_odometry_pose));

    /// <summary>Sets the estimate to a known pose.</summary>
    /// <param name="pose">The pose to set.</param>
    public void Reset(Pose pose) =>
        _handle.Use(handle => NativeMethods.pamoja_odometry_reset(handle, pose.ToNative()));

    /// <summary>Adds a forward speed and yaw rate held for a time step.</summary>
    /// <param name="linear">The forward speed.</param>
    /// <param name="angular">The yaw rate, positive turning left.</param>
    /// <param name="dt">The length of the step.</param>
    /// <returns>The new pose.</returns>
    public Pose Integrate(float linear, float angular, float dt) =>
        Pose.From(_handle.Use(handle => NativeMethods.pamoja_odometry_integrate(handle, linear, angular, dt)));

    /// <summary>Adds the distances two wheels rolled, through a differential drive.</summary>
    /// <param name="left">The distance the left wheel rolled since the last update.</param>
    /// <param name="right">The distance the right wheel rolled since the last update.</param>
    /// <param name="drive">The drive, for the track between the wheels.</param>
    /// <returns>The new pose.</returns>
    public Pose IntegrateWheels(float left, float right, DiffDrive drive) =>
        Pose.From(_handle.Use(handle =>
            NativeMethods.pamoja_odometry_integrate_wheels(handle, left, right, drive.ToNative())));

    /// <summary>Nudges the heading toward an absolute measurement, the way a compass tames gyro drift.</summary>
    /// <param name="measured">The absolute heading in radians.</param>
    /// <param name="weight">How strongly to trust it, from 0, ignore it, to 1, take it outright.</param>
    public void FuseHeading(float measured, float weight) =>
        _handle.Use(handle => NativeMethods.pamoja_odometry_fuse_heading(handle, measured, weight));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>The command toward a waypoint, with the geometry behind it.</summary>
/// <param name="Twist">The body motion to drive.</param>
/// <param name="DistanceM">The distance left to the target, in meters.</param>
/// <param name="HeadingErrorDeg">The heading error to the target, in degrees, in <c>(-180, 180]</c>.</param>
/// <param name="Arrived">Whether the target is within the arrival radius.</param>
public readonly record struct Guidance(Twist Twist, double DistanceM, float HeadingErrorDeg, bool Arrived);

/// <summary>Steers toward a waypoint: pivot toward it, then drive, slowing as the heading error grows.</summary>
/// <param name="Cruise">The forward speed when pointed at the target; its magnitude is used.</param>
/// <param name="ArrivalM">How close, in meters, counts as arrived; its magnitude is used.</param>
/// <param name="HeadingGain">The yaw rate commanded per radian of heading error; its magnitude is used.</param>
/// <param name="MaxAngular">The largest yaw rate to command; its magnitude is used.</param>
public readonly record struct WaypointFollower(float Cruise, double ArrivalM, float HeadingGain, float MaxAngular)
{
    /// <summary>Produces the command from a position and a compass heading toward a target.</summary>
    /// <param name="here">The robot's position.</param>
    /// <param name="headingDeg">The robot's heading in degrees clockwise from north.</param>
    /// <param name="target">The waypoint.</param>
    /// <returns>
    /// The guidance. Within the arrival radius its twist is zero; a heading or position that is
    /// not a finite number also gives a zero twist.
    /// </returns>
    public Guidance Guide(Coordinate here, float headingDeg, Coordinate target)
    {
        PamojaWaypointFollower follower = new()
        {
            Cruise = Cruise,
            ArrivalM = ArrivalM,
            HeadingGain = HeadingGain,
            MaxAngular = MaxAngular,
        };
        PamojaGuidance guidance = NativeMethods.pamoja_waypoint_follower_guide(
            follower, Geofence.ToNative(here), headingDeg, Geofence.ToNative(target));
        return new Guidance(
            Twist.From(guidance.Twist), guidance.DistanceM, guidance.HeadingErrorDeg, guidance.Arrived != 0);
    }
}

/// <summary>An emergency stop that latches until a person resets it.</summary>
public sealed class EStop : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an e-stop that is not engaged.</summary>
    public EStop()
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_estop_new(), NativeMethods.pamoja_estop_free, "e-stop", serialized: true);
    }

    /// <summary>Gets whether the stop is engaged.</summary>
    public bool IsEngaged => _handle.Use(NativeMethods.pamoja_estop_is_engaged);

    /// <summary>Engages the stop; it holds until reset.</summary>
    public void Engage() => _handle.Use(NativeMethods.pamoja_estop_engage);

    /// <summary>Clears the stop.</summary>
    public void Reset() => _handle.Use(NativeMethods.pamoja_estop_reset);

    /// <summary>Passes a command through the stop.</summary>
    /// <param name="desired">The command to apply while clear.</param>
    /// <returns><paramref name="desired"/> while clear, or a zero twist while engaged.</returns>
    public Twist Gate(Twist desired) =>
        Twist.From(_handle.Use(handle => NativeMethods.pamoja_estop_gate(handle, desired.ToNative())));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>A deadman timer that expires unless fed often enough.</summary>
public sealed class Watchdog : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a watchdog that expires after a silence of <paramref name="timeout"/>.</summary>
    /// <param name="timeout">The allowed silence; its magnitude is used, and one that is not a number is taken as 0.</param>
    public Watchdog(float timeout)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_watchdog_new(timeout), NativeMethods.pamoja_watchdog_free, "watchdog", serialized: true);
    }

    /// <summary>Gets whether the watchdog has expired.</summary>
    public bool IsExpired => _handle.Use(NativeMethods.pamoja_watchdog_is_expired);

    /// <summary>Feeds the watchdog, restarting its silence timer.</summary>
    public void Feed() => _handle.Use(NativeMethods.pamoja_watchdog_feed);

    /// <summary>Advances the timer and reports whether it has expired.</summary>
    /// <param name="dt">The time since the last update; one that is not a finite number expires the watchdog until it is fed.</param>
    /// <returns>Whether the watchdog has expired.</returns>
    public bool Update(float dt) => _handle.Use(handle => NativeMethods.pamoja_watchdog_update(handle, dt));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Speed and acceleration limits, with the motion they last allowed.</summary>
public sealed class Limits : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates limits starting from rest.</summary>
    /// <param name="maxLinear">The largest planar speed.</param>
    /// <param name="maxAngular">The largest yaw rate.</param>
    /// <param name="maxLinearAccel">The largest change in speed per second.</param>
    /// <param name="maxAngularAccel">The largest change in yaw rate per second.</param>
    /// <remarks>Each magnitude is used, and one that is not a number is taken as 0, which holds the robot still.</remarks>
    public Limits(float maxLinear, float maxAngular, float maxLinearAccel, float maxAngularAccel)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_limits_new(maxLinear, maxAngular, maxLinearAccel, maxAngularAccel),
            NativeMethods.pamoja_limits_free,
            "limits",
            serialized: true);
    }

    /// <summary>Holds a command to the speed limits and eases toward it within the acceleration limits.</summary>
    /// <param name="desired">The requested motion; a part that is not a finite number is taken as 0.</param>
    /// <param name="dt">The time since the last call; one that is not a finite number allows no change.</param>
    /// <returns>The bounded command.</returns>
    public Twist Apply(Twist desired, float dt) =>
        Twist.From(_handle.Use(handle => NativeMethods.pamoja_limits_apply(handle, desired.ToNative(), dt)));

    /// <summary>Forgets the motion last allowed, so the next command eases up from rest.</summary>
    public void Reset() => _handle.Use(NativeMethods.pamoja_limits_reset);

    /// <summary>Creates a native safety gate that starts from these limits as they stand.</summary>
    /// <param name="watchdogTimeout">The gate's watchdog timeout.</param>
    /// <returns>The native gate handle.</returns>
    internal IntPtr NewGate(float watchdogTimeout) =>
        _handle.Use(handle => NativeMethods.pamoja_safety_gate_new(handle, watchdogTimeout));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>The gate every motion command passes through: an e-stop, a watchdog, and limits.</summary>
public sealed class SafetyGate : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a gate from limits, copied as they stand, and a watchdog timeout.</summary>
    /// <param name="limits">The limits for normal motion; they stay the caller's.</param>
    /// <param name="watchdogTimeout">The silence after which the gate stops the robot.</param>
    public SafetyGate(Limits limits, float watchdogTimeout)
    {
        ArgumentNullException.ThrowIfNull(limits);
        _handle = NativeHandle.Create(
            limits.NewGate(watchdogTimeout), NativeMethods.pamoja_safety_gate_free, "safety gate", serialized: true);
    }

    /// <summary>Gets whether the gate is forcing a stop: its e-stop is engaged or its watchdog expired.</summary>
    public bool IsStopped => _handle.Use(NativeMethods.pamoja_safety_gate_is_stopped);

    /// <summary>Feeds the watchdog; call it whenever a fresh command arrives.</summary>
    public void Feed() => _handle.Use(NativeMethods.pamoja_safety_gate_feed);

    /// <summary>Engages the latching e-stop.</summary>
    public void EngageEstop() => _handle.Use(NativeMethods.pamoja_safety_gate_engage_estop);

    /// <summary>Clears the e-stop.</summary>
    public void ResetEstop() => _handle.Use(NativeMethods.pamoja_safety_gate_reset_estop);

    /// <summary>Returns the command that is safe to drive for a desired one over a time step.</summary>
    /// <param name="desired">The requested motion.</param>
    /// <param name="dt">The time since the last call.</param>
    /// <returns>A zero twist while stopped, otherwise <paramref name="desired"/> bounded by the limits.</returns>
    public Twist Command(Twist desired, float dt) =>
        Twist.From(_handle.Use(handle => NativeMethods.pamoja_safety_gate_command(handle, desired.ToNative(), dt)));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
