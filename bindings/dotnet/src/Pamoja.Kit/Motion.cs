using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Where a robot is and which way it faces.</summary>
/// <param name="X">Position along the world x axis, in meters.</param>
/// <param name="Y">Position along the world y axis, in meters.</param>
/// <param name="Theta">
/// Heading from the world x axis, in radians, positive counter-clockwise. A pose the helpers
/// return has its heading in <c>(-pi, pi]</c>, and one passed in is wrapped there.
/// </param>
public readonly record struct Pose(float X, float Y, float Theta)
{
    /// <summary>Converts a pose into the layout the C ABI expects.</summary>
    /// <returns>The native pose.</returns>
    internal PamojaPose ToNative() => new() { X = X, Y = Y, Theta = Theta };

    /// <summary>Converts a pose the C ABI returned.</summary>
    /// <param name="pose">The native pose.</param>
    /// <returns>The pose.</returns>
    internal static Pose From(PamojaPose pose) => new(pose.X, pose.Y, pose.Theta);
}

/// <summary>How fast a robot is asked to move.</summary>
/// <param name="Vx">Forward speed along the x axis.</param>
/// <param name="Vy">Leftward speed along the y axis; zero for drives that cannot strafe.</param>
/// <param name="Omega">Yaw rate about the z axis, positive counter-clockwise.</param>
public readonly record struct Twist(float Vx, float Vy = 0.0f, float Omega = 0.0f)
{
    /// <summary>Converts a twist into the layout the C ABI expects.</summary>
    /// <returns>The native twist.</returns>
    internal PamojaTwist ToNative() => new() { Vx = Vx, Vy = Vy, Omega = Omega };

    /// <summary>Converts a twist the C ABI returned.</summary>
    /// <param name="twist">The native twist.</param>
    /// <returns>The twist.</returns>
    internal static Twist From(PamojaTwist twist) => new(twist.Vx, twist.Vy, twist.Omega);
}

/// <summary>
/// Wheel speeds for a desired body motion, and the body motion measured wheel speeds make, for
/// a robot that steers by spinning two wheels at different speeds.
/// </summary>
/// <param name="Track">The distance between the left and right wheels; its magnitude is used.</param>
public readonly record struct DiffDrive(float Track)
{
    /// <summary>Returns the wheel speeds for a forward speed and a turn rate.</summary>
    /// <param name="linear">The forward speed.</param>
    /// <param name="angular">The turn rate, positive turning left.</param>
    /// <returns>The left and right wheel speeds.</returns>
    public (float Left, float Right) WheelSpeeds(float linear, float angular)
    {
        PamojaSideSpeeds speeds = NativeMethods.pamoja_diff_drive_wheel_speeds(ToNative(), linear, angular);
        return (speeds.Left, speeds.Right);
    }

    /// <summary>Returns the body motion measured wheel speeds make.</summary>
    /// <param name="left">The left wheel speed.</param>
    /// <param name="right">The right wheel speed.</param>
    /// <returns>The forward speed and turn rate; the turn rate is 0 for a drive with no track.</returns>
    public (float Linear, float Angular) BodyMotion(float left, float right)
    {
        PamojaBodyMotion motion = NativeMethods.pamoja_diff_drive_body_motion(ToNative(), left, right);
        return (motion.Linear, motion.Angular);
    }

    /// <summary>Converts the drive into the layout the C ABI expects.</summary>
    /// <returns>The native drive.</returns>
    internal PamojaDiffDrive ToNative() => new() { Track = Track };
}

/// <summary>Car-like steering: one steered axle and a driven axle a wheelbase apart.</summary>
/// <param name="Wheelbase">The distance from the steered axle to the driven axle; its magnitude is used.</param>
public readonly record struct Ackermann(float Wheelbase)
{
    /// <summary>Returns the steering angle for a forward speed and a yaw rate.</summary>
    /// <param name="linear">The forward speed.</param>
    /// <param name="angular">The desired yaw rate, positive turning left.</param>
    /// <returns>The steering angle in radians, or 0 while the vehicle is stopped.</returns>
    public float SteeringAngle(float linear, float angular) =>
        NativeMethods.pamoja_ackermann_steering_angle(ToNative(), linear, angular);

    /// <summary>Returns the yaw rate a forward speed and a steering angle produce.</summary>
    /// <param name="linear">The forward speed.</param>
    /// <param name="steering">The steering angle in radians.</param>
    /// <returns>The yaw rate, or 0 for a vehicle with no wheelbase.</returns>
    public float YawRate(float linear, float steering) =>
        NativeMethods.pamoja_ackermann_yaw_rate(ToNative(), linear, steering);

    /// <summary>Returns the turn radius of a steering angle.</summary>
    /// <param name="steering">The steering angle in radians.</param>
    /// <returns>The radius in meters, or <see cref="float.PositiveInfinity"/> with the wheels straight.</returns>
    public float TurnRadius(float steering) =>
        NativeMethods.pamoja_ackermann_turn_radius(ToNative(), steering);

    /// <summary>Returns the path curvature of a steering angle, the reciprocal of the turn radius.</summary>
    /// <param name="steering">The steering angle in radians.</param>
    /// <returns>The curvature, or 0 for a vehicle with no wheelbase.</returns>
    public float Curvature(float steering) =>
        NativeMethods.pamoja_ackermann_curvature(ToNative(), steering);

    private PamojaAckermann ToNative() => new() { Wheelbase = Wheelbase };
}

