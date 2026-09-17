using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for a running LoRaWAN relay and for the relay mode of an end
/// device, mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The device never sends through a relay, the default.</summary>
    public const byte LorawanRelayDisabled = 0;

    /// <summary>It always does.</summary>
    public const byte LorawanRelayEnabled = 1;

    /// <summary>It does once a run of uplinks goes unanswered.</summary>
    public const byte LorawanRelayDynamic = 2;

    /// <summary>The device itself decides.</summary>
    public const byte LorawanRelayDeviceControlled = 3;

    /// <summary>No wake-on-radio frame has gone out yet.</summary>
    public const byte LorawanRelayInitialized = 0;

    /// <summary>One went out unanswered, so the next preamble spans a scan period.</summary>
    public const byte LorawanRelayUnsynchronized = 1;

    /// <summary>The device knows when the relay next scans.</summary>
    public const byte LorawanRelaySynchronized = 2;

    /// <summary>The uplink goes out at the time the exchange named.</summary>
    public const byte LorawanWorNextUplink = 0;

    /// <summary>The relay is woken again first.</summary>
    public const byte LorawanWorNextWakeUp = 1;

    /// <summary>A wake-on-radio frame ahead of a join request the relay lets through.</summary>
    public const byte LorawanWakeJoinRequest = 0;

    /// <summary>One ahead of a trusted device's uplink.</summary>
    public const byte LorawanWakeUplink = 1;

    /// <summary>One from a device the relay tells its network about.</summary>
    public const byte LorawanWakeNotified = 2;

    /// <summary>The relay's own device read the frame.</summary>
    public const byte LorawanRelayHeardDevice = 0;

    /// <summary>The frame carried a downlink to pass on to an end device.</summary>
    public const byte LorawanRelayHeardDownlink = 1;

    /// <summary>It carried one that cannot be passed on.</summary>
    public const byte LorawanRelayHeardUndeliverable = 2;

    /// <summary>Turns an end device's relay mode on or off.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_use_relay(
        IntPtr device,
        byte on,
        out byte outTaken);

    /// <summary>Reports how a device uses a relay.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_relay_mode(
        IntPtr device,
        out byte outRelaying,
        out byte outActivation,
        out byte outSync,
        out uint outWorCounter);

    /// <summary>Returns what the relay's last acknowledgment said about itself.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_relay_status(
        IntPtr device,
        out PamojaLorawanRelayStatus outStatus);

    /// <summary>Reads the acknowledgment a relay answered the last WOR frame with.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_heard_wor_ack(
        IntPtr device,
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        out PamojaLorawanRelayStatus outStatus);

    /// <summary>Says what to do once the acknowledgment window closed empty.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_no_wor_ack(
        IntPtr device,
        ulong nowUs,
        out byte outNext,
        out PamojaLorawanRelayExchange outExchange);

    /// <summary>Makes a relay whose own device is activated by personalization.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_personalized(
        IntPtr plan,
        IntPtr session,
        in PamojaLorawanDeviceSettings settings,
        byte xtalAccuracy,
        byte cadToRx,
        out IntPtr outRelay);

    /// <summary>Makes a relay whose own device joins over the air.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_new(
        IntPtr plan,
        IntPtr credentials,
        in PamojaLorawanDeviceSettings settings,
        byte xtalAccuracy,
        byte cadToRx,
        out IntPtr outRelay);

    /// <summary>Releases a relay handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_relay_free(IntPtr relay);

    /// <summary>Returns why the last relay call failed, or null.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_lorawan_relay_error(IntPtr relay);

    /// <summary>Starts scanning, or changes what a running relay scans.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_start(
        IntPtr relay,
        byte cadPeriodicity,
        byte defaultChannelIndex,
        byte hasSecondChannel,
        uint secondWorFrequencyHz,
        uint secondAckFrequencyHz,
        byte secondDataRate);

    /// <summary>Stops scanning.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_relay_stop(IntPtr relay);

    /// <summary>Reports whether the relay is scanning.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_lorawan_relay_running(IntPtr relay);

    /// <summary>Trusts an end device, as an <c>UpdateUplinkListReq</c> does.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_trust(
        IntPtr relay,
        byte index,
        uint devAddr,
        ReadOnlySpan<byte> rootWorSKey,
        uint nextWfcnt,
        byte reloadRate,
        byte bucketSize);

    /// <summary>Says when and where to scan next.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_next_scan(
        IntPtr relay,
        ulong nowUs,
        out PamojaLorawanScan outScan);

    /// <summary>Reads a wake-on-radio frame a scan heard.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_heard_wor(
        IntPtr relay,
        in PamojaLorawanScan scan,
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        short rssiDbm,
        sbyte snrDb,
        ulong endedUs,
        out PamojaLorawanWake outWake);

    /// <summary>Reads the uplink a wake-on-radio frame announced.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_heard_uplink(
        IntPtr relay,
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        short rssiDbm,
        sbyte snrDb,
        ulong endedUs,
        out ulong outDueUs);

    /// <summary>Clears the uplink a wake-on-radio frame announced.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_relay_uplink_missed(IntPtr relay);

    /// <summary>Reports when the forwarded uplink waiting to go out is due.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_lorawan_relay_forward_due(IntPtr relay, out ulong outDueUs);

    /// <summary>Sends the uplink the relay is holding, in one of its own.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_forward(
        IntPtr relay,
        ulong nowUs,
        out IntPtr outFrame,
        out PamojaLorawanTransmission outTransmission);

    /// <summary>Reads a frame the relay's own device heard.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_heard_in(
        IntPtr relay,
        byte window,
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        sbyte snrDb,
        out byte outKind,
        out PamojaLorawanHeard outHeard,
        out IntPtr outPayload,
        out IntPtr outFrame,
        out PamojaLorawanRxr outDownlink);

    /// <summary>Says what comes next once the relay's own windows closed empty.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_nothing_heard(
        IntPtr relay,
        ulong nowUs,
        out byte outNext,
        out ulong outNotBeforeUs);

    /// <summary>Makes the relay's own join request.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_join(
        IntPtr relay,
        ushort devNonce,
        ulong nowUs,
        out IntPtr outFrame,
        out PamojaLorawanTransmission outTransmission);

    /// <summary>Sends one of the relay's own uplinks.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_send(
        IntPtr relay,
        byte port,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        byte confirmed,
        ulong nowUs,
        out IntPtr outFrame,
        out PamojaLorawanTransmission outTransmission);

    /// <summary>Reads where the relay's own device stands.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_relay_status(
        IntPtr relay,
        out PamojaLorawanEndDeviceStatus outStatus);
}

