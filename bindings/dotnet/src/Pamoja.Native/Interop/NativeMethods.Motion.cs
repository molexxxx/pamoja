using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the motion helpers of the pamoja C ABI: chassis kinematics,
/// an arm, odometry, waypoint guidance, the safety gate, and the servo, ESC, and encoder
/// conversions, mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. Every part must be
/// updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>Returns the wheel speeds a differential drive needs for a body motion.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSideSpeeds pamoja_diff_drive_wheel_speeds(PamojaDiffDrive drive, float linear, float angular);

    /// <summary>Returns the body motion a differential drive makes from measured wheel speeds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaBodyMotion pamoja_diff_drive_body_motion(PamojaDiffDrive drive, float left, float right);

    /// <summary>Returns the steering angle for a forward speed and a yaw rate.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ackermann_steering_angle(PamojaAckermann car, float linear, float angular);

    /// <summary>Returns the yaw rate a forward speed and a steering angle produce.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ackermann_yaw_rate(PamojaAckermann car, float linear, float steering);

    /// <summary>Returns the turn radius of a steering angle, infinity with the wheels straight.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ackermann_turn_radius(PamojaAckermann car, float steering);

    /// <summary>Returns the path curvature of a steering angle.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ackermann_curvature(PamojaAckermann car, float steering);

    /// <summary>Returns the side speeds a skid-steer drive needs for a body motion.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSideSpeeds pamoja_skid_steer_wheel_speeds(PamojaSkidSteer drive, float linear, float angular);

    /// <summary>Returns the body motion a skid-steer drive makes from measured side speeds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaBodyMotion pamoja_skid_steer_body_motion(PamojaSkidSteer drive, float left, float right);

    /// <summary>Returns the four wheel speeds a mecanum base needs for a body twist.</summary>
    [LibraryImport(Library)]
    public static partial PamojaWheelSpeeds pamoja_mecanum_wheel_speeds(PamojaMecanum mecanum, PamojaTwist twist);

    /// <summary>Returns the body twist a mecanum base makes from measured wheel speeds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTwist pamoja_mecanum_body_motion(PamojaMecanum mecanum, PamojaWheelSpeeds wheels);

    /// <summary>Returns how close and how far a two-link arm reaches.</summary>
    [LibraryImport(Library)]
    public static partial PamojaReach pamoja_two_link_arm_reach(PamojaTwoLinkArm arm);

    /// <summary>Returns where a two-link arm's hand is for its joint angles.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPoint pamoja_two_link_arm_tip(PamojaTwoLinkArm arm, float shoulder, float elbow);

    /// <summary>Finds the joint angles that put a two-link arm's hand at a point.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_two_link_arm_joints_for(
        PamojaTwoLinkArm arm,
        float x,
        float y,
        PamojaElbow elbow,
        out PamojaJoints outJoints);

    /// <summary>Returns the transform one Denavit-Hartenberg joint makes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTransform pamoja_dh_transform(PamojaDhParameters joint);

    /// <summary>Returns the transform from a serial arm's base to its tool.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTransform pamoja_forward_kinematics(ReadOnlySpan<PamojaDhParameters> joints, nuint len);

    /// <summary>Returns the product of two transforms.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTransform pamoja_transform_multiply(PamojaTransform first, PamojaTransform second);

    /// <summary>Returns the identity transform.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTransform pamoja_transform_identity();

    /// <summary>Creates an odometry estimate starting from a known pose.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_odometry_new(PamojaPose start);

    /// <summary>Reads an odometry estimate's pose.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPose pamoja_odometry_pose(IntPtr odometry);

    /// <summary>Sets an odometry estimate to a known pose.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_odometry_reset(IntPtr odometry, PamojaPose pose);

    /// <summary>Integrates a body motion over a time step onto an odometry estimate.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPose pamoja_odometry_integrate(IntPtr odometry, float linear, float angular, float dt);

    /// <summary>Integrates the distances two wheels rolled onto an odometry estimate.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPose pamoja_odometry_integrate_wheels(IntPtr odometry, float left, float right, PamojaDiffDrive drive);

    /// <summary>Nudges an odometry estimate's heading toward an absolute measurement.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_odometry_fuse_heading(IntPtr odometry, float measured, float weight);

    /// <summary>Releases an odometry handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_odometry_free(IntPtr odometry);

    /// <summary>Produces the command from a robot's position and heading toward a target.</summary>
    [LibraryImport(Library)]
    public static partial PamojaGuidance pamoja_waypoint_follower_guide(
        PamojaWaypointFollower follower,
        PamojaCoordinate here,
        float headingDeg,
        PamojaCoordinate target);

    /// <summary>Cuts forward and sideways motion when an obstacle is within the stopping distance.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTwist pamoja_obstacle_stop(PamojaTwist twist, float rangeM, float stopDistanceM);

    /// <summary>Creates an e-stop that is not engaged.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_estop_new();

    /// <summary>Engages an e-stop.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_estop_engage(IntPtr estop);

    /// <summary>Clears an e-stop.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_estop_reset(IntPtr estop);

    /// <summary>Reports whether an e-stop is engaged.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_estop_is_engaged(IntPtr estop);

    /// <summary>Passes a command through an e-stop.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTwist pamoja_estop_gate(IntPtr estop, PamojaTwist desired);

    /// <summary>Releases an e-stop handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_estop_free(IntPtr estop);

    /// <summary>Creates a watchdog that expires after a timeout without being fed.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_watchdog_new(float timeout);

    /// <summary>Feeds a watchdog.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_watchdog_feed(IntPtr watchdog);

    /// <summary>Advances a watchdog's timer and reports whether it has expired.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_watchdog_update(IntPtr watchdog, float dt);

    /// <summary>Reports whether a watchdog has expired.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_watchdog_is_expired(IntPtr watchdog);

    /// <summary>Releases a watchdog handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_watchdog_free(IntPtr watchdog);

    /// <summary>Creates limits on speed and acceleration, starting from rest.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_limits_new(float maxLinear, float maxAngular, float maxLinearAccel, float maxAngularAccel);

    /// <summary>Bounds a command in speed and acceleration.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTwist pamoja_limits_apply(IntPtr limits, PamojaTwist desired, float dt);

    /// <summary>Forgets the motion limits last allowed.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_limits_reset(IntPtr limits);

    /// <summary>Releases a limits handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_limits_free(IntPtr limits);

    /// <summary>Creates a safety gate from limits and a watchdog timeout, or null for null limits.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_safety_gate_new(IntPtr limits, float watchdogTimeout);

    /// <summary>Feeds a safety gate's watchdog.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_safety_gate_feed(IntPtr gate);

    /// <summary>Engages a safety gate's e-stop.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_safety_gate_engage_estop(IntPtr gate);

    /// <summary>Clears a safety gate's e-stop.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_safety_gate_reset_estop(IntPtr gate);

    /// <summary>Reports whether a safety gate is forcing a stop.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_safety_gate_is_stopped(IntPtr gate);

    /// <summary>Returns the command that is safe to drive for a desired one over a time step.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTwist pamoja_safety_gate_command(IntPtr gate, PamojaTwist desired, float dt);

    /// <summary>Releases a safety gate handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_safety_gate_free(IntPtr gate);

    /// <summary>Returns the standard hobby servo.</summary>
    [LibraryImport(Library)]
    public static partial PamojaServoMap pamoja_servo_map_standard();

    /// <summary>Returns the pulse width that sets a servo to an angle.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_servo_map_pulse(PamojaServoMap servo, float angleDeg);

    /// <summary>Returns the angle a servo pulse width sets.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_servo_map_angle(PamojaServoMap servo, ushort pulseUs);

    /// <summary>Returns the common reversible ESC.</summary>
    [LibraryImport(Library)]
    public static partial PamojaEsc pamoja_esc_bidirectional();

    /// <summary>Returns the pulse width for a throttle.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_esc_pulse(PamojaEsc esc, float throttle);

    /// <summary>Creates a quadrature decoder seeded with the encoder's current channel levels.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_quadrature_new(
        [MarshalAs(UnmanagedType.U1)] bool a,
        [MarshalAs(UnmanagedType.U1)] bool b);

    /// <summary>Feeds a quadrature decoder the channel levels it reads now.</summary>
    [LibraryImport(Library)]
    public static partial sbyte pamoja_quadrature_update(
        IntPtr quadrature,
        [MarshalAs(UnmanagedType.U1)] bool a,
        [MarshalAs(UnmanagedType.U1)] bool b);

    /// <summary>Reads a quadrature decoder's signed count of steps.</summary>
    [LibraryImport(Library)]
    public static partial long pamoja_quadrature_count(IntPtr quadrature);

    /// <summary>Sets a quadrature decoder's count back to 0.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_quadrature_reset(IntPtr quadrature);

    /// <summary>Releases a quadrature decoder handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_quadrature_free(IntPtr quadrature);

    /// <summary>Returns the distance a wheel rolled for a step count.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_quadrature_scale_distance(PamojaQuadratureScale scale, long count);

    /// <summary>Returns a wheel's speed from the steps counted over a time step.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_quadrature_scale_velocity(PamojaQuadratureScale scale, long deltaCount, float dt);
}

