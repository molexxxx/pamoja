namespace Pamoja.Kit;

/// <summary>
/// Where a fix sits relative to a <see cref="Geofence"/>, including the moment it
/// crosses.
/// </summary>
public enum Boundary
{
    /// <summary>The fix is inside the fence and was inside before, or is the first fix inside.</summary>
    Inside = 0,

    /// <summary>The fix is outside the fence and was outside before, or is the first fix outside.</summary>
    Outside = 1,

    /// <summary>The fix just crossed from inside to outside: the moment to raise a breach alert.</summary>
    Exited = 2,

    /// <summary>The fix just crossed from outside back inside.</summary>
    Entered = 3,
}
