using Pamoja.Lora;
using Pamoja.Native.Interop;

namespace Pamoja.Lorawan;

/// <summary>How an end device decides whether to send through a relay, TS011-1.0.1 section 10.2.</summary>
public enum LorawanRelayActivation
{
    /// <summary>Never, the default.</summary>
    Disabled = 0,

    /// <summary>Always.</summary>
    Enabled = 1,

    /// <summary>Only once a run of uplinks went unanswered, as the network's level sets.</summary>
    Dynamic = 2,

    /// <summary>However the device itself decides, which <see cref="LorawanEndDevice.UseRelay"/> sets.</summary>
    DeviceControlled = 3,
}

/// <summary>What an end device knows of its relay's scans, TS011-1.0.1 section 3.9.</summary>
public enum LorawanRelaySync
{
    /// <summary>Nothing: no wake-on-radio frame has gone out yet.</summary>
    Initialized = 0,

    /// <summary>One went out unanswered, so the next preamble spans a whole scan period.</summary>
    Unsynchronized = 1,

    /// <summary>The device knows when the relay scans, so a short preamble reaches it.</summary>
    Synchronized = 2,
}

/// <summary>The frame that wakes a relay, and where it goes.</summary>
/// <param name="Frame">Five bytes ahead of a join request, fifteen ahead of an uplink.</param>
/// <param name="StartMicros">When to start sending it.</param>
/// <param name="Carrier">Where it goes, and how fast.</param>
/// <param name="Link">Its LoRa settings, with the preamble this frame needs, sent with inverted IQ.</param>
/// <param name="OutputDbm">The power to ask of the radio, conducted, in dBm.</param>
/// <param name="AirtimeMicros">How long it holds the air.</param>
public sealed record LorawanWakeUp(
    byte[] Frame,
    ulong StartMicros,
    LorawanCarrier Carrier,
    LoraLink Link,
    sbyte OutputDbm,
    ulong AirtimeMicros);

/// <summary>When and where a relay's acknowledgment would arrive.</summary>
/// <param name="StartMicros">When it starts.</param>
/// <param name="Carrier">Where it arrives, and how fast.</param>
/// <param name="Link">The LoRa settings to listen with.</param>
/// <param name="AirtimeMicros">How long it lasts.</param>
public sealed record LorawanAckWindow(
    ulong StartMicros,
    LorawanCarrier Carrier,
    LoraLink Link,
    ulong AirtimeMicros);

/// <summary>The wake-on-radio exchange an uplink under a relay goes out behind.</summary>
/// <param name="WakeUp">The frame that wakes the relay.</param>
/// <param name="Ack">
/// Where the relay's acknowledgment would arrive, or <c>null</c> ahead of a join request, which
/// no relay acknowledges.
/// </param>
/// <param name="UplinkStartMicros">
/// When the uplink itself goes out, whether or not the acknowledgment arrives.
/// </param>
/// <param name="Rxr">The relay window, timed from the end of the uplink like the other two.</param>
public sealed record LorawanRelayExchange(
    LorawanWakeUp WakeUp,
    LorawanAckWindow? Ack,
    ulong UplinkStartMicros,
    LorawanWindow Rxr);

/// <summary>What a relay's acknowledgment said about itself, TS011-1.0.1 table 14.</summary>
/// <param name="CadPeriodicity">How often it scans.</param>
/// <param name="XtalAccuracy">How accurate its crystal is.</param>
/// <param name="CadToRx">How long it takes to start receiving.</param>
/// <param name="RelayDataRate">The data rate it forwards at, which bounds what the device may send.</param>
/// <param name="Forward">Whether it will forward.</param>
public readonly record struct LorawanRelayStatus(
    LorawanCadPeriodicity CadPeriodicity,
    LorawanXtalAccuracy XtalAccuracy,
    LorawanCadToRx CadToRx,
    byte RelayDataRate,
    LorawanRelayForward Forward);

/// <summary>What an end device does once its wake-on-radio frame went unanswered.</summary>
/// <param name="Uplink">Whether to send the uplink at the time the exchange named anyway.</param>
/// <param name="WakeUp">The next exchange, when the relay is woken again first.</param>
public readonly record struct LorawanWorNext(bool Uplink, LorawanRelayExchange? WakeUp);