/// <summary>The speeds of a two-sided drive's left and right wheels or tracks.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSideSpeeds
{
    /// <summary>The left side's speed.</summary>
    public float Left;

    /// <summary>The right side's speed.</summary>
    public float Right;
}

/// <summary>A planar body motion: a forward speed and a yaw rate.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBodyMotion
{
    /// <summary>The forward speed.</summary>
    public float Linear;

    /// <summary>The yaw rate, positive turning left.</summary>
    public float Angular;
}

/// <summary>A differential drive: two wheels a track apart.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaDiffDrive
{
    /// <summary>The distance between the wheels; its magnitude is used.</summary>
    public float Track;
}

/// <summary>Car-like steering: axles a wheelbase apart.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaAckermann
{
    /// <summary>The distance from the steered axle to the driven axle; its magnitude is used.</summary>
    public float Wheelbase;
}

/// <summary>A skid-steer drive: sides a track apart that slip to turn.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSkidSteer
{
    /// <summary>The distance between the sides; its magnitude is used.</summary>
    public float Track;

    /// <summary>How much wider the effective track is; 0 is taken as 1.</summary>
    public float Slip;
}

/// <summary>A mecanum base: four wheels a wheelbase and a track apart.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaMecanum
{
    /// <summary>The front-to-rear distance between the axles.</summary>
    public float Wheelbase;

    /// <summary>The left-to-right distance between the wheels.</summary>
    public float Track;
}