/// <summary>A tracked or four-wheel drive that turns by skidding, with its track widened by a slip factor.</summary>
/// <param name="Track">The distance between the left and right sides; its magnitude is used.</param>
/// <param name="Slip">How much wider the effective track is than the geometric one; its magnitude is used, and 0 is taken as 1.</param>
public readonly record struct SkidSteer(float Track, float Slip = 1.0f)
{
    /// <summary>Returns the side speeds for a forward speed and a yaw rate.</summary>
    /// <param name="linear">The forward speed.</param>
    /// <param name="angular">The yaw rate, positive turning left.</param>
    /// <returns>The left and right speeds, split across the slip-corrected track.</returns>
    public (float Left, float Right) WheelSpeeds(float linear, float angular)
    {
        PamojaSideSpeeds speeds = NativeMethods.pamoja_skid_steer_wheel_speeds(ToNative(), linear, angular);
        return (speeds.Left, speeds.Right);
    }

    /// <summary>Returns the body motion measured side speeds make.</summary>
    /// <param name="left">The left side's speed.</param>
    /// <param name="right">The right side's speed.</param>
    /// <returns>The forward speed and yaw rate; the yaw rate is 0 for a drive with no track.</returns>
    public (float Linear, float Angular) BodyMotion(float left, float right)
    {
        PamojaBodyMotion motion = NativeMethods.pamoja_skid_steer_body_motion(ToNative(), left, right);
        return (motion.Linear, motion.Angular);
    }

    private PamojaSkidSteer ToNative() => new() { Track = Track, Slip = Slip };
}

/// <summary>The four wheel speeds of a mecanum base.</summary>
/// <param name="FrontLeft">The front-left wheel's speed.</param>
/// <param name="FrontRight">The front-right wheel's speed.</param>
/// <param name="RearLeft">The rear-left wheel's speed.</param>
/// <param name="RearRight">The rear-right wheel's speed.</param>
public readonly record struct WheelSpeeds(float FrontLeft, float FrontRight, float RearLeft, float RearRight);

/// <summary>A four-wheel mecanum base, which drives, strafes, and turns at once.</summary>
/// <param name="Wheelbase">The front-to-rear distance between the axles; its magnitude is used.</param>
/// <param name="Track">The left-to-right distance between the wheels; its magnitude is used.</param>
public readonly record struct Mecanum(float Wheelbase, float Track)
{
    /// <summary>Returns the four wheel speeds for a body twist.</summary>
    /// <param name="twist">The body motion, using its forward, sideways, and yaw parts.</param>
    /// <returns>The four wheel speeds.</returns>
    public WheelSpeeds WheelSpeeds(Twist twist)
    {
        PamojaWheelSpeeds wheels = NativeMethods.pamoja_mecanum_wheel_speeds(ToNative(), twist.ToNative());
        return new WheelSpeeds(wheels.FrontLeft, wheels.FrontRight, wheels.RearLeft, wheels.RearRight);
    }

    /// <summary>Returns the body twist measured wheel speeds make.</summary>
    /// <param name="wheels">The four measured wheel speeds.</param>
    /// <returns>The body twist; its yaw rate is 0 for a base with no size.</returns>
    public Twist BodyMotion(WheelSpeeds wheels)
    {
        PamojaWheelSpeeds native = new()
        {
            FrontLeft = wheels.FrontLeft,
            FrontRight = wheels.FrontRight,
            RearLeft = wheels.RearLeft,
            RearRight = wheels.RearRight,
        };
        return Twist.From(NativeMethods.pamoja_mecanum_body_motion(ToNative(), native));
    }

    private PamojaMecanum ToNative() => new() { Wheelbase = Wheelbase, Track = Track };
}
