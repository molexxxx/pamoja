using System.Runtime.InteropServices;

using Pamoja.Lora;
using Pamoja.Native.Interop;

namespace Pamoja.Lorawan;

/// <summary>What a device's radio can do, and how it takes part.</summary>
/// <remarks>
/// Only the output power range is required. The rest start as a typical node: TS001-1.0.4,
/// adaptive data rate on, an antenna with no gain over its cable, a radio that tunes 137 to
/// 1020 MHz as an SX1276 does, the region's duty cycle kept, and no repeater in the path.
/// </remarks>
/// <param name="MinOutputDbm">The lowest power the radio puts out, conducted, in dBm.</param>
/// <param name="MaxOutputDbm">The highest, conducted, in dBm.</param>
public sealed record LorawanDeviceSettings(sbyte MinOutputDbm, sbyte MaxOutputDbm)
{
    /// <summary>The link layer revision the network was told the device follows.</summary>
    public LorawanVersion Version { get; init; } = LorawanVersion.V1_0_4;

    /// <summary>Whether the network manages the data rate and power.</summary>
    public bool Adr { get; init; } = true;

    /// <summary>The antenna gain less the cable and connector losses, in dB.</summary>
    public sbyte AntennaGainDb { get; init; }

    /// <summary>The lowest frequency the radio and its front end can use, in hertz.</summary>
    public uint LowestHz { get; init; } = 137_000_000;

    /// <summary>The highest, in hertz.</summary>
    public uint HighestHz { get; init; } = 1_020_000_000;

    /// <summary>Whether to hold the device to the region's sub-band duty cycles.</summary>
    public bool RegionalDutyCycle { get; init; } = true;

    /// <summary>Whether to size payloads for a path through a relay.</summary>
    public bool BehindRepeater { get; init; }

    /// <summary>
    /// A seed for the random choices of channel and retry delay, ideally from a hardware
    /// random source. The device identifier is mixed in.
    /// </summary>
    public uint Seed { get; init; }

    /// <summary>The settings as the C ABI takes them.</summary>
    internal PamojaLorawanDeviceSettings Native => new()
    {
        LowestHz = LowestHz,
        HighestHz = HighestHz,
        Seed = Seed,
        Version = (byte)Version,
        Adr = (byte)(Adr ? 1 : 0),
        MinOutputDbm = MinOutputDbm,
        MaxOutputDbm = MaxOutputDbm,
        AntennaGainDb = AntennaGainDb,
        RegionalDutyCycle = (byte)(RegionalDutyCycle ? 1 : 0),
        BehindRepeater = (byte)(BehindRepeater ? 1 : 0),
    };
}

/// <summary>When and where to listen for a downlink.</summary>
/// <param name="DelayMicros">How long after the end of the transmission the window opens.</param>
/// <param name="FrequencyHz">The carrier, in hertz.</param>
/// <param name="DataRate">The downlink data rate, as the region numbers them.</param>
/// <param name="Link">
/// The LoRa settings to listen with: no payload CRC, and inverted IQ, as RP002-1.0.5 table 112
/// has for a downlink.
/// </param>
public sealed record LorawanWindow(uint DelayMicros, uint FrequencyHz, byte DataRate, LoraLink Link);

/// <summary>A frame to put on the air, and where to listen afterward.</summary>
/// <param name="Frame">The frame.</param>
/// <param name="FrequencyHz">The carrier, in hertz.</param>
/// <param name="DataRate">The data rate, as the region numbers them.</param>
/// <param name="Link">
/// The LoRa settings: an eight-symbol preamble, an explicit header and a payload CRC, sent with
/// standard IQ.
/// </param>
/// <param name="OutputDbm">The power to ask of the radio, conducted, in dBm.</param>
/// <param name="AirtimeMicros">How long the frame holds the air.</param>
/// <param name="Rx1">The first receive window.</param>
/// <param name="Rx2">The second, which opens only if nothing for this device arrived in the first.</param>
/// <param name="CarriesPayload">
/// Whether the application payload went out in this frame. When the answers the device owed
/// left no room, it did not, and has to be sent again.
/// </param>
/// <param name="Relay">
/// The wake-on-radio exchange this frame goes out behind, for a device under a relay, and
/// <c>null</c> for a frame that goes straight to a gateway.
/// </param>
public sealed record LorawanTransmission(
    byte[] Frame,
    uint FrequencyHz,
    byte DataRate,
    LoraLink Link,
    sbyte OutputDbm,
    ulong AirtimeMicros,
    LorawanWindow Rx1,
    LorawanWindow Rx2,
    bool CarriesPayload,
    LorawanRelayExchange? Relay = null);

/// <summary>How well the network heard a link check.</summary>
/// <param name="MarginDb">How far above the demodulation floor the best gateway heard it.</param>
/// <param name="Gateways">How many gateways heard it.</param>
public readonly record struct LorawanLinkCheck(byte MarginDb, byte Gateways);