/// <summary>A scan for wake-on-radio frames, due next.</summary>
/// <param name="StartMicros">When to start detecting.</param>
/// <param name="Channel">Which channel it listens on.</param>
/// <param name="Carrier">Where to listen, and how fast.</param>
/// <param name="Link">The LoRa settings of a wake-on-radio frame, heard with inverted IQ.</param>
/// <param name="PreambleSymbols">The longest preamble an end device sends on the channel.</param>
public sealed record LorawanScan(
    ulong StartMicros,
    LorawanWorChannel Channel,
    LorawanCarrier Carrier,
    LoraLink Link,
    ushort PreambleSymbols)
{
    /// <summary>The scan as the C ABI holds it, which <c>HeardWor</c> hands back.</summary>
    internal PamojaLorawanScan Native { get; init; }
}

/// <summary>The acknowledgment a relay answers a wake-on-radio frame with.</summary>
/// <param name="Frame">The frame, seven bytes.</param>
/// <param name="StartMicros">When to start sending it.</param>
/// <param name="Carrier">Where it goes, and how fast.</param>
/// <param name="Link">Its LoRa settings, sent with inverted IQ.</param>
/// <param name="OutputDbm">The power to ask of the radio, conducted, in dBm.</param>
/// <param name="AirtimeMicros">How long it holds the air.</param>
public sealed record LorawanAcknowledgment(
    byte[] Frame,
    ulong StartMicros,
    LorawanCarrier Carrier,
    LoraLink Link,
    sbyte OutputDbm,
    ulong AirtimeMicros);

/// <summary>When and where a relay listens for the uplink a wake-on-radio frame announced.</summary>
/// <param name="StartMicros">When the uplink starts.</param>
/// <param name="Carrier">Where it arrives, and how fast.</param>
/// <param name="Link">Its LoRa settings, heard with standard IQ.</param>
/// <param name="MaxLen">The longest frame the relay forwards; stop receiving anything longer.</param>
public sealed record LorawanListen(
    ulong StartMicros,
    LorawanCarrier Carrier,
    LoraLink Link,
    int MaxLen);

/// <summary>What a wake-on-radio frame led a relay to do.</summary>
public abstract record LorawanWake
{
    private LorawanWake()
    {
    }

    /// <summary>A join request from a device the relay's filters let through.</summary>
    /// <param name="Listen">Where and when to listen for the join request.</param>
    public sealed record JoinRequest(LorawanListen Listen) : LorawanWake;

    /// <summary>An uplink from a trusted device.</summary>
    /// <param name="DevAddr">The device that sent it.</param>
    /// <param name="Wfcnt">The wake-on-radio frame counter it carried.</param>
    /// <param name="Forward">Whether the relay forwards the uplink.</param>
    /// <param name="Acknowledgment">The acknowledgment to send, where there is one.</param>
    /// <param name="Listen">Where and when to listen for the uplink, where the relay will.</param>
    public sealed record Uplink(
        uint DevAddr,
        uint Wfcnt,
        LorawanRelayForward Forward,
        LorawanAcknowledgment? Acknowledgment,
        LorawanListen? Listen) : LorawanWake;

    /// <summary>A frame from a device the relay does not know, which it tells its network about.</summary>
    /// <param name="DevAddr">The address the frame named.</param>
    public sealed record Notified(uint DevAddr) : LorawanWake;
}

/// <summary>A downlink for an end device, to send in its relay window.</summary>
/// <param name="Frame">The frame to send.</param>
/// <param name="StartMicros">When to start sending it.</param>
/// <param name="Carrier">Where it goes, and how fast.</param>
/// <param name="Link">Its LoRa settings, sent with inverted IQ and a payload CRC.</param>
/// <param name="OutputDbm">The power to ask of the radio, conducted, in dBm.</param>
/// <param name="AirtimeMicros">How long it holds the air.</param>
public sealed record LorawanRxrDownlink(
    byte[] Frame,
    ulong StartMicros,
    LorawanCarrier Carrier,
    LoraLink Link,
    sbyte OutputDbm,
    ulong AirtimeMicros);

