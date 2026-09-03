using System.Runtime.CompilerServices;

namespace Pamoja.Native.Interop;

/// <summary>A 16-byte identifier carried inline inside a blittable struct.</summary>
/// <remarks>
/// The C ABI declares vendor and device-class identifiers as fixed arrays inside
/// a struct that crosses by value, so the managed mirror needs a type of exactly
/// that width rather than a reference to bytes held elsewhere.
/// </remarks>
[InlineArray(Length)]
public struct PamojaId
{
    /// <summary>The width of an identifier, in bytes.</summary>
    public const int Length = 16;

    private byte _element0;

    /// <summary>Copies an identifier out as an array.</summary>
    /// <returns>The 16 bytes.</returns>
    public readonly byte[] ToArray()
    {
        PamojaId copy = this;
        return ((ReadOnlySpan<byte>)copy).ToArray();
    }

    /// <summary>Reads an identifier from exactly 16 bytes.</summary>
    /// <param name="bytes">The identifier.</param>
    /// <param name="name">What the bytes are, for the exception message.</param>
    /// <returns>The identifier.</returns>
    /// <exception cref="ArgumentException">
    /// <paramref name="bytes"/> is not <see cref="Length"/> bytes.
    /// </exception>
    public static PamojaId From(ReadOnlySpan<byte> bytes, string name)
    {
        if (bytes.Length != Length)
        {
            throw new ArgumentException($"{name} must be exactly {Length} bytes", name);
        }

        PamojaId id = default;
        bytes.CopyTo(id);
        return id;
    }
}

/// <summary>A 32-byte key or digest carried inline inside a blittable struct.</summary>
[InlineArray(Length)]
public struct PamojaDigest
{
    /// <summary>The width of a key or digest, in bytes.</summary>
    public const int Length = 32;

    private byte _element0;

    /// <summary>Copies the value out as an array.</summary>
    /// <returns>The 32 bytes.</returns>
    public readonly byte[] ToArray()
    {
        PamojaDigest copy = this;
        return ((ReadOnlySpan<byte>)copy).ToArray();
    }

    /// <summary>Reads a key or digest from exactly 32 bytes.</summary>
    /// <param name="bytes">The value.</param>
    /// <param name="name">What the bytes are, for the exception message.</param>
    /// <returns>The value.</returns>
    /// <exception cref="ArgumentException">
    /// <paramref name="bytes"/> is not <see cref="Length"/> bytes.
    /// </exception>
    public static PamojaDigest From(ReadOnlySpan<byte> bytes, string name)
    {
        if (bytes.Length != Length)
        {
            throw new ArgumentException($"{name} must be exactly {Length} bytes", name);
        }

        PamojaDigest digest = default;
        bytes.CopyTo(digest);
        return digest;
    }
}

/// <summary>A 16-byte authentication tag carried inline inside a blittable struct.</summary>
[InlineArray(Length)]
public struct PamojaTag
{
    /// <summary>The width of a tag, in bytes.</summary>
    public const int Length = 16;

    private byte _element0;

    /// <summary>Copies the tag out as an array.</summary>
    /// <returns>The 16 bytes.</returns>
    public readonly byte[] ToArray()
    {
        PamojaTag copy = this;
        return ((ReadOnlySpan<byte>)copy).ToArray();
    }

    /// <summary>Reads a tag from exactly 16 bytes.</summary>
    /// <param name="bytes">The tag.</param>
    /// <param name="name">What the bytes are, for the exception message.</param>
    /// <returns>The tag.</returns>
    /// <exception cref="ArgumentException">
    /// <paramref name="bytes"/> is not <see cref="Length"/> bytes.
    /// </exception>
    public static PamojaTag From(ReadOnlySpan<byte> bytes, string name)
    {
        if (bytes.Length != Length)
        {
            throw new ArgumentException($"{name} must be exactly {Length} bytes", name);
        }

        PamojaTag tag = default;
        bytes.CopyTo(tag);
        return tag;
    }
}
