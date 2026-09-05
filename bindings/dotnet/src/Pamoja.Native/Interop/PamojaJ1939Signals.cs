using System.Runtime.CompilerServices;

namespace Pamoja.Native.Interop;

/// <summary>The eight data bytes of a J1939 frame, carried inline by value.</summary>
/// <remarks>
/// The C ABI declares a payload as a fixed array inside a struct that crosses by
/// value, so the managed mirror needs a type of exactly that width rather than a
/// reference to bytes held elsewhere.
/// </remarks>
[InlineArray(Length)]
public struct PamojaJ1939Signals
{
    /// <summary>The width of a J1939 payload, in bytes.</summary>
    public const int Length = 8;

    private byte _element0;

    /// <summary>Copies the payload out as an array.</summary>
    /// <returns>The eight bytes, in wire order.</returns>
    public readonly byte[] ToArray()
    {
        PamojaJ1939Signals copy = this;
        return ((ReadOnlySpan<byte>)copy).ToArray();
    }

    /// <summary>Reads a payload from exactly eight bytes.</summary>
    /// <param name="bytes">The frame's payload.</param>
    /// <param name="name">What the bytes are, for the exception message.</param>
    /// <returns>The payload.</returns>
    /// <exception cref="ArgumentException">
    /// <paramref name="bytes"/> is not <see cref="Length"/> bytes.
    /// </exception>
    public static PamojaJ1939Signals From(ReadOnlySpan<byte> bytes, string name)
    {
        if (bytes.Length != Length)
        {
            throw new ArgumentException($"{name} must be exactly {Length} bytes", name);
        }

        PamojaJ1939Signals signals = default;
        bytes.CopyTo(signals);
        return signals;
    }
}
