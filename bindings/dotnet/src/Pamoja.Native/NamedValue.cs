namespace Pamoja.Native.Interop;

/// <summary>Checks an enum argument before a native call reads it.</summary>
/// <remarks>
/// A C# enum can hold any value of its underlying type, such as <c>(PowerMode)7</c>, but
/// the native library reads an enum as one of the values it declares and has no answer
/// for any other. A value that is not one of its type's named values is refused here,
/// before it crosses.
/// </remarks>
public static class NamedValue
{
    /// <summary>Returns <paramref name="value"/> if it is one of its type's named values.</summary>
    /// <typeparam name="TEnum">The enum type.</typeparam>
    /// <param name="value">The argument.</param>
    /// <param name="name">The argument's name, for the exception.</param>
    /// <returns>The argument, unchanged.</returns>
    /// <exception cref="ArgumentOutOfRangeException">
    /// <paramref name="value"/> is not one of <typeparamref name="TEnum"/>'s named values.
    /// </exception>
    public static TEnum Require<TEnum>(TEnum value, string name)
        where TEnum : struct, Enum =>
        Enum.IsDefined(value)
            ? value
            : throw new ArgumentOutOfRangeException(
                name, value, $"{name} must be one of the {typeof(TEnum).Name} values");
}
