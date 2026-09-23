using System.Globalization;
using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Which way a <see cref="TwoLinkArm"/>'s elbow bends; both reach the same point.</summary>
public enum Elbow
{
    /// <summary>The elbow angle is positive, counter-clockwise.</summary>
    Up = 0,

    /// <summary>The elbow angle is negative, clockwise.</summary>
    Down = 1,
}

/// <summary>A planar arm of two links, with a closed-form inverse.</summary>
/// <param name="L1">The shoulder link's length; its magnitude is used.</param>
/// <param name="L2">The elbow link's length; its magnitude is used.</param>
public readonly record struct TwoLinkArm(float L1, float L2)
{
    /// <summary>Gets the closest and farthest the hand reaches from the shoulder.</summary>
    public (float Min, float Max) Reach
    {
        get
        {
            PamojaReach reach = NativeMethods.pamoja_two_link_arm_reach(ToNative());
            return (reach.Min, reach.Max);
        }
    }

    /// <summary>Returns where the hand is for its joint angles.</summary>
    /// <param name="shoulder">The shoulder angle, in radians from the x axis.</param>
    /// <param name="elbow">The elbow angle, in radians relative to the first link.</param>
    /// <returns>The hand's position.</returns>
    public (float X, float Y) Tip(float shoulder, float elbow)
    {
        PamojaPoint tip = NativeMethods.pamoja_two_link_arm_tip(ToNative(), shoulder, elbow);
        return (tip.X, tip.Y);
    }

    /// <summary>Finds the joint angles that put the hand at a point.</summary>
    /// <param name="x">The target's x coordinate.</param>
    /// <param name="y">The target's y coordinate.</param>
    /// <param name="elbow">Which way the elbow bends.</param>
    /// <returns>
    /// The shoulder and elbow angles, or <c>null</c> for a point out of reach, an arm with a
    /// link of no length, or a coordinate that is not a finite number.
    /// </returns>
    public (float Shoulder, float Elbow)? JointsFor(float x, float y, Elbow elbow = Elbow.Up)
    {
        PamojaElbow branch = elbow == Elbow.Down ? PamojaElbow.Down : PamojaElbow.Up;
        return NativeMethods.pamoja_two_link_arm_joints_for(ToNative(), x, y, branch, out PamojaJoints joints)
            ? (joints.Shoulder, joints.Elbow)
            : null;
    }

    private PamojaTwoLinkArm ToNative() => new() { L1 = L1, L2 = L2 };
}

/// <summary>One joint of a serial arm in the Denavit-Hartenberg convention.</summary>
/// <param name="A">The link length along the common normal, in meters.</param>
/// <param name="Alpha">The link twist about the common normal, in radians.</param>
/// <param name="D">The link offset along the previous z axis, in meters.</param>
/// <param name="Theta">The joint angle about the previous z axis, in radians.</param>
public readonly record struct DhParameters(float A = 0.0f, float Alpha = 0.0f, float D = 0.0f, float Theta = 0.0f)
{
    /// <summary>Returns the homogeneous transform this joint makes.</summary>
    /// <returns>The joint's transform.</returns>
    public Transform Transform() => new(NativeMethods.pamoja_dh_transform(ToNative()));

    /// <summary>Converts the joint into the layout the C ABI expects.</summary>
    /// <returns>The native joint.</returns>
    internal PamojaDhParameters ToNative() => new() { A = A, Alpha = Alpha, D = D, Theta = Theta };
}

/// <summary>A 4x4 homogeneous transform: a rotation and a translation.</summary>
public sealed class Transform : IEquatable<Transform>
{
    private readonly float[] _elements;

    /// <summary>Wraps a transform the C ABI returned.</summary>
    /// <param name="native">The native transform.</param>
    internal Transform(PamojaTransform native)
    {
        _elements = ((ReadOnlySpan<float>)native.M).ToArray();
    }

    /// <summary>Gets the identity: no rotation and no translation.</summary>
    public static Transform Identity => new(NativeMethods.pamoja_transform_identity());

    /// <summary>Gets the sixteen elements, row-major.</summary>
    public IReadOnlyList<float> Elements => _elements;

    /// <summary>Gets where this transform places the origin: its translation.</summary>
    public (float X, float Y, float Z) Position => (_elements[3], _elements[7], _elements[11]);

    /// <summary>Returns <c>this * other</c>, the transform that applies <paramref name="other"/> and then this one.</summary>
    /// <param name="other">The transform further down the chain.</param>
    /// <returns>The product.</returns>
    public Transform Multiply(Transform other)
    {
        ArgumentNullException.ThrowIfNull(other);
        return new Transform(NativeMethods.pamoja_transform_multiply(ToNative(), other.ToNative()));
    }

    /// <inheritdoc/>
    public bool Equals(Transform? other) =>
        other is not null && _elements.AsSpan().SequenceEqual(other._elements);

    /// <inheritdoc/>
    public override bool Equals(object? obj) => Equals(obj as Transform);

    /// <inheritdoc/>
    public override int GetHashCode()
    {
        var hash = new HashCode();
        foreach (float element in _elements)
        {
            hash.Add(element);
        }

        return hash.ToHashCode();
    }

    /// <inheritdoc/>
    public override string ToString() =>
        "Transform(" + string.Join(", ", _elements.Select(e => e.ToString(CultureInfo.InvariantCulture))) + ")";

    private PamojaTransform ToNative()
    {
        PamojaTransform native = default;
        _elements.CopyTo((Span<float>)native.M);
        return native;
    }
}