/// <summary>The time a network gave a device.</summary>
/// <param name="GpsSeconds">
/// Whole seconds since the GPS epoch, 1980-01-06 00:00:00 UTC, at the end of the uplink that asked.
/// </param>
/// <param name="Fraction">The fraction of a second, in 256ths.</param>
public readonly record struct LorawanDeviceTime(uint GpsSeconds, byte Fraction);

/// <summary>A downlink, read and acted on.</summary>
/// <param name="Port">
/// The application port, or <c>null</c> for a frame that carried only MAC commands or nothing.
/// </param>
/// <param name="Payload">The application payload, decrypted.</param>
/// <param name="Acknowledged">Whether the network acknowledged the confirmed uplink this answered.</param>
/// <param name="Confirmed">
/// Whether the network asked for this downlink to be acknowledged, which the next uplink does.
/// </param>
/// <param name="MorePending">Whether the network has more waiting.</param>
/// <param name="LinkCheck">The answer to a link check the device asked for.</param>
/// <param name="DeviceTime">The answer to a time request the device asked for.</param>
public sealed record LorawanDelivery(
    byte? Port,
    byte[] Payload,
    bool Acknowledged,
    bool Confirmed,
    bool MorePending,
    LorawanLinkCheck? LinkCheck,
    LorawanDeviceTime? DeviceTime);

/// <summary>What a frame heard in a receive window turned out to be.</summary>
public abstract record LorawanHeard
{
    private LorawanHeard(uint devAddr) => DevAddr = devAddr;

    /// <summary>The address the device is on the network by.</summary>
    public uint DevAddr { get; }

    /// <summary>A join accept: the device is on the network.</summary>
    /// <param name="DevAddr">The address the network gave it.</param>
    public sealed record Joined(uint DevAddr) : LorawanHeard(DevAddr);

    /// <summary>A data frame for this device.</summary>
    /// <param name="DevAddr">The address the device is on the network by.</param>
    /// <param name="Delivery">What the frame carried.</param>
    public sealed record Data(uint DevAddr, LorawanDelivery Delivery) : LorawanHeard(DevAddr);
}

/// <summary>Which receive window a frame arrived in.</summary>
public enum LorawanReceiveWindow
{
    /// <summary>The first window, on the uplink's downlink channel.</summary>
    Rx1 = 1,

    /// <summary>The second, on the fixed frequency and data rate.</summary>
    Rx2 = 2,

    /// <summary>The relay window, which a device under a relay opens last, TS011-1.0.1 chapter 7.</summary>
    Rxr = 3,
}

/// <summary>What to do once both receive windows closed with nothing for the device.</summary>
public enum LorawanNextKind
{
    /// <summary>Send the same frame again with <see cref="LorawanEndDevice.Repeat"/>.</summary>
    Repeat = 0,

    /// <summary>The uplink is finished.</summary>
    Done = 1,

    /// <summary>A confirmed uplink went out every time it may without an acknowledgment.</summary>
    Unacknowledged = 2,

    /// <summary>The join got no answer; join again with a new nonce.</summary>
    JoinAgain = 3,
}

/// <summary>What to do once both receive windows closed with nothing for the device.</summary>
/// <param name="Kind">What to do.</param>
/// <param name="NotBeforeMicros">For a repeat or another join, the earliest time to send.</param>
public readonly record struct LorawanNext(LorawanNextKind Kind, ulong? NotBeforeMicros);

/// <summary>A channel a device may send on.</summary>
/// <param name="Index">The channel's index in the device's table.</param>
/// <param name="UplinkHz">Where uplinks go out, in hertz.</param>
/// <param name="DownlinkHz">Where the first receive window listens, in hertz.</param>
/// <param name="MinDataRate">The slowest data rate the channel carries.</param>
/// <param name="MaxDataRate">The fastest.</param>
public readonly record struct LorawanChannel(
    int Index,
    uint UplinkHz,
    uint DownlinkHz,
    byte MinDataRate,
    byte MaxDataRate);

/// <summary>How a device reports its battery when a network asks.</summary>
/// <param name="Code">The byte <c>DevStatusAns</c> carries: 0 external, 1 to 254 a level, 255 unknown.</param>
public readonly record struct LorawanBattery(byte Code)
{
    /// <summary>Running from an external supply.</summary>
    public static LorawanBattery External => new(0);

    /// <summary>The device cannot measure it.</summary>
    public static LorawanBattery Unknown => new(255);

    /// <summary>A level from 1, empty, to 254, full.</summary>
    /// <param name="level">The level, clamped into that range.</param>
    /// <returns>The battery.</returns>
    public static LorawanBattery Level(byte level) => new(Math.Clamp(level, (byte)1, (byte)254));
}

/// <summary>Why an end device could not do what it was asked.</summary>
public enum LorawanDeviceErrorKind
{
    /// <summary>The plan defines more channels than a device keeps.</summary>
    TooManyChannels = 1,

