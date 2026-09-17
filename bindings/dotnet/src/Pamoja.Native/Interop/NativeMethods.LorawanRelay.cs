using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for a LoRaWAN relay and a plan's relay channels, mirroring
/// <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>A WOR frame ahead of a join request.</summary>
    public const byte LorawanWorJoinRequest = 0;

    /// <summary>A WOR frame ahead of a Class A uplink.</summary>
    public const byte LorawanWorUplink = 1;

    /// <summary>Counts a plan's default relay channels.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_relay_channel_count(
        IntPtr plan,
        out byte outCount);

    /// <summary>Returns one of a plan's default relay channels.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_relay_channel(
        IntPtr plan,
        byte index,
        out PamojaLoraRelayChannel outChannel);

    /// <summary>Appends the next default relay channel to a plan.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_add_relay_channel(
        IntPtr builder,
        in PamojaLoraRelayChannel channel);

    /// <summary>Derives a root relay session key from a network session key.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_root_wor_s_key(
        ReadOnlySpan<byte> networkKey,
        nuint networkKeyLen,
        Span<byte> outKey);

    /// <summary>Derives an end device's wake-on-radio keys from its root key.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_wor_keys(
        ReadOnlySpan<byte> rootKey,
        nuint rootKeyLen,
        uint devAddr,
        out PamojaLorawanWorKeys outKeys);

    /// <summary>Returns the root relay session key of a session's device.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_session_root_wor_s_key(
        IntPtr session,
        Span<byte> outKey);

    /// <summary>Returns the wake-on-radio keys of a session's device.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_session_wor_keys(
        IntPtr session,
        out PamojaLorawanWorKeys outKeys);

    /// <summary>Builds the WOR frame ahead of a join request.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_wor_join_request(
        in PamojaLorawanCarrier uplink,
        Span<byte> outFrame);

    /// <summary>Builds the WOR frame ahead of a Class A uplink.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_wor_uplink(
        in PamojaLorawanWorKeys keys,
        uint devAddr,
        uint wfcnt,
        in PamojaLorawanCarrier uplink,
        in PamojaLorawanCarrier wor,
        Span<byte> outFrame);

    /// <summary>Reads a WOR frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_wor_parse(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        out PamojaLorawanWor outWor);

    /// <summary>Checks a WOR frame ahead of a Class A uplink and reads its carrier.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_wor_open(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        in PamojaLorawanWorKeys keys,
        uint wfcnt,
        in PamojaLorawanCarrier wor,
        out PamojaLorawanCarrier outUplink);

    /// <summary>Builds a relay's WOR ACK.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_wor_ack(
        in PamojaLorawanWorKeys keys,
        uint devAddr,
        uint wfcnt,
        in PamojaLorawanCarrier ack,
        in PamojaLorawanCarrier uplink,
        in PamojaLorawanStateSync state,
        Span<byte> outFrame);

    /// <summary>Checks and reads a WOR ACK.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_wor_ack_open(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        in PamojaLorawanWorKeys keys,
        uint devAddr,
        uint wfcnt,
        in PamojaLorawanCarrier ack,
        in PamojaLorawanCarrier uplink,
        out PamojaLorawanStateSync outState);

    /// <summary>Writes an uplink a relay forwards.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_forward_encode(
        in PamojaLorawanUplinkMetadata metadata,
        uint frequencyHz,
        ReadOnlySpan<byte> phyPayload,
        nuint phyPayloadLen,
        out IntPtr outPayload);

    /// <summary>Reads an uplink a relay forwarded.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_forward_parse(
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out PamojaLorawanUplinkMetadata outMetadata,
        out uint outFrequencyHz,
        out IntPtr outPhyPayload);

    /// <summary>Works out an unsynchronized WOR preamble.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_unsynchronized_preamble(
        byte cadPeriodicity,
        ulong symbolUs,
        byte cadToRx,
        out ushort outSymbols);

    /// <summary>Works out the offset a relay reports in a WOR ACK.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_t_offset_ms(
        ulong scanStartUs,
        ulong preambleEndUs,
        out ushort outOffsetMs);

    /// <summary>Works out when a relay scanned from the WOR ACK that answered a frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_synchronization(
        ulong worStartUs,
        ushort preambleSymbols,
        ulong symbolUs,
        in PamojaLorawanStateSync state,
        out PamojaLorawanSynchronization outSynchronization);

    /// <summary>Picks the relay scan a synchronized end device aims its next WOR frame at.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_next_wor(
        in PamojaLorawanSynchronization synchronization,
        ulong nowUs,
        uint deviceXtalPpm,
        ulong symbolUs,
        byte otherChannel,
        out PamojaLorawanWorSlot outSlot,
        out byte outSynchronized);

    /// <summary>Reads the second channel a relay or end device configuration describes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_second_channel(
        byte secondChannelIndex,
        byte dataRate,
        byte ackOffset,
        uint frequencyHz,
        out PamojaLoraRelayChannel outChannel);
}

