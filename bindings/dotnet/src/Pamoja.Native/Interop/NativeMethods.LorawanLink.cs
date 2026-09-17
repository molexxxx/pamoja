using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for what keeps a LoRaWAN link running, mirroring
/// <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>LoRaWAN 1.0.3.</summary>
    public const byte LorawanVersion103 = 3;

    /// <summary>TS001-1.0.4, the LoRaWAN 1.0.4 link layer.</summary>
    public const byte LorawanVersion104 = 4;

    /// <summary>The number of bytes a channel list occupies.</summary>
    public const int LorawanCfListLen = 16;

    /// <summary>How many frequencies a type 0 channel list carries.</summary>
    public const int LorawanCfListFrequencies = 5;

    /// <summary>How many sixteen-bit masks a type 1 channel list carries.</summary>
    public const int LorawanCfListMaskGroups = 6;

    /// <summary>The CFListType byte of a list of frequencies.</summary>
    public const byte LorawanCfListTypeFrequencies = 0;

    /// <summary>The CFListType byte of a list of channel mask groups.</summary>
    public const byte LorawanCfListTypeChannelMasks = 1;

    /// <summary>Starts a back-off count from zero.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_backoff_new(
        byte version,
        uint limit,
        uint delay,
        out IntPtr outBackoff);

    /// <summary>Counts one new uplink, and says what to do before sending it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_backoff_uplink(
        IntPtr backoff,
        byte defaultDataRate,
        out PamojaLorawanBackoffStep outStep);

    /// <summary>Counts a Class A downlink, which resets the count.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_backoff_downlink(IntPtr backoff);

    /// <summary>Returns how many uplinks have gone unanswered.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_lorawan_backoff_counter(IntPtr backoff);

    /// <summary>Returns the revision whose steps a count follows.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_lorawan_backoff_version(IntPtr backoff);

    /// <summary>Releases a back-off handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_backoff_free(IntPtr backoff);

    /// <summary>Builds a type 0 channel list from frequencies.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_cflist_from_frequencies(
        ReadOnlySpan<uint> frequenciesHz,
        nuint len,
        Span<byte> outCflist);

    /// <summary>Builds a type 1 channel list from channel mask groups.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_cflist_from_channel_masks(
        ReadOnlySpan<ushort> masks,
        nuint len,
        Span<byte> outCflist);

    /// <summary>Reads which form a channel list takes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_cflist_type(
        ReadOnlySpan<byte> cflist,
        nuint cflistLen,
        out byte outType);

    /// <summary>Reads the frequencies out of a type 0 channel list.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_cflist_frequencies(
        ReadOnlySpan<byte> cflist,
        nuint cflistLen,
        Span<uint> outFrequenciesHz,
        nuint len);

    /// <summary>Reads the mask groups out of a type 1 channel list.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_cflist_channel_masks(
        ReadOnlySpan<byte> cflist,
        nuint cflistLen,
        Span<ushort> outMasks,
        nuint len);

    /// <summary>Reports whether a type 1 channel list enables a channel.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_cflist_enables(
        ReadOnlySpan<byte> cflist,
        nuint cflistLen,
        byte channel,
        out byte outEnabled);
}

/// <summary>
/// What a back-off says to do with one uplink, mirroring <c>PamojaLorawanBackoffStep</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanBackoffStep
{
    /// <summary><c>1</c> to set the ADRACKReq bit.</summary>
    public byte RequestAck;

    /// <summary><c>1</c> to go back to the default transmit power.</summary>
    public byte RestorePower;

    /// <summary><c>1</c> to step the data rate down.</summary>
    public byte LowerDataRate;

    /// <summary><c>1</c> to re-enable the default channels and reset the repetition count.</summary>
    public byte RestoreChannels;
}