/// <summary>A wake-on-radio acknowledgment carried inline, seven bytes.</summary>
[InlineArray(Length)]
public struct PamojaWorAck
{
    /// <summary>The width of an acknowledgment, in bytes.</summary>
    public const int Length = 7;

    private byte _element0;

    /// <summary>Copies the frame out as an array.</summary>
    /// <returns>The seven bytes.</returns>
    public readonly byte[] ToArray()
    {
        PamojaWorAck copy = this;
        return ((ReadOnlySpan<byte>)copy).ToArray();
    }
}

/// <summary>A wake-on-radio frame carried inline, at most fifteen bytes.</summary>
[InlineArray(Length)]
public struct PamojaWorFrame
{
    /// <summary>The width of the longest wake-on-radio frame, in bytes.</summary>
    public const int Length = 15;

    private byte _element0;

    /// <summary>Copies the frame out as an array.</summary>
    /// <param name="len">How many of the bytes the frame uses.</param>
    /// <returns>The frame.</returns>
    public readonly byte[] ToArray(int len)
    {
        PamojaWorFrame copy = this;
        return ((ReadOnlySpan<byte>)copy)[..len].ToArray();
    }
}

/// <summary>A scan for wake-on-radio frames, mirroring <c>PamojaLorawanScan</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanScan
{
    /// <summary>When to start detecting, in microseconds.</summary>
    public ulong StartUs;

    /// <summary>The frequency to listen on, in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>The data rate to listen at.</summary>
    public byte DataRate;

    /// <summary><c>0</c> for the default channel, <c>1</c> for the second.</summary>
    public byte Channel;

    /// <summary>The LoRa settings of a wake-on-radio frame.</summary>
    public PamojaLoraLink Link;

    /// <summary>The longest preamble an end device sends on the channel, in symbols.</summary>
    public ushort PreambleSymbols;
}