/// <summary>What a frame a relay's own device heard turned out to be.</summary>
public abstract record LorawanRelayHeard
{
    private LorawanRelayHeard(LorawanHeard heard) => Heard = heard;

    /// <summary>What the relay's own device made of the frame.</summary>
    public LorawanHeard Heard { get; }

    /// <summary>The relay's own device read it.</summary>
    /// <param name="Heard">What it turned out to be.</param>
    public sealed record Device(LorawanHeard Heard) : LorawanRelayHeard(Heard);

    /// <summary>A downlink for an end device, to send in its relay window.</summary>
    /// <param name="Heard">What the relay's own device read of the frame carrying it.</param>
    /// <param name="Forwarded">The downlink to send on.</param>
    public sealed record Downlink(LorawanHeard Heard, LorawanRxrDownlink Forwarded) : LorawanRelayHeard(Heard);

    /// <summary>A downlink for an end device the relay cannot send on.</summary>
    /// <param name="Heard">What the relay's own device read of the frame carrying it.</param>
    /// <param name="Reason">Why it cannot be sent on.</param>
    public sealed record Undeliverable(LorawanHeard Heard, string Reason) : LorawanRelayHeard(Heard);
}

/// <summary>An end device a relay forwards for, as an <c>UpdateUplinkListReq</c> describes it.</summary>
/// <param name="DevAddr">The device's address.</param>
/// <param name="RootWorSKey">
/// Its root relay session key, from <see cref="LorawanRelay.RootWorSKey"/> or the network's
/// trust command.
/// </param>
public sealed record LorawanTrustedDevice(uint DevAddr, byte[] RootWorSKey)
{
    /// <summary>The wake-on-radio frame counter to expect from it next.</summary>
    public uint NextWfcnt { get; init; }

    /// <summary>How many of its uplinks are forwarded an hour, 63 for no limit, table 55.</summary>
    public byte ReloadRate { get; init; } = 63;

    /// <summary>The coded bucket size multiplier, table 55.</summary>
    public byte BucketSize { get; init; }
}

/// <summary>Thrown when a relay cannot do what it was asked.</summary>
public sealed class LorawanRelayException : PamojaException
{
    /// <summary>Creates the exception from what the native relay reported.</summary>
    /// <param name="message">The reason, in words.</param>
    internal LorawanRelayException(string message)
        : base(message)
    {
    }
}

/// <summary>A LoRaWAN relay: an end device that also carries the uplinks of the devices around it.</summary>
/// <remarks>
/// A relay sleeps, waking every scan period to look for radio activity on its wake-on-radio
/// channel. One turn runs like this: <see cref="NextScan"/> says when and where to listen, a
/// frame heard there goes to <see cref="HeardWor"/>, the uplink it announced to
/// <see cref="HeardUplink"/>, and <see cref="Forward"/> wraps that in one of the relay's own
/// uplinks on port 226. What the relay's own receive windows hear goes to <see cref="HeardIn"/>,
/// which turns a downlink meant for an end device into one to send in that device's relay window.
/// <para>
/// It owns no radio and no clock, so the same relay runs over any radio, or in a test with none.
/// The codecs behind it are on <see cref="LorawanRelay"/>.
/// </para>
/// </remarks>
public sealed class LorawanRelayNode : IDisposable
{
    private readonly NativeHandle _handle;

    private LorawanRelayNode(IntPtr relay) =>
        _handle = new NativeHandle(relay, NativeMethods.pamoja_lorawan_relay_free);

