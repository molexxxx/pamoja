using Pamoja.Native.Interop;

namespace Pamoja.Can;

/// <summary>The priorities J1939 publishes, so a caller does not write the number out.</summary>
public enum J1939Priority : byte
{
    /// <summary>Ahead of ordinary traffic, for a message that controls something.</summary>
    Control = NativeMethods.J1939PriorityControl,

    /// <summary>What ordinary traffic uses.</summary>
    Normal = NativeMethods.J1939PriorityDefault,

    /// <summary>Yields to everything else on the bus.</summary>
    Lowest = NativeMethods.J1939PriorityLowest,
}

/// <summary>The eight data bytes of a J1939 frame, addressed by the signals inside them.</summary>
/// <remarks>
/// A parameter group places each signal at a fixed byte offset, little-endian. A
/// payload starts with every signal marked not available, so a controller writes
/// only the signals it actually reports.
/// </remarks>
public struct Signals
{
    /// <summary>The byte a J1939 sender writes for a signal it is not reporting.</summary>
    public const byte NotAvailable = NativeMethods.J1939NotAvailable;

    /// <summary>The destination address every node on the bus reads.</summary>
    public const byte BroadcastAddress = NativeMethods.J1939BroadcastAddress;

    private PamojaJ1939Signals _bytes;

    private Signals(PamojaJ1939Signals bytes)
    {
        _bytes = bytes;
    }

    /// <summary>Builds a payload with every signal marked not available.</summary>
    /// <returns>Eight bytes a controller writes only its own signals into.</returns>
    public static Signals New()
    {
        return new Signals(NativeMethods.pamoja_can_signals_new());
    }

    /// <summary>Reads the eight data bytes of a frame that arrived off the bus.</summary>
    /// <param name="data">The frame's payload.</param>
    /// <returns>The payload, ready for its signals to be read out.</returns>
    /// <exception cref="ArgumentException">
    /// <paramref name="data"/> is not exactly eight bytes.
    /// </exception>
    public static Signals From(ReadOnlySpan<byte> data)
    {
        return new Signals(PamojaJ1939Signals.From(data, nameof(data)));
    }

    /// <summary>Writes a one-byte signal at the offset its parameter group defines.</summary>
    /// <param name="at">The byte offset, 0 to 7.</param>
    /// <param name="value">The raw value, already scaled as the group defines.</param>
    public void SetU8(int at, byte value)
    {
        _bytes = NativeMethods.pamoja_can_signals_set_u8(_bytes, (nuint)at, value);
    }

    /// <summary>Writes a two-byte little-endian signal at the offset its group defines.</summary>
    /// <param name="at">The offset of the signal's first byte, 0 to 6.</param>
    /// <param name="value">The raw value, already scaled as the group defines.</param>
    public void SetU16(int at, ushort value)
    {
        _bytes = NativeMethods.pamoja_can_signals_set_u16(_bytes, (nuint)at, value);
    }

    /// <summary>Reads a one-byte signal at the offset its parameter group defines.</summary>
    /// <param name="at">The byte offset.</param>
    /// <returns>The raw value, or <c>null</c> if the offset is past the payload.</returns>
    public readonly byte? U8(int at)
    {
        return NativeMethods.pamoja_can_signals_u8(_bytes, (nuint)at, out byte value)
            ? value
            : null;
    }

    /// <summary>Reads a two-byte little-endian signal at the offset its group defines.</summary>
    /// <param name="at">The offset of the signal's first byte.</param>
    /// <returns>
    /// The raw value, or <c>null</c> if the signal would run past the payload.
    /// </returns>
    public readonly ushort? U16(int at)
    {
        return NativeMethods.pamoja_can_signals_u16(_bytes, (nuint)at, out ushort value)
            ? value
            : null;
    }

    /// <summary>The eight data bytes, ready to put in a frame.</summary>
    /// <returns>The payload in wire order.</returns>
    public readonly byte[] ToArray()
    {
        return _bytes.ToArray();
    }
}
