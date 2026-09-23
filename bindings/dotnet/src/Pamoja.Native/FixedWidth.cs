namespace Pamoja.Native.Interop;

/// <summary>Checks a fixed-width argument before a native call reads it.</summary>
/// <remarks>
/// A native call that takes a key, an identifier, or a signature reads a fixed number
/// of bytes from the pointer it is given, with no length to check them against. A span
/// of any other length is refused here, before it crosses.
/// </remarks>
public static class FixedWidth
{
    /// <summary>Throws unless <paramref name="value"/> is exactly <paramref name="length"/> bytes.</summary>
    /// <param name="value">The argument.</param>
    /// <param name="length">The number of bytes the native call reads.</param>
    /// <param name="name">The argument's name, for the exception.</param>
    /// <exception cref="ArgumentException"><paramref name="value"/> is another length.</exception>
    public static void Require(ReadOnlySpan<byte> value, int length, string name)
    {
        if (value.Length != length)
        {
            throw new ArgumentException($"{name} must be exactly {length} bytes", name);
        }
    }
}