/// <summary>A default channel of a LoRaWAN relay, mirroring <c>PamojaLoraRelayChannel</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraRelayChannel
{
    /// <summary>Where an end device sends its wake-on-radio frame, in hertz.</summary>
    public uint WorFrequencyHz;

    /// <summary>Where the relay acknowledges it, in hertz.</summary>
    public uint AckFrequencyHz;

    /// <summary>The data rate of both.</summary>
    public byte DataRate;
}

/// <summary>An end device's wake-on-radio keys, mirroring <c>PamojaLorawanWorKeys</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanWorKeys
{
    /// <summary><c>WorSIntKey</c>.</summary>
    public PamojaId Integrity;

    /// <summary><c>WorSEncKey</c>.</summary>
    public PamojaId Encryption;
}

/// <summary>Where a frame goes and how fast, mirroring <c>PamojaLorawanCarrier</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanCarrier
{
    /// <summary>The frequency in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>The data rate.</summary>
    public byte DataRate;
}

/// <summary>A wake-on-radio frame as a relay reads it, mirroring <c>PamojaLorawanWor</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanWor
{
    /// <summary>Which frame it is.</summary>
    public byte Kind;

    /// <summary>For a join request, where and how fast it follows.</summary>
    public PamojaLorawanCarrier Uplink;

    /// <summary>For an uplink, the address it names.</summary>
    public uint DevAddr;

    /// <summary>For an uplink, the low sixteen bits of its counter.</summary>
    public ushort Wfcnt;
}

/// <summary>What a relay tells an end device, mirroring <c>PamojaLorawanStateSync</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanStateSync
{
    /// <summary>The coded time to start receiving.</summary>
    public byte CadToRx;

    /// <summary>The coded forwarding state.</summary>
    public byte Forward;

    /// <summary>The data rate the relay forwards at.</summary>
    public byte RelayDataRate;

    /// <summary>The coded crystal accuracy.</summary>
    public byte XtalAccuracy;

    /// <summary>The coded scan periodicity.</summary>
    public byte CadPeriodicity;

    /// <summary>Milliseconds from the scan to the end of the WOR preamble.</summary>
    public ushort TOffsetMs;
}

/// <summary>What a relay heard of an uplink, mirroring <c>PamojaLorawanUplinkMetadata</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanUplinkMetadata
{
    /// <summary>The WOR channel, 0 or 1.</summary>
    public byte WorChannel;

    /// <summary>The signal strength in dBm.</summary>
    public short RssiDbm;

    /// <summary>The signal-to-noise ratio in dB.</summary>
    public sbyte SnrDb;

    /// <summary>The data rate.</summary>
    public byte DataRate;
}

/// <summary>What a device knows of a relay's scans, mirroring <c>PamojaLorawanSynchronization</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanSynchronization
{
    /// <summary>When the relay scanned, in the device's microseconds.</summary>
    public ulong ReferenceUs;

    /// <summary>The coded scan periodicity.</summary>
    public byte CadPeriodicity;

    /// <summary>The coded crystal accuracy.</summary>
    public byte RelayXtal;

    /// <summary>The coded time to start receiving.</summary>
    public byte CadToRx;
}

/// <summary>When a WOR frame goes out, mirroring <c>PamojaLorawanWorSlot</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanWorSlot
{
    /// <summary>When to start sending, in microseconds.</summary>
    public ulong StartUs;

    /// <summary>The preamble length in symbols.</summary>
    public ushort PreambleSymbols;
}