    /// <summary>A device activated by personalization has nothing to join with.</summary>
    NoCredentials = 2,

    /// <summary>The device has not joined.</summary>
    NotJoined = 3,

    /// <summary>A transmission is still waiting on its receive windows.</summary>
    Busy = 4,

    /// <summary>There is no transmission waiting on its windows or due to repeat.</summary>
    NothingPending = 5,

    /// <summary>The air is not free until <see cref="LorawanDeviceException.UntilMicros"/>.</summary>
    Wait = 6,

    /// <summary>No enabled channel carries the data rate.</summary>
    NoChannel = 7,

    /// <summary>The data rate is not a LoRa one the device can use.</summary>
    DataRate = 8,

    /// <summary>The payload does not fit; <see cref="LorawanDeviceException.Max"/> bytes do.</summary>
    PayloadTooLong = 9,

    /// <summary>The uplink frame counter is spent, and the device has to join again.</summary>
    CounterExhausted = 10,

    /// <summary>The frame did not decode, or port 0 was asked for an application payload.</summary>
    Frame = 11,

    /// <summary>The frame is addressed to another device.</summary>
    Foreign = 12,

    /// <summary>The frame repeats or precedes the last downlink the device accepted.</summary>
    Replayed = 13,

    /// <summary>The frame counter jumped further ahead than the device follows.</summary>
    CounterGap = 14,

    /// <summary>A join accept carries settings the region does not allow.</summary>
    Refused = 15,

    /// <summary>A saved state was not resumed; <see cref="LorawanDeviceException.State"/> says why.</summary>
    State = 16,
}

/// <summary>Why a saved device state was not resumed.</summary>
public enum LorawanStateErrorKind
{
    /// <summary>The bytes are not the length a saved state takes.</summary>
    Length = 1,

    /// <summary>The bytes are corrupt.</summary>
    Corrupt = 2,

    /// <summary>The state was saved in a format this build does not read.</summary>
    Format = 3,

    /// <summary>The state was saved on another channel plan.</summary>
    Plan = 4,
}

/// <summary>Thrown when an end device cannot do what it was asked.</summary>
public sealed class LorawanDeviceException : PamojaException
{
    /// <summary>Creates the exception from what the native device reported.</summary>
    /// <param name="message">The reason, in words.</param>
    /// <param name="error">The reason, as the C ABI describes it.</param>
    internal LorawanDeviceException(string message, PamojaLorawanDeviceError error)
        : base(message)
    {
        Kind = (LorawanDeviceErrorKind)error.Kind;
        UntilMicros = Kind == LorawanDeviceErrorKind.Wait ? error.UntilUs : null;
        Max = Kind is LorawanDeviceErrorKind.PayloadTooLong or LorawanDeviceErrorKind.TooManyChannels
            ? error.Max
            : null;
        DataRate = Kind == LorawanDeviceErrorKind.DataRate ? error.DataRate : null;
        State = Kind == LorawanDeviceErrorKind.State ? (LorawanStateErrorKind)error.State : null;
        Format = State == LorawanStateErrorKind.Format ? error.Format : null;
    }

    /// <summary>Why the call failed.</summary>
    public LorawanDeviceErrorKind Kind { get; }

    /// <summary>For <see cref="LorawanDeviceErrorKind.Wait"/>, the earliest time to try again.</summary>
    public ulong? UntilMicros { get; }

    /// <summary>
    /// For <see cref="LorawanDeviceErrorKind.PayloadTooLong"/>, the most the frame carries; for
    /// <see cref="LorawanDeviceErrorKind.TooManyChannels"/>, the most channels a device keeps.
    /// </summary>
    public uint? Max { get; }

    /// <summary>For <see cref="LorawanDeviceErrorKind.DataRate"/>, the data rate.</summary>
    public byte? DataRate { get; }

    /// <summary>For <see cref="LorawanDeviceErrorKind.State"/>, why the state was not resumed.</summary>
    public LorawanStateErrorKind? State { get; }

    /// <summary>For <see cref="LorawanStateErrorKind.Format"/>, the format the state was saved in.</summary>
    public byte? Format { get; }
}

/// <summary>A LoRaWAN Class A end device, without a radio.</summary>
/// <remarks>
/// It joins, chooses a channel and data rate for each uplink, says when and where to listen for
/// the answer, reads what comes back, and does what the network's MAC commands ask. It owns no
/// radio and no clock: every call takes the time in microseconds and hands back what to put on
/// the air, so the same device runs over any radio, or in a test with none.
/// <para>
/// One exchange runs like this: <see cref="Join"/> or <see cref="Send"/> returns a
/// transmission and two receive windows timed from its end. A frame heard in either window goes
/// to <see cref="Heard"/>. If neither held one, <see cref="NothingHeard"/> says whether to
/// <see cref="Repeat"/> the frame, or move on.
/// </para>
/// </remarks>
public sealed class LorawanEndDevice : IDisposable
{
    private readonly NativeHandle _handle;

