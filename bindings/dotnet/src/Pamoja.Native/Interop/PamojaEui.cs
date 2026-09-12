using System.Runtime.CompilerServices;

namespace Pamoja.Native.Interop;

/// <summary>An 8-byte device or gateway identifier carried inline inside a blittable struct.</summary>
/// <remarks>
/// LoRaWAN names a device and a gateway with an EUI, and the C ABI declares one as a fixed
/// array inside a struct that crosses by value, so the managed mirror needs a type of exactly
/// that width rather than a reference to bytes held elsewhere.
/// </remarks>
[InlineArray(Length)]
public struct PamojaEui
{
    /// <summary>The width of an identifier, in bytes.</summary>
    public const int Length = 8;

    private byte _element0;

    /// <summary>Copies an identifier out as an array.</summary>
    /// <returns>The 8 bytes.</returns>
    public readonly byte[] ToArray()
    {
        PamojaEui copy = this;
        return ((ReadOnlySpan<byte>)copy).ToArray();
    }

    /// <summary>Reads an identifier from exactly 8 bytes.</summary>
    /// <param name="bytes">The identifier.</param>
    /// <param name="name">What the bytes are, for the exception message.</param>
    /// <returns>The identifier.</returns>
    /// <exception cref="ArgumentException">
    /// <paramref name="bytes"/> is not <see cref="Length"/> bytes.
    /// </exception>
    public static PamojaEui From(ReadOnlySpan<byte> bytes, string name)
    {
        if (bytes.Length != Length)
        {
            throw new ArgumentException($"{name} must be exactly {Length} bytes", name);
        }

        PamojaEui eui = default;
        bytes.CopyTo(eui);
        return eui;
    }
}