/// <summary>What a wake-on-radio frame led to, mirroring <c>PamojaLorawanWake</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanWake
{
    /// <summary>When to start sending the acknowledgment, in microseconds.</summary>
    public ulong AckStartUs;

    /// <summary>How long it holds the air, in microseconds.</summary>
    public ulong AckAirtimeUs;

    /// <summary>When the uplink starts arriving, in microseconds.</summary>
    public ulong ListenStartUs;

    /// <summary>Which of the wake kinds this is.</summary>
    public byte Kind;

    /// <summary>The device the frame came from.</summary>
    public uint DevAddr;

    /// <summary>The wake-on-radio frame counter it carried.</summary>
    public uint Wfcnt;

    /// <summary>Whether the relay forwards the uplink, as table 16 codes it.</summary>
    public byte Forward;

    /// <summary><c>1</c> when there is an acknowledgment to send.</summary>
    public byte HasAck;

    /// <summary>The acknowledgment, seven bytes.</summary>
    public PamojaWorAck AckFrame;

    /// <summary>Where it goes, in hertz.</summary>
    public uint AckFrequencyHz;

    /// <summary>The data rate it goes out at.</summary>
    public byte AckDataRate;

    /// <summary>Its LoRa settings.</summary>
    public PamojaLoraLink AckLink;

    /// <summary>The power to ask of the radio for it, conducted, in dBm.</summary>
    public sbyte AckOutputDbm;

    /// <summary><c>1</c> when the relay listens for the uplink behind the frame.</summary>
    public byte HasListen;

    /// <summary>Where the uplink arrives, in hertz.</summary>
    public uint ListenFrequencyHz;

    /// <summary>The data rate it arrives at.</summary>
    public byte ListenDataRate;

    /// <summary>Its LoRa settings.</summary>
    public PamojaLoraLink ListenLink;

    /// <summary>The longest frame to receive.</summary>
    public nuint ListenMaxLen;
}

/// <summary>A downlink for an end device, mirroring <c>PamojaLorawanRxr</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanRxr
{
    /// <summary>When to start sending it, in microseconds.</summary>
    public ulong StartUs;

    /// <summary>How long it holds the air, in microseconds.</summary>
    public ulong AirtimeUs;

    /// <summary>The frequency, in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>The data rate.</summary>
    public byte DataRate;

    /// <summary>The LoRa settings.</summary>
    public PamojaLoraLink Link;

    /// <summary>The power to ask of the radio, conducted, in dBm.</summary>
    public sbyte OutputDbm;
}

/// <summary>What a relay said about itself, mirroring <c>PamojaLorawanRelayStatus</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanRelayStatus
{
    /// <summary>The coded scan periodicity.</summary>
    public byte CadPeriodicity;

    /// <summary>The coded crystal accuracy.</summary>
    public byte XtalAccuracy;

    /// <summary>The coded time to start receiving.</summary>
    public byte CadToRx;

    /// <summary>The data rate it forwards at.</summary>
    public byte RelayDataRate;

    /// <summary>The coded forwarding state.</summary>
    public byte Forward;
}

/// <summary>A wake-on-radio exchange, mirroring <c>PamojaLorawanRelayExchange</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanRelayExchange
{
    /// <summary>When to start sending the frame that wakes the relay, in microseconds.</summary>
    public ulong WakeUpStartUs;

    /// <summary>How long that frame holds the air, in microseconds.</summary>
    public ulong WakeUpAirtimeUs;

    /// <summary>When the acknowledgment would start arriving, in microseconds.</summary>
    public ulong AckStartUs;

    /// <summary>How long it would last, in microseconds.</summary>
    public ulong AckAirtimeUs;

    /// <summary>When the uplink itself goes out, in microseconds.</summary>
    public ulong UplinkStartUs;

    /// <summary>The frame that wakes the relay.</summary>
    public PamojaWorFrame WakeUpFrame;

    /// <summary>How many of those bytes to send.</summary>
    public byte WakeUpLen;

    /// <summary>Where the frame goes, in hertz.</summary>
    public uint WakeUpFrequencyHz;

    /// <summary>The data rate it goes out at.</summary>
    public byte WakeUpDataRate;

    /// <summary>Its LoRa settings, with the preamble this frame needs.</summary>
    public PamojaLoraLink WakeUpLink;

    /// <summary>The power to ask of the radio for it, conducted, in dBm.</summary>
    public sbyte WakeUpOutputDbm;

    /// <summary><c>1</c> when an acknowledgment is expected at all.</summary>
    public byte HasAck;

    /// <summary>Where the acknowledgment would arrive, in hertz.</summary>
    public uint AckFrequencyHz;

    /// <summary>The data rate it would arrive at.</summary>
    public byte AckDataRate;

    /// <summary>Its LoRa settings.</summary>
    public PamojaLoraLink AckLink;

    /// <summary>The relay window, timed from the end of the uplink.</summary>
    public PamojaLorawanWindow Rxr;
}