    /// <summary>Makes a relay whose own device is activated by personalization.</summary>
    /// <param name="plan">A published channel plan.</param>
    /// <param name="session">The address and session keys the relay was provisioned with.</param>
    /// <param name="settings">What its radio can do.</param>
    /// <param name="xtalAccuracy">How accurate its crystal is, which its acknowledgments report.</param>
    /// <param name="cadToRx">How long it takes to start receiving once it detects activity.</param>
    /// <returns>The relay, stopped until <see cref="Start"/>.</returns>
    /// <exception cref="PamojaException">The plan was built rather than published.</exception>
    public static LorawanRelayNode Personalized(
        LoraChannelPlan plan,
        LorawanSession session,
        LorawanDeviceSettings settings,
        LorawanXtalAccuracy xtalAccuracy = LorawanXtalAccuracy.Ppm40,
        LorawanCadToRx cadToRx = LorawanCadToRx.Symbols8)
    {
        PamojaLorawanDeviceSettings native = settings.Native;
        using NativeLease held = plan.Lease();
        IntPtr planPointer = held.Pointer;
        IntPtr relay = session.UseHandle(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_personalized(
                planPointer,
                handle,
                in native,
                (byte)xtalAccuracy,
                (byte)cadToRx,
                out IntPtr made));
            return made;
        });
        return new LorawanRelayNode(relay);
    }

    /// <summary>Makes a relay whose own device joins over the air.</summary>
    /// <param name="plan">A published channel plan.</param>
    /// <param name="credentials">The relay's identifiers and root key.</param>
    /// <param name="settings">What its radio can do.</param>
    /// <param name="xtalAccuracy">How accurate its crystal is.</param>
    /// <param name="cadToRx">How long it takes to start receiving once it detects activity.</param>
    /// <returns>The relay, neither joined nor scanning.</returns>
    /// <exception cref="PamojaException">The plan was built rather than published.</exception>
    public static LorawanRelayNode OverTheAir(
        LoraChannelPlan plan,
        LorawanDevice credentials,
        LorawanDeviceSettings settings,
        LorawanXtalAccuracy xtalAccuracy = LorawanXtalAccuracy.Ppm40,
        LorawanCadToRx cadToRx = LorawanCadToRx.Symbols8)
    {
        PamojaLorawanDeviceSettings native = settings.Native;
        using NativeLease held = plan.Lease();
        IntPtr planPointer = held.Pointer;
        IntPtr relay = credentials.UseHandle(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_new(
                planPointer,
                handle,
                in native,
                (byte)xtalAccuracy,
                (byte)cadToRx,
                out IntPtr made));
            return made;
        });
        return new LorawanRelayNode(relay);
    }

    /// <summary>Whether the relay is scanning.</summary>
    public bool Running => _handle.Use(handle => NativeMethods.pamoja_lorawan_relay_running(handle) != 0);

    /// <summary>When the forwarded uplink waiting to go out is due, or <c>null</c> with none waiting.</summary>
    public ulong? ForwardDue => _handle.Use<ulong?>(handle =>
        NativeMethods.pamoja_lorawan_relay_forward_due(handle, out ulong dueUs) != 0 ? dueUs : null);

    /// <summary>Whether the relay's own device is on a network.</summary>
    public bool Joined => ReadStatus().Joined != 0;

    /// <summary>The address the relay's own device is on the network by, or <c>null</c> before joining.</summary>
    public uint? DevAddr
    {
        get
        {
            PamojaLorawanEndDeviceStatus status = ReadStatus();
            return status.Joined != 0 ? status.DevAddr : null;
        }
    }

    /// <summary>The data rate the relay forwards at, which its acknowledgments report.</summary>
    public byte DataRate => ReadStatus().DataRate;

    /// <summary>Starts scanning, or changes what a running relay scans from its next scan on.</summary>
    /// <param name="cadPeriodicity">How often to scan.</param>
    /// <param name="defaultChannelIndex">Which of the region's wake-on-radio channels is the default.</param>
    /// <param name="secondChannel">A second channel the network configured.</param>
    /// <exception cref="PamojaException">The region does not define that channel.</exception>
    /// <exception cref="LorawanRelayException">The configuration scans too often for its data rates.</exception>
    public void Start(
        LorawanCadPeriodicity cadPeriodicity = LorawanCadPeriodicity.Ms1000,
        byte defaultChannelIndex = 0,
        LoraRelayChannel? secondChannel = null) =>
        _handle.Use(handle => ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_relay_start(
            handle,
            (byte)cadPeriodicity,
            defaultChannelIndex,
            (byte)(secondChannel.HasValue ? 1 : 0),
            secondChannel?.WorFrequencyHz ?? 0,
            secondChannel?.AckFrequencyHz ?? 0,
            secondChannel?.DataRate ?? 0)));

    /// <summary>Stops scanning. A forwarded uplink already waiting still goes out.</summary>
    public void Stop() => _handle.Use(NativeMethods.pamoja_lorawan_relay_stop);

    /// <summary>Trusts an end device, as an <c>UpdateUplinkListReq</c> with the same fields does.</summary>
    /// <param name="index">The entry, 0 to 15.</param>
    /// <param name="device">The device, its key, and what it may spend.</param>
    /// <exception cref="ArgumentException">The device's key is not sixteen bytes.</exception>
    /// <exception cref="PamojaException">The index is past 15.</exception>
    public void Trust(byte index, LorawanTrustedDevice device)
    {
        ArgumentNullException.ThrowIfNull(device);
        FixedWidth.Require(device.RootWorSKey, NativeMethods.LorawanKeyLen, nameof(device.RootWorSKey));
        _handle.Use(handle => ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_relay_trust(
            handle,
            index,
            device.DevAddr,
            device.RootWorSKey,
            device.NextWfcnt,
            device.ReloadRate,
            device.BucketSize)));
    }

    /// <summary>Says when and where to scan next.</summary>
    /// <param name="nowMicros">The time, in microseconds.</param>
    /// <returns>The scan, or <c>null</c> while the relay is stopped.</returns>
    public LorawanScan? NextScan(ulong nowMicros) =>
        _handle.Use<LorawanScan?>(handle =>
        {
            PamojaStatus status = NativeMethods.pamoja_lorawan_relay_next_scan(
                handle, nowMicros, out PamojaLorawanScan scan);
            if (status != PamojaStatus.Ok)
            {
                OwnedString.ReadOrNull(NativeMethods.pamoja_lorawan_relay_error(handle));
                return null;
            }

            return new LorawanScan(
                scan.StartUs,
                (LorawanWorChannel)scan.Channel,
                new LorawanCarrier(scan.FrequencyHz, scan.DataRate),
                LorawanEndDevice.LinkOf(scan.Link),
                scan.PreambleSymbols)
            {
                Native = scan,
            };
        });

    /// <summary>Reads a wake-on-radio frame a scan heard.</summary>
    /// <param name="scan">The scan that heard it, as <see cref="NextScan"/> returned it.</param>
    /// <param name="frame">The bytes the radio received.</param>
    /// <param name="rssiDbm">Its received signal strength, which the forwarded uplink carries.</param>
    /// <param name="snrDb">Its signal-to-noise ratio, which the forwarded uplink carries.</param>
    /// <param name="endedMicros">When the frame ended, in microseconds.</param>
    /// <returns>What the frame led to.</returns>
    /// <exception cref="LorawanRelayException">
    /// The frame does not verify, the relay does not forward for that device, or a limit is spent.
    /// </exception>
    public LorawanWake HeardWor(
        LorawanScan scan,
        ReadOnlySpan<byte> frame,
        short rssiDbm,
        sbyte snrDb,
        ulong endedMicros)
    {
        byte[] bytes = frame.ToArray();
        PamojaLorawanScan native = scan.Native;
        return _handle.Use<LorawanWake>(handle =>
        {
            ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_relay_heard_wor(
                handle,
                in native,
                bytes,
                (nuint)bytes.Length,
                rssiDbm,
                snrDb,
                endedMicros,
                out PamojaLorawanWake wake));
            LorawanListen? listen = wake.HasListen != 0
                ? new LorawanListen(
                    wake.ListenStartUs,
                    new LorawanCarrier(wake.ListenFrequencyHz, wake.ListenDataRate),
                    LorawanEndDevice.LinkOf(wake.ListenLink),
                    (int)wake.ListenMaxLen)
                : null;
            return wake.Kind switch
            {
                NativeMethods.LorawanWakeJoinRequest => new LorawanWake.JoinRequest(listen!),
                NativeMethods.LorawanWakeNotified => new LorawanWake.Notified(wake.DevAddr),
                _ => new LorawanWake.Uplink(
                    wake.DevAddr,
                    wake.Wfcnt,
                    (LorawanRelayForward)wake.Forward,
                    wake.HasAck != 0
                        ? new LorawanAcknowledgment(
                            wake.AckFrame.ToArray(),
                            wake.AckStartUs,
                            new LorawanCarrier(wake.AckFrequencyHz, wake.AckDataRate),
                            LorawanEndDevice.LinkOf(wake.AckLink),
                            wake.AckOutputDbm,
                            wake.AckAirtimeUs)
                        : null,
                    listen),
            };
        });
    }

    /// <summary>Reads the uplink a wake-on-radio frame announced, and holds it to forward.</summary>
    /// <param name="frame">The bytes the radio received.</param>
    /// <param name="rssiDbm">Its received signal strength.</param>
    /// <param name="snrDb">Its signal-to-noise ratio.</param>
    /// <param name="endedMicros">When the frame ended, in microseconds.</param>
    /// <returns>When to <see cref="Forward"/> it: fifty milliseconds after it ended.</returns>
    /// <exception cref="LorawanRelayException">
    /// Nothing announced it, or the frame is not the one it announced.
    /// </exception>
    public ulong HeardUplink(ReadOnlySpan<byte> frame, short rssiDbm, sbyte snrDb, ulong endedMicros)
    {
        byte[] bytes = frame.ToArray();
        return _handle.Use(handle =>
        {
            ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_relay_heard_uplink(
                handle, bytes, (nuint)bytes.Length, rssiDbm, snrDb, endedMicros, out ulong dueUs));
            return dueUs;
        });
    }

    /// <summary>Clears the uplink a wake-on-radio frame announced, once listening heard nothing.</summary>
    public void UplinkMissed() => _handle.Use(NativeMethods.pamoja_lorawan_relay_uplink_missed);

    /// <summary>Sends the uplink the relay is holding, in one of its own on port 226.</summary>
    /// <param name="nowMicros">The time, in microseconds.</param>
    /// <returns>What to transmit, with the relay's own receive windows.</returns>
    /// <exception cref="LorawanRelayException">
    /// Nothing is waiting to go out, or the relay's own device cannot send it yet.
    /// </exception>
    public LorawanTransmission Forward(ulong nowMicros) =>
        _handle.Use(handle => Taken(
            handle,
            NativeMethods.pamoja_lorawan_relay_forward(
                handle, nowMicros, out IntPtr frame, out PamojaLorawanTransmission transmission),
            frame,
            transmission));

    /// <summary>Reads a frame the relay's own device heard, acting on the relay commands in it.</summary>
    /// <param name="window">The window the radio heard it in.</param>
    /// <param name="frame">The bytes the radio received.</param>
    /// <param name="snrDb">Its signal-to-noise ratio.</param>
    /// <returns>
    /// What it turned out to be: the relay's own downlink, one to pass on to an end device, or one
    /// that cannot be passed on.
    /// </returns>
    /// <exception cref="LorawanRelayException">As <see cref="LorawanEndDevice.Heard"/>.</exception>
    public LorawanRelayHeard HeardIn(LorawanReceiveWindow window, ReadOnlySpan<byte> frame, sbyte snrDb)
    {
        if (!Enum.IsDefined(window))
        {
            throw new ArgumentOutOfRangeException(nameof(window), window, "not a receive window");
        }

        byte[] bytes = frame.ToArray();
        return _handle.Use<LorawanRelayHeard>(handle =>
        {
            ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_relay_heard_in(
                handle,
                (byte)window,
                bytes,
                (nuint)bytes.Length,
                snrDb,
                out byte kind,
                out PamojaLorawanHeard heard,
                out IntPtr payload,
                out IntPtr downlinkFrame,
                out PamojaLorawanRxr downlink));
            LorawanHeard read = LorawanEndDevice.HeardOf(heard, payload);
            return kind switch
            {
                NativeMethods.LorawanRelayHeardDownlink => new LorawanRelayHeard.Downlink(
                    read,
                    new LorawanRxrDownlink(
                        Pamoja.Codec.Codec.TakeBytes(downlinkFrame),
                        downlink.StartUs,
                        new LorawanCarrier(downlink.FrequencyHz, downlink.DataRate),
                        LorawanEndDevice.LinkOf(downlink.Link),
                        downlink.OutputDbm,
                        downlink.AirtimeUs)),
                NativeMethods.LorawanRelayHeardUndeliverable => new LorawanRelayHeard.Undeliverable(
                    read,
                    OwnedString.ReadOrNull(NativeMethods.pamoja_lorawan_relay_error(handle))
                        ?? "the downlink cannot be sent on"),
                _ => new LorawanRelayHeard.Device(read),
            };
        });
    }

    /// <summary>Says what comes next once the relay's own windows closed with nothing in them.</summary>
    /// <param name="nowMicros">The time the second window closed, in microseconds.</param>
    /// <returns>Whether to repeat the frame, join again, or move on.</returns>
    /// <exception cref="LorawanRelayException">No transmission waits on its windows.</exception>
    public LorawanNext NothingHeard(ulong nowMicros) =>
        _handle.Use(handle =>
        {
            ThrowIfFailed(handle, NativeMethods.pamoja_lorawan_relay_nothing_heard(
                handle, nowMicros, out byte next, out ulong notBeforeUs));
            LorawanNextKind kind = (LorawanNextKind)next;
            return new LorawanNext(
                kind,
                kind is LorawanNextKind.Repeat or LorawanNextKind.JoinAgain ? notBeforeUs : null);
        });

    /// <summary>Makes the relay's own join request.</summary>
    /// <param name="devNonce">A nonce this relay has never used with its join EUI.</param>
    /// <param name="nowMicros">The time, in microseconds.</param>
    /// <returns>What to transmit, with the join accept windows.</returns>
    /// <exception cref="LorawanRelayException">No credentials, busy, or a wait for the air.</exception>
    public LorawanTransmission Join(ushort devNonce, ulong nowMicros) =>
        _handle.Use(handle => Taken(
            handle,
            NativeMethods.pamoja_lorawan_relay_join(
                handle, devNonce, nowMicros, out IntPtr frame, out PamojaLorawanTransmission transmission),
            frame,
            transmission));

    /// <summary>Sends one of the relay's own uplinks, which also carries what it owes its network.</summary>
    /// <param name="port">The application port, 1 to 223.</param>
    /// <param name="payload">The application payload.</param>
    /// <param name="nowMicros">The time, in microseconds.</param>
    /// <param name="confirmed">Whether to ask the network to acknowledge it.</param>
    /// <returns>What to transmit, with its receive windows.</returns>
    /// <exception cref="LorawanRelayException">As <see cref="LorawanEndDevice.Send"/>.</exception>
    public LorawanTransmission Send(byte port, ReadOnlySpan<byte> payload, ulong nowMicros, bool confirmed = false)
    {
        byte[] body = payload.ToArray();
        return _handle.Use(handle => Taken(
            handle,
            NativeMethods.pamoja_lorawan_relay_send(
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

    /// <summary>Sends an uplink with no payload, carrying whatever the relay owes its network.</summary>
    /// <param name="nowMicros">The time, in microseconds.</param>
    /// <returns>What to transmit, with its receive windows.</returns>
    /// <exception cref="LorawanRelayException">As <see cref="LorawanEndDevice.Send"/>.</exception>
    public LorawanTransmission SendEmpty(ulong nowMicros) =>
        _handle.Use(handle => Taken(
            handle,
            NativeMethods.pamoja_lorawan_relay_send(
                handle,
                0,
                ReadOnlySpan<byte>.Empty,
                0,
                0,
                nowMicros,
                out IntPtr frame,
                out PamojaLorawanTransmission transmission),
            frame,
            transmission));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Reads where the relay's own device stands.</summary>
    /// <returns>The status.</returns>
    private PamojaLorawanEndDeviceStatus ReadStatus() =>
        _handle.Use(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_status(
                handle, out PamojaLorawanEndDeviceStatus status));
            return status;
        });

    /// <summary>Takes the frame of a transmitting call, or throws why it failed.</summary>
    /// <param name="handle">The live relay handle.</param>
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
        return LorawanEndDevice.TransmissionOf(frame, transmission);
    }

    /// <summary>Throws the relay's recorded reason when a call failed.</summary>
    /// <param name="handle">The live relay handle.</param>
    /// <param name="status">What the call returned.</param>
    private static void ThrowIfFailed(IntPtr handle, PamojaStatus status)
    {
        if (status == PamojaStatus.Ok)
        {
            return;
        }

        string? reason = OwnedString.ReadOrNull(NativeMethods.pamoja_lorawan_relay_error(handle));
        if (reason is not null)
        {
            throw new LorawanRelayException(reason);
        }

        Status.ThrowIfError(status);
    }
}