    private LorawanEndDevice(IntPtr device) =>
        _handle = new NativeHandle(device, NativeMethods.pamoja_lorawan_end_device_free);

    /// <summary>Makes a device that joins over the air.</summary>
    /// <param name="plan">A published channel plan, from <see cref="LoraChannelPlan.ForRegion"/> or <see cref="LoraChannelPlan.ForCn470"/>.</param>
    /// <param name="credentials">The device's identifiers and root key, which the device copies.</param>
    /// <param name="settings">What its radio can do.</param>
    /// <param name="fcntUp">The next uplink frame counter carried over a restart; a join starts it over.</param>
    /// <param name="fcntDown">The last downlink frame counter accepted, if any was.</param>
    /// <returns>The device, not yet joined.</returns>
    /// <exception cref="PamojaException">
    /// The plan was built rather than published, or the settings run backward.
    /// </exception>
    public static LorawanEndDevice OverTheAir(
        LoraChannelPlan plan,
        LorawanDevice credentials,
        LorawanDeviceSettings settings,
        uint fcntUp = 0,
        uint? fcntDown = null)
    {
        PamojaLorawanDeviceSettings native = settings.Native;
        using NativeLease held = plan.Lease();
        IntPtr planPointer = held.Pointer;
        IntPtr device = credentials.UseHandle(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_end_device_new(
                planPointer,
                handle,
                in native,
                fcntUp,
                (byte)(fcntDown.HasValue ? 1 : 0),
                fcntDown ?? 0,
                out IntPtr made));
            return made;
        });
        return new LorawanEndDevice(device);
    }

    /// <summary>Makes a device activated by personalization, with its session provisioned.</summary>
    /// <param name="plan">A published channel plan.</param>
    /// <param name="session">The address and session keys it was provisioned with, which the device copies.</param>
    /// <param name="settings">What its radio can do.</param>
    /// <param name="fcntUp">The next uplink frame counter.</param>
    /// <param name="fcntDown">The last downlink frame counter accepted, if any was.</param>
    /// <returns>The device, ready to send at its slowest data rate.</returns>
    /// <exception cref="PamojaException">The plan was built rather than published.</exception>
    /// <remarks>
    /// Such a device never resets its frame counters, TS001-1.0.4 section 4.3.1.5, so one that
    /// lost power passes the counters it kept.
    /// </remarks>
    public static LorawanEndDevice Personalized(
        LoraChannelPlan plan,
        LorawanSession session,
        LorawanDeviceSettings settings,
        uint fcntUp = 0,
        uint? fcntDown = null)
    {
        PamojaLorawanDeviceSettings native = settings.Native;
        using NativeLease held = plan.Lease();
        IntPtr planPointer = held.Pointer;
        IntPtr device = session.UseHandle(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_end_device_personalized(
                planPointer,
                handle,
                in native,
                fcntUp,
                (byte)(fcntDown.HasValue ? 1 : 0),
                fcntDown ?? 0,
                out IntPtr made));
            return made;
        });
        return new LorawanEndDevice(device);
    }

    /// <summary>Whether the device is on a network: once joined, or from the start when personalized.</summary>
    public bool IsJoined => ReadStatus().Joined != 0;

    /// <summary>The address the device is on the network by, or <c>null</c> before joining.</summary>
    public uint? DevAddr
    {
        get
        {
            PamojaLorawanEndDeviceStatus status = ReadStatus();
            return status.Joined != 0 ? status.DevAddr : null;
        }
    }

    /// <summary>The data rate the next uplink goes out at, before any back-off step.</summary>
    public byte DataRate => ReadStatus().DataRate;

    /// <summary>The next uplink frame counter, which a device stores to carry over a restart.</summary>
    public uint FcntUp => ReadStatus().FcntUp;

    /// <summary>The last downlink frame counter accepted, or <c>null</c> before any downlink.</summary>
    public uint? FcntDown
    {
        get
        {
            PamojaLorawanEndDeviceStatus status = ReadStatus();
            return status.HasFcntDown != 0 ? status.FcntDown : null;
        }
    }

    /// <summary>How many times each uplink goes out, as the network last set it.</summary>
    public byte Transmissions => ReadStatus().Transmissions;

    /// <summary>Where the second receive window listens.</summary>
    public LoraRx2 Rx2
    {
        get
        {
            PamojaLorawanEndDeviceStatus status = ReadStatus();
            return new LoraRx2(status.Rx2FrequencyHz, status.Rx2DataRate);
        }
    }

    /// <summary>The delay from the end of an uplink to the first receive window, in microseconds.</summary>
    public uint ReceiveDelayMicros => ReadStatus().ReceiveDelayUs;

    /// <summary>The lowest and highest frequency the device transmits or listens on, in hertz.</summary>
    public (uint LowestHz, uint HighestHz) FrequencySpan
    {
        get
        {
            PamojaLorawanEndDeviceStatus status = ReadStatus();
            return (status.LowestHz, status.HighestHz);
        }
    }

    /// <summary>Builds a join request.</summary>
    /// <param name="devNonce">A nonce this device has never used with its join EUI.</param>
    /// <param name="nowMicros">The time, in microseconds.</param>
    /// <returns>What to transmit, with the join accept windows.</returns>
    /// <exception cref="LorawanDeviceException">No credentials, busy, or a wait for the air.</exception>
    public LorawanTransmission Join(ushort devNonce, ulong nowMicros) =>
        _handle.Use(handle => Taken(
            handle,
            NativeMethods.pamoja_lorawan_end_device_join(
                handle, devNonce, nowMicros, out IntPtr frame, out PamojaLorawanTransmission transmission),
            frame,
            transmission));

    /// <summary>Builds an uplink carrying a payload.</summary>
    /// <param name="port">The application port, 1 to 223, or 224 for the certification test port.</param>
    /// <param name="payload">The application payload.</param>
    /// <param name="nowMicros">The time, in microseconds.</param>
    /// <param name="confirmed">Whether to ask the network to acknowledge it.</param>
    /// <returns>What to transmit, with its receive windows.</returns>
    /// <exception cref="LorawanDeviceException">
    /// Not joined, busy, a wait for the air, a payload too long, or port 0.
    /// </exception>
    public LorawanTransmission Send(byte port, ReadOnlySpan<byte> payload, ulong nowMicros, bool confirmed = false)
    {
        byte[] body = payload.ToArray();
        return _handle.Use(handle => Taken(
            handle,
            NativeMethods.pamoja_lorawan_end_device_send(
                handle,
                port,
                body,
                (nuint)body.Length,
                (byte)(confirmed ? 1 : 0),
                nowMicros,
                out IntPtr frame,
                out PamojaLorawanTransmission transmission),
            frame,
            transmission));
    }

    /// <summary>
    /// Builds an uplink with no payload, carrying the answers the device owes, an acknowledgment,
    /// or an ADR acknowledgment request.
    /// </summary>
    /// <param name="nowMicros">The time, in microseconds.</param>
    /// <returns>What to transmit, with its receive windows.</returns>
    /// <exception cref="LorawanDeviceException">As <see cref="Send"/>.</exception>
    public LorawanTransmission SendEmpty(ulong nowMicros) =>
        _handle.Use(handle => Taken(
            handle,
            NativeMethods.pamoja_lorawan_end_device_send_empty(
                handle, nowMicros, out IntPtr frame, out PamojaLorawanTransmission transmission),
            frame,
            transmission));

    /// <summary>Sends the last uplink again, the same frame on a channel chosen afresh.</summary>
    /// <param name="nowMicros">The time, in microseconds.</param>
    /// <returns>What to transmit, with its receive windows.</returns>
    /// <exception cref="LorawanDeviceException">Nothing due to repeat, or a wait.</exception>
    public LorawanTransmission Repeat(ulong nowMicros) =>
        _handle.Use(handle => Taken(
            handle,
            NativeMethods.pamoja_lorawan_end_device_repeat(
                handle, nowMicros, out IntPtr frame, out PamojaLorawanTransmission transmission),
            frame,
            transmission));

    /// <summary>Reads a frame heard in one of the receive windows of the last transmission.</summary>
    /// <param name="frame">The bytes the radio received.</param>
    /// <param name="snrDb">The frame's signal-to-noise ratio, which a <c>DevStatusAns</c> reports.</param>
    /// <param name="window">
    /// The window the radio heard it in, when known. TS001-1.0.4 section 4.1 has a device discard a
    /// frame whose MACPayload is longer than the data rate it was received at carries, so a named
    /// window holds the frame to its own limit; without one, the frame may be as long as the faster
    /// window allows.
    /// </param>
    /// <returns>The join, or the downlink read and acted on.</returns>
    /// <exception cref="LorawanDeviceException">
    /// Nothing pending, another device's frame, a replayed or far-ahead counter, a refused join
    /// accept, or a frame that did not decode or is longer than the window carries. The windows
    /// stay open, so the second still listens.
    /// </exception>
    /// <exception cref="ArgumentOutOfRangeException">The window is not one of the two.</exception>
    public LorawanHeard Heard(ReadOnlySpan<byte> frame, sbyte snrDb, LorawanReceiveWindow? window = null)
    {
        if (window is { } named && !Enum.IsDefined(named))
        {
            throw new ArgumentOutOfRangeException(nameof(window), named, "not a receive window");
        }

        byte[] bytes = frame.ToArray();
        return _handle.Use<LorawanHeard>(handle =>
        {
            PamojaLorawanHeard heard;
            IntPtr payload;
            PamojaStatus status = window is { } known
                ? NativeMethods.pamoja_lorawan_end_device_heard_in(
                    handle, (byte)known, bytes, (nuint)bytes.Length, snrDb, out heard, out payload)
                : NativeMethods.pamoja_lorawan_end_device_heard(
                    handle, bytes, (nuint)bytes.Length, snrDb, out heard, out payload);
            ThrowIfFailed(handle, status);
            return HeardOf(heard, payload);
        });
    }

    /// <summary>Says what comes next once both receive windows closed with nothing for the device.</summary>
    /// <param name="nowMicros">The time the second window closed, in microseconds.</param>
    /// <returns>Whether to repeat the frame, join again, or move on.</returns>
    /// <exception cref="LorawanDeviceException">No transmission waits on its windows.</exception>
    public LorawanNext NothingHeard(ulong nowMicros) =>
        _handle.Use(handle =>
        {
            ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_end_device_nothing_heard(
                handle, nowMicros, out PamojaLorawanNext next));
            LorawanNextKind kind = (LorawanNextKind)next.Kind;
            return new LorawanNext(
                kind,
                kind is LorawanNextKind.Repeat or LorawanNextKind.JoinAgain ? next.NotBeforeUs : null);
        });

    /// <summary>Sets what the device reports its battery as, when a network asks.</summary>
    /// <param name="battery">The battery.</param>
    public void SetBattery(LorawanBattery battery)
    {
        (byte kind, byte level) = battery.Code switch
        {
            0 => ((byte)0, (byte)0),
            255 => ((byte)2, (byte)0),
            byte code => ((byte)1, code),
        };
        _handle.Use(handle => Status.ThrowIfError(
            NativeMethods.pamoja_lorawan_end_device_set_battery(handle, kind, level)));
    }

    /// <summary>Asks the network, with the next uplink, how well it hears the device.</summary>
    public void RequestLinkCheck() =>
        _handle.Use(handle => Status.ThrowIfError(
            NativeMethods.pamoja_lorawan_end_device_request_link_check(handle)));

    /// <summary>Asks the network, with the next uplink, for the time.</summary>
    public void RequestDeviceTime() =>
        _handle.Use(handle => Status.ThrowIfError(
            NativeMethods.pamoja_lorawan_end_device_request_device_time(handle)));

    /// <summary>Lists the channels the device may send on.</summary>
    /// <returns>Each enabled channel, with its index.</returns>
    public IReadOnlyList<LorawanChannel> Channels() =>
        _handle.Use<IReadOnlyList<LorawanChannel>>(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_end_device_status(handle, out PamojaLorawanEndDeviceStatus status));
            List<LorawanChannel> channels = new(status.ChannelCount);
            for (ushort position = 0; position < status.ChannelCount; position++)
            {
                Status.ThrowIfError(NativeMethods.pamoja_lorawan_end_device_channel(
                    handle, position, out ushort index, out PamojaLorawanChannel channel));
                channels.Add(new LorawanChannel(
                    index, channel.UplinkHz, channel.DownlinkHz, channel.MinDataRate, channel.MaxDataRate));
            }

            return channels;
        });

    /// <summary>Saves a joined device's state, to keep across a loss of power.</summary>
    /// <param name="nowMicros">The time, in microseconds.</param>
    /// <returns>The state, which holds the session keys: keep it wherever the keys would be safe.</returns>
    /// <exception cref="LorawanDeviceException">Not joined, or busy.</exception>
    public byte[] Save(ulong nowMicros) =>
        _handle.Use(handle =>
        {
            byte[] saved = new byte[NativeMethods.LorawanSavedLen];
            ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_end_device_save(
                handle, nowMicros, saved, (nuint)saved.Length));
            return saved;
        });

    /// <summary>Puts a saved state back on a device made the same way.</summary>
    /// <param name="saved">The state <see cref="Save"/> returned.</param>
    /// <param name="nowMicros">The time on the clock the device woke to, in microseconds.</param>
    /// <exception cref="LorawanDeviceException">
    /// A state of the wrong length, a corrupt one, one in another format, one from another plan,
    /// or a busy device.
    /// </exception>
    public void Resume(ReadOnlySpan<byte> saved, ulong nowMicros)
    {
        byte[] bytes = saved.ToArray();
        _handle.Use(handle => ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_end_device_resume(
            handle, bytes, (nuint)bytes.Length, nowMicros)));
    }


    /// <summary>Turns relay mode on or off, TS011-1.0.1 section 10.2 and appendix 5.</summary>
    /// <param name="on">Whether the next uplinks go through a relay.</param>
    /// <returns>
    /// <c>false</c> when the network holds the decision, leaving the mode as it was.
    /// </returns>
    /// <remarks>
    /// From here on the decision is the caller's rather than the device's own policy, until the
    /// network takes it over with an <c>EndDeviceConfReq</c> or hands it back.
    /// </remarks>
    public bool UseRelay(bool on) =>
        _handle.Use(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_end_device_use_relay(
                handle, (byte)(on ? 1 : 0), out byte taken));
            return taken != 0;
        });

    /// <summary>Whether the next uplink goes through a relay.</summary>
    public bool Relaying => ReadRelayMode().Relaying;

    /// <summary>How the device decides whether to use a relay.</summary>
    public LorawanRelayActivation RelayActivation => ReadRelayMode().Activation;

    /// <summary>What the device knows of when its relay listens, TS011-1.0.1 section 3.9.</summary>
    public LorawanRelaySync RelaySync => ReadRelayMode().Sync;

    /// <summary>The wake-on-radio frame counter the next frame will use, section 5.3.2.</summary>
    public uint WorCounter => ReadRelayMode().WorCounter;

    /// <summary>What the relay's last acknowledgment said, or <c>null</c> before one arrived.</summary>
    public LorawanRelayStatus? RelayStatus =>
        _handle.Use<LorawanRelayStatus?>(handle =>
            NativeMethods.pamoja_lorawan_end_device_relay_status(
                handle, out PamojaLorawanRelayStatus status) == PamojaStatus.Ok
                ? RelayStatusOf(status)
                : null);

    /// <summary>Reads the acknowledgment a relay answered the last wake-on-radio frame with.</summary>
    /// <param name="frame">The seven bytes the radio received in the acknowledgment window.</param>
    /// <returns>What the relay said about itself.</returns>
    /// <exception cref="LorawanDeviceException">
    /// No exchange is waiting, or the acknowledgment does not verify.
    /// </exception>
    /// <remarks>
    /// The device is now synchronized: it knows when the relay scans, so its next frames carry
    /// only as much preamble as the two clocks could have drifted apart.
    /// </remarks>
    public LorawanRelayStatus HeardWorAck(ReadOnlySpan<byte> frame)
    {
        byte[] bytes = frame.ToArray();
        return _handle.Use(handle =>
        {
            ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_end_device_heard_wor_ack(
                handle, bytes, (nuint)bytes.Length, out PamojaLorawanRelayStatus status));
            return RelayStatusOf(status);
        });
    }

    /// <summary>Says what to do once the acknowledgment window closed with nothing in it.</summary>
    /// <param name="nowMicros">The time the window closed, in microseconds.</param>
    /// <returns>
    /// Whether to send the uplink at the time the exchange named anyway, or wake the relay again
    /// first, as the network's back-off asks.
    /// </returns>
    /// <exception cref="LorawanDeviceException">No wake-on-radio frame waits on an answer.</exception>
    public LorawanWorNext NoWorAck(ulong nowMicros) =>
        _handle.Use(handle =>
        {
            ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_end_device_no_wor_ack(
                handle, nowMicros, out byte next, out PamojaLorawanRelayExchange exchange));
            return next == NativeMethods.LorawanWorNextUplink
                ? new LorawanWorNext(true, null)
                : new LorawanWorNext(false, ExchangeOf(exchange));
        });

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Reads how the device uses a relay.</summary>
    /// <returns>The mode, what it knows of the relay, and its frame counter.</returns>
    private (bool Relaying, LorawanRelayActivation Activation, LorawanRelaySync Sync, uint WorCounter) ReadRelayMode() =>
        _handle.Use(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_end_device_relay_mode(
                handle,
                out byte relaying,
                out byte activation,
                out byte sync,
                out uint worCounter));
            return (
                relaying != 0,
                (LorawanRelayActivation)activation,
                (LorawanRelaySync)sync,
                worCounter);
        });

    /// <summary>Rebuilds what a relay said about itself from the fields the C ABI reported.</summary>
    /// <param name="status">The status as the C ABI describes it.</param>
    /// <returns>The status.</returns>
    internal static LorawanRelayStatus RelayStatusOf(PamojaLorawanRelayStatus status) =>
        new(
            (LorawanCadPeriodicity)status.CadPeriodicity,
            (LorawanXtalAccuracy)status.XtalAccuracy,
            (LorawanCadToRx)status.CadToRx,
            status.RelayDataRate,
            (LorawanRelayForward)status.Forward);

    /// <summary>Rebuilds a wake-on-radio exchange from the fields the C ABI reported.</summary>
    /// <param name="exchange">The exchange as the C ABI describes it.</param>
    /// <returns>The exchange.</returns>
    internal static LorawanRelayExchange ExchangeOf(PamojaLorawanRelayExchange exchange) =>
        new(
            new LorawanWakeUp(
                exchange.WakeUpFrame.ToArray(exchange.WakeUpLen),
                exchange.WakeUpStartUs,
                new LorawanCarrier(exchange.WakeUpFrequencyHz, exchange.WakeUpDataRate),
                LinkOf(exchange.WakeUpLink),
                exchange.WakeUpOutputDbm,
                exchange.WakeUpAirtimeUs),
            exchange.HasAck != 0
                ? new LorawanAckWindow(
                    exchange.AckStartUs,
                    new LorawanCarrier(exchange.AckFrequencyHz, exchange.AckDataRate),
                    LinkOf(exchange.AckLink),
                    exchange.AckAirtimeUs)
                : null,
            exchange.UplinkStartUs,
            WindowOf(exchange.Rxr));

    /// <summary>Rebuilds a transmission from what a transmitting call handed back.</summary>
    /// <param name="frame">The frame buffer it set.</param>
    /// <param name="transmission">The transmission it described.</param>
    /// <returns>The transmission.</returns>
    internal static LorawanTransmission TransmissionOf(
        IntPtr frame,
        PamojaLorawanTransmission transmission) =>
        new(
            Pamoja.Codec.Codec.TakeBytes(frame),
            transmission.FrequencyHz,
            transmission.DataRate,
            LinkOf(transmission.Link),
            transmission.OutputDbm,
            transmission.AirtimeUs,
            WindowOf(transmission.Rx1),
            WindowOf(transmission.Rx2),
            transmission.CarriesPayload != 0,
            transmission.HasRelay != 0 ? ExchangeOf(transmission.Relay) : null);

    /// <summary>Rebuilds what a frame turned out to be from what the C ABI reported.</summary>
    /// <param name="heard">The frame as the C ABI describes it.</param>
    /// <param name="payload">The payload buffer it set.</param>
    /// <returns>The join, or the downlink read and acted on.</returns>
    internal static LorawanHeard HeardOf(PamojaLorawanHeard heard, IntPtr payload)
    {
        if (heard.Kind == 0)
        {
            return new LorawanHeard.Joined(heard.DevAddr);
        }

        return new LorawanHeard.Data(
            heard.DevAddr,
            new LorawanDelivery(
                heard.HasPort != 0 ? heard.Port : null,
                Pamoja.Codec.Codec.TakeBytes(payload),
                heard.Acknowledged != 0,
                heard.Confirmed != 0,
                heard.MorePending != 0,
                heard.HasLinkCheck != 0 ? new LorawanLinkCheck(heard.MarginDb, heard.Gateways) : null,
                heard.HasDeviceTime != 0 ? new LorawanDeviceTime(heard.GpsSeconds, heard.Fraction) : null));
    }

    /// <summary>Reads where the device stands.</summary>
    /// <returns>The status.</returns>
    private PamojaLorawanEndDeviceStatus ReadStatus() =>
        _handle.Use(handle =>
        {
            Status.ThrowIfError(
                NativeMethods.pamoja_lorawan_end_device_status(handle, out PamojaLorawanEndDeviceStatus status));
            return status;
        });

    /// <summary>Takes the frame of a transmitting call, or throws why it failed.</summary>
    /// <param name="handle">The live device handle.</param>
    /// <param name="status">What the call returned.</param>
    /// <param name="frame">The frame buffer it set.</param>
    /// <param name="transmission">The transmission it described.</param>
    /// <returns>The transmission.</returns>
    private static LorawanTransmission Taken(
        IntPtr handle,
        PamojaStatus status,
        IntPtr frame,
        PamojaLorawanTransmission transmission)
    {
        ThrowIfFailed(handle, status);
        return TransmissionOf(frame, transmission);
    }

    /// <summary>Throws the device's recorded reason when a call failed.</summary>
    /// <param name="handle">The live device handle.</param>
    /// <param name="status">What the call returned.</param>
    private static void ThrowIfFailed(IntPtr handle, PamojaStatus status)
    {
        if (status == PamojaStatus.Ok)
        {
            return;
        }

        string message = Status.LastError() ?? status.ToString();
        if (NativeMethods.pamoja_lorawan_end_device_error(handle, out PamojaLorawanDeviceError error) == PamojaStatus.Ok
            && error.Kind != 0)
        {
            throw new LorawanDeviceException(message, error);
        }

        Status.ThrowIfError(status);
    }

    /// <summary>Rebuilds a link from the fields the C ABI reported.</summary>
    /// <param name="link">The link as the C ABI describes it.</param>
    /// <returns>The link settings.</returns>
    internal static LoraLink LinkOf(PamojaLoraLink link)
    {
        LoraLink built = new LoraLink(link.SpreadingFactor, link.BandwidthHz)
            .WithCodingRate(link.CodingRateDenominator)
            .WithPreamble(link.PreambleSymbols);
        if (link.ExplicitHeader == 0)
        {
            built = built.WithImplicitHeader();
        }

        return link.Crc == 0 ? built.WithoutCrc() : built;
    }

    /// <summary>Rebuilds a receive window from the fields the C ABI reported.</summary>
    /// <param name="window">The window as the C ABI describes it.</param>
    /// <returns>The window.</returns>
    internal static LorawanWindow WindowOf(PamojaLorawanWindow window) =>
        new(window.DelayUs, window.FrequencyHz, window.DataRate, LinkOf(window.Link));
}