/// <summary>The four wheel speeds of a mecanum base.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaWheelSpeeds
{
    /// <summary>The front-left wheel's speed.</summary>
    public float FrontLeft;

    /// <summary>The front-right wheel's speed.</summary>
    public float FrontRight;

    /// <summary>The rear-left wheel's speed.</summary>
    public float RearLeft;

    /// <summary>The rear-right wheel's speed.</summary>
    public float RearRight;
}

/// <summary>A planar two-link arm.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaTwoLinkArm
{
    /// <summary>The shoulder link's length.</summary>
    public float L1;

    /// <summary>The elbow link's length.</summary>
    public float L2;
}

/// <summary>Which way a two-link arm's elbow bends.</summary>
public enum PamojaElbow
{
    /// <summary>The elbow angle is positive, counter-clockwise.</summary>
    Up = 0,

    /// <summary>The elbow angle is negative, clockwise.</summary>
    Down = 1,
}

/// <summary>The closest and farthest a two-link arm's hand reaches.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaReach
{
    /// <summary>The closest reach.</summary>
    public float Min;

    /// <summary>The farthest reach.</summary>
    public float Max;
}

/// <summary>A point in a plane.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaPoint
{
    /// <summary>The x coordinate.</summary>
    public float X;

    /// <summary>The y coordinate.</summary>
    public float Y;
}

/// <summary>A two-link arm's joint angles, in radians.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaJoints
{
    /// <summary>The shoulder angle, from the x axis.</summary>
    public float Shoulder;

    /// <summary>The elbow angle, relative to the first link.</summary>
    public float Elbow;
}

/// <summary>One joint of a serial arm in the Denavit-Hartenberg convention.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaDhParameters
{
    /// <summary>The link length along the common normal, in meters.</summary>
    public float A;

    /// <summary>The link twist about the common normal, in radians.</summary>
    public float Alpha;

    /// <summary>The link offset along the previous z axis, in meters.</summary>
    public float D;

    /// <summary>The joint angle about the previous z axis, in radians.</summary>
    public float Theta;
}

/// <summary>A 4x4 homogeneous transform, sixteen floats in row-major order.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaTransform
{
    /// <summary>The sixteen elements, row 0 first.</summary>
    public PamojaMatrix16 M;
}

/// <summary>Sixteen floats carried inline inside a blittable struct.</summary>
[InlineArray(Length)]
public struct PamojaMatrix16
{
    /// <summary>The number of elements.</summary>
    public const int Length = 16;

    private float _element0;
}

/// <summary>A carrot-following guide's speeds and gains.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaWaypointFollower
{
    /// <summary>The forward speed when pointed at the target.</summary>
    public float Cruise;

    /// <summary>How close, in meters, counts as arrived.</summary>
    public double ArrivalM;

    /// <summary>The yaw rate commanded per radian of heading error.</summary>
    public float HeadingGain;

    /// <summary>The largest yaw rate to command.</summary>
    public float MaxAngular;
}

/// <summary>The command toward a waypoint, with the geometry behind it.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGuidance
{
    /// <summary>The body motion to drive.</summary>
    public PamojaTwist Twist;

    /// <summary>The distance left to the target, in meters.</summary>
    public double DistanceM;

    /// <summary>The heading error to the target, in degrees.</summary>
    public float HeadingErrorDeg;

    /// <summary>1 once the target is within the arrival radius, else 0.</summary>
    public byte Arrived;
}

/// <summary>A hobby servo's pulse range and travel.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaServoMap
{
    /// <summary>The pulse width at zero degrees, in microseconds.</summary>
    public ushort MinUs;

    /// <summary>The pulse width at full travel, in microseconds.</summary>
    public ushort MaxUs;

    /// <summary>The full travel in degrees.</summary>
    public float RangeDeg;
}

/// <summary>An electronic speed controller's pulse widths.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaEsc
{
    /// <summary>The pulse width at full reverse, in microseconds.</summary>
    public ushort MinUs;

    /// <summary>The pulse width at rest, in microseconds.</summary>
    public ushort NeutralUs;

    /// <summary>The pulse width at full forward, in microseconds.</summary>
    public ushort MaxUs;
}

/// <summary>An encoder's resolution and its wheel's radius.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaQuadratureScale
{
    /// <summary>Steps per wheel revolution.</summary>
    public float CountsPerRev;

    /// <summary>The wheel radius in meters.</summary>
    public float WheelRadius;
}
