using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>Which control policy a profile applies to each reading.</summary>
public enum PamojaControlKind
{
    /// <summary>Hold a reading near a setpoint by switching an output on and off.</summary>
    Setpoint = 0,

    /// <summary>Watch a falling level and warn before it reaches empty.</summary>
    Level = 1,

    /// <summary>Warn when a reading changes faster than a limit.</summary>
    Surge = 2,

    /// <summary>Report readings only, with no output and no alerts.</summary>
    Monitor = 3,
}

/// <summary>Which threshold a reading crossed, if any.</summary>
public enum PamojaAlertKind
{
    /// <summary>The reading raised nothing.</summary>
    None = 0,

    /// <summary>A controlled reading drifted outside its safe band.</summary>
    OutOfRange = 1,

    /// <summary>A falling level will reach empty within a few more samples.</summary>
    RunningOut = 2,

    /// <summary>A reading is changing faster than its safe rate.</summary>
    ChangingFast = 3,
}

/// <summary>The ROS 2 subsystem a name belongs to, which fixes its DDS prefix.</summary>
public enum PamojaEntityKind
{
    /// <summary>A topic, which takes the <c>rt</c> prefix.</summary>
    Topic = 0,

    /// <summary>The request side of a service, which takes the <c>rq</c> prefix.</summary>
    ServiceRequest = 1,

    /// <summary>The reply side of a service, which takes the <c>rr</c> prefix.</summary>
    ServiceResponse = 2,
}

/// <summary>A control policy, flattened so every variant crosses as one value.</summary>
/// <remarks>Only the fields belonging to <c>Kind</c> carry meaning; the rest are zero.</remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaControlSpec
{
    /// <summary>Which policy this describes.</summary>
    public PamojaControlKind Kind;

    /// <summary>The target reading, for a setpoint policy.</summary>
    public float Setpoint;

    /// <summary>Half the deadband width, for a setpoint policy.</summary>
    public float Hysteresis;

    /// <summary>Whether the output cools rather than heats, for a setpoint policy.</summary>
    public byte Cooling;

    /// <summary>How far the reading may stray before an alert, for a setpoint policy.</summary>
    public float SafeBand;

    /// <summary>The level treated as empty, for a level policy.</summary>
    public float Empty;

    /// <summary>How many samples ahead to warn, for a level policy.</summary>
    public uint WarnWithin;

    /// <summary>Whether a rise rather than a fall is watched, for a surge policy.</summary>
    public byte Rising;

    /// <summary>The largest safe change per sample, for a surge policy.</summary>
    public float Limit;
}

/// <summary>How often a node samples as its battery drains, in whole seconds.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaPowerSchedule
{
    /// <summary>Seconds between samples at a healthy charge.</summary>
    public ulong ActiveSecs;

    /// <summary>Seconds between samples while conserving.</summary>
    public ulong SaverSecs;

    /// <summary>Seconds between samples when critically low.</summary>
    public ulong CriticalSecs;

    /// <summary>Enter the saver cadence below this state of charge.</summary>
    public float SaverBelow;

    /// <summary>Enter the critical cadence below this state of charge.</summary>
    public float CriticalBelow;
}

/// <summary>What a controller decided about one reading.</summary>
/// <remarks>Only the field belonging to <c>Alert</c> carries meaning; the rest are zero.</remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaReaction
{
    /// <summary>Whether the profile drives an output at all.</summary>
    public byte HasActuator;

    /// <summary>The setting the output should take, when <c>HasActuator</c> is nonzero.</summary>
    public byte Actuator;

    /// <summary>Which threshold the reading crossed.</summary>
    public PamojaAlertKind Alert;

    /// <summary>The offending reading, for an out-of-range alert.</summary>
    public float Reading;

    /// <summary>The estimated samples until empty, for a running-out alert.</summary>
    public uint Samples;

    /// <summary>The change since the previous sample, for a changing-fast alert.</summary>
    public float Rate;
}

/// <summary>A RIHS01 type hash: the digest that identifies a message definition.</summary>
/// <remarks>
/// The C ABI declares the digest as a fixed array inside a struct that crosses by
/// value, so the managed mirror needs a type of exactly that width.
/// </remarks>
[InlineArray(Length)]
public struct PamojaTypeHashDigest
{
    /// <summary>The width of a digest, in bytes.</summary>
    public const int Length = 32;

    private byte _element0;

    /// <summary>Copies a digest out as an array.</summary>
    /// <returns>The 32 bytes.</returns>
    public readonly byte[] ToArray()
    {
        PamojaTypeHashDigest copy = this;
        return ((ReadOnlySpan<byte>)copy).ToArray();
    }
}

/// <summary>A RIHS01 type hash as it crosses the boundary.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaTypeHash
{
    /// <summary>The SHA-256 digest the hash carries.</summary>
    public PamojaTypeHashDigest Digest;
}

/// <summary>A three-dimensional vector, matching <c>geometry_msgs/msg/Vector3</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaVector3
{
    /// <summary>The x component.</summary>
    public double X;

    /// <summary>The y component.</summary>
    public double Y;

    /// <summary>The z component.</summary>
    public double Z;
}

/// <summary>A body velocity command, matching <c>geometry_msgs/msg/Twist</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaRos2Twist
{
    /// <summary>The linear velocity in metres per second.</summary>
    public PamojaVector3 Linear;

    /// <summary>The angular velocity in radians per second.</summary>
    public PamojaVector3 Angular;
}
