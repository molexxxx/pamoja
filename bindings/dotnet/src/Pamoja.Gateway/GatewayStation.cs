using System.Text;

using Pamoja.Native.Interop;

using NativeStatus = Pamoja.Native.Interop.Status;

namespace Pamoja.Gateway;

/// <summary>Which kind of message a session carries.</summary>
public enum GatewayStationKind : byte
{
    /// <summary>A join request the station heard.</summary>
    JoinRequest = 0,

    /// <summary>A data frame the station heard.</summary>
    Uplink = 1,

    /// <summary>A frame of a kind this protocol does not describe, carried whole.</summary>
    Proprietary = 2,

    /// <summary>What the station reports about itself when a session opens.</summary>
    Version = 3,

    /// <summary>How the server tells the station to configure its radios.</summary>
    RouterConfig = 4,

    /// <summary>A frame the server asks the station to transmit.</summary>
    Downlink = 5,

    /// <summary>Frames the server asks the station to transmit to a group.</summary>
    Schedule = 6,

    /// <summary>What became of a frame the station was asked to transmit.</summary>
    Transmitted = 7,

    /// <summary>The clock the two keep between them.</summary>
    TimeSync = 8,

    /// <summary>A kind this build does not model, readable only as its text.</summary>
    Other = 9,
}

/// <summary>How a station heard a packet, as it reports it.</summary>
/// <param name="Rctx">The radio it arrived on, which an answer goes back out on.</param>
/// <param name="Xtime">The station clock, in microseconds.</param>
public sealed record GatewayStationLevels(long Rctx, long Xtime)
{
    /// <summary>The GPS time, when the station has one.</summary>
    public long? Gpstime { get; init; }

    /// <summary>The received signal strength, in dBm.</summary>
    public double Rssi { get; init; }

    /// <summary>The signal-to-noise ratio, in dB.</summary>
    public double Snr { get; init; }
}

/// <summary>A receive window, or the ping slot a class B frame goes out in.</summary>
/// <param name="DataRate">The data rate, as the network's table numbers it.</param>
/// <param name="FrequencyHz">The frequency in hertz.</param>
public sealed record GatewayStationWindow(byte DataRate, uint FrequencyHz);

/// <summary>One number of a configuration's data-rate table.</summary>
/// <param name="SpreadingFactor">The spreading factor, 0 for FSK.</param>
/// <param name="BandwidthHz">The bandwidth in hertz.</param>
/// <param name="DownlinkOnly">Whether the rate is used only for downlinks.</param>
public sealed record GatewayStationDataRate(byte SpreadingFactor, uint BandwidthHz, bool DownlinkOnly = false);

/// <summary>A range of join identifiers whose join requests a station forwards, both ends included.</summary>
/// <param name="First">The first identifier, read as a big-endian number.</param>
/// <param name="Last">The last identifier, read the same way.</param>
public sealed record GatewayStationJoinRange(ulong First, ulong Last);

/// <summary>One frame of a schedule, transmitted to a group rather than a device.</summary>
/// <param name="Pdu">The frame to transmit.</param>
/// <param name="DataRate">The data rate to transmit at.</param>
/// <param name="FrequencyHz">The frequency to transmit on, in hertz.</param>
public sealed record GatewayStationBroadcast(byte[] Pdu, byte DataRate, uint FrequencyHz)
{
    /// <summary>How urgent it is.</summary>
    public byte Priority { get; init; }

    /// <summary>When to transmit it, in microseconds since the GPS epoch.</summary>
    public long? Gpstime { get; init; }

    /// <summary>The radio to transmit on.</summary>
    public long? Rctx { get; init; }
}

/// <summary>A station clock value taken apart.</summary>
/// <param name="Unit">The radio unit the time was read on, 0 to 127.</param>
/// <param name="Session">The run of the station the time belongs to.</param>
/// <param name="Micros">The microseconds the run had counted, below 2^48.</param>
public sealed record GatewayStationXtime(byte Unit, byte Session, long Micros);

/// <summary>The answer a discovery endpoint gives.</summary>
/// <param name="Uri">The websocket to open, or <c>null</c> when the station was refused.</param>
/// <param name="Error">Why the station was refused, or <c>null</c> when it was not.</param>
public sealed record GatewayStationRouter(string? Uri, string? Error)
{
    /// <summary>The station, as the server read it, when the answer names it.</summary>
    public byte[]? Router { get; init; }

    /// <summary>The server endpoint that carries the session, when the answer names it.</summary>
    public byte[]? Muxs { get; init; }
}

/// <summary>A message either side of a session sends.</summary>
/// <param name="Kind">Which kind of message.</param>
/// <remarks>
/// Every message names its kind, and carries the fields that kind uses; the rest read as
/// <c>null</c>, zero, or empty. Build one with an initializer and write it with
/// <see cref="GatewayStation.Encode"/>, or read one with <see cref="GatewayStation.Parse"/>
/// or <see cref="GatewayStation.Heard"/>.
/// </remarks>
public sealed record GatewayStationMessage(GatewayStationKind Kind)
{
    /// <summary>
    /// The word the message calls itself on the wire, such as <c>updf</c>. A message of a kind
    /// this build does not model is written with this word.
    /// </summary>
    public string? Msgtype { get; init; }

    /// <summary>The MAC header byte, for a join request or a data frame.</summary>
    public byte Mhdr { get; init; }

    /// <summary>The application being joined, for a join request.</summary>
    public byte[]? JoinEui { get; init; }

    /// <summary>The device, for a join request, a downlink, or a transmission report.</summary>
    public byte[]? DevEui { get; init; }

    /// <summary>The nonce a join request used.</summary>
    public ushort DevNonce { get; init; }

    /// <summary>The address a data frame came from.</summary>
    public int DevAddr { get; init; }

    /// <summary>The frame control byte.</summary>
    public byte Fctrl { get; init; }

    /// <summary>The frame counter, as the sixteen bits on the air.</summary>
    public ushort Fcnt { get; init; }

    /// <summary>The port, or <c>null</c> for a frame carrying only options.</summary>
    public byte? Fport { get; init; }

    /// <summary>The frame options a data frame carries.</summary>
    public byte[] Fopts { get; init; } = [];

    /// <summary>The payload, still encrypted, the whole of a proprietary frame, or the frame a downlink transmits.</summary>
    public byte[] Payload { get; init; } = [];

    /// <summary>The message integrity code.</summary>
    public int Mic { get; init; }

    /// <summary>The data rate a frame arrived at.</summary>
    public byte DataRate { get; init; }

    /// <summary>The frequency a frame arrived on, in hertz.</summary>
    public uint FrequencyHz { get; init; }

    /// <summary>How it was heard, for the kinds a station sends up.</summary>
    public GatewayStationLevels? Levels { get; init; }

    /// <summary>Which class of downlink this is: 0 for A, 1 for B, 2 for C.</summary>
    public byte Class { get; init; }

    /// <summary>The identifier a downlink and its transmission report share.</summary>
    public long Diid { get; init; }

    /// <summary>The delay before the first receive window, in seconds.</summary>
    public byte? RxDelay { get; init; }

    /// <summary>How urgent a downlink is.</summary>
    public byte Priority { get; init; }

    /// <summary>The station software, for a version.</summary>
    public string? Station { get; init; }

    /// <summary>Its firmware, for a version.</summary>
    public string? Firmware { get; init; }

    /// <summary>The package it came from, for a version.</summary>
    public string? Package { get; init; }

    /// <summary>The hardware model, for a version.</summary>
    public string? Model { get; init; }

    /// <summary>The protocol version it speaks, for a version; <see cref="GatewayStation.ProtocolVersion"/> when not set.</summary>
    public uint? Protocol { get; init; }

    /// <summary>What it can do, such as <c>gps</c>, for a version.</summary>
    public string? Features { get; init; }

    /// <summary>
    /// The networks whose data frames a configuration forwards, or <c>null</c> to forward
    /// every network's.
    /// </summary>
    /// <remarks>
    /// A station matches each entry against the top seven bits of a frame's device address, so
    /// an empty list forwards no data frame at all.
    /// </remarks>
    public IReadOnlyList<uint>? NetIds { get; init; }

    /// <summary>The join identifier ranges a configuration forwards; empty forwards every join.</summary>
    public IReadOnlyList<GatewayStationJoinRange> JoinEuiRanges { get; init; } = [];

    /// <summary>The region a configuration names, such as <c>EU868</c>.</summary>
    public string? Region { get; init; }

    /// <summary>The concentrator a configuration is written for, such as <c>sx1301/1</c>.</summary>
    public string? Hwspec { get; init; }

    /// <summary>The highest radiated power a configuration allows, in dBm.</summary>
    public double? MaxEirp { get; init; }

    /// <summary>The lowest frequency a configuration allows, in hertz.</summary>
    public uint? FreqMinHz { get; init; }

    /// <summary>The highest frequency a configuration allows, in hertz.</summary>
    public uint? FreqMaxHz { get; init; }

    /// <summary>
    /// A configuration's data rates, indexed by data-rate number, with <c>null</c> for a
    /// number the table leaves undefined.
    /// </summary>
    public IReadOnlyList<GatewayStationDataRate?> DataRates { get; init; } = [];

    /// <summary>The first receive window a downlink names.</summary>
    public GatewayStationWindow? Rx1 { get; init; }

    /// <summary>The second receive window a downlink names.</summary>
    public GatewayStationWindow? Rx2 { get; init; }

    /// <summary>The ping slot a class B downlink goes out in.</summary>
    public GatewayStationWindow? PingSlot { get; init; }

    /// <summary>
    /// The station clock in microseconds: the uplink a downlink answers, the moment a reported
    /// frame went out, or the one a time sync carries.
    /// </summary>
    public long? Xtime { get; init; }

    /// <summary>The radio a downlink goes out on, or a report says it went out on.</summary>
    public long? Rctx { get; init; }

    /// <summary>The GPS time in microseconds since the GPS epoch.</summary>
    public long? Gpstime { get; init; }

    /// <summary>
    /// When a reported frame went out, in seconds, or the station time a time sync carries,
    /// in microseconds.
    /// </summary>
    public double? Txtime { get; init; }

    /// <summary>What to transmit to a group, for a schedule.</summary>
    public IReadOnlyList<GatewayStationBroadcast> Schedule { get; init; } = [];
}

/// <summary>The LoRa Basics Station protocol, from either side.</summary>
/// <remarks>
/// A station and its network server exchange JSON messages over a websocket the caller owns,
/// so nothing here opens a socket: split a frame the radio heard with <see cref="Heard"/>,
/// build any other message as a <see cref="GatewayStationMessage"/>, send what
/// <see cref="Encode"/> writes, and read whatever arrives with <see cref="Parse"/>.
/// </remarks>
public static class GatewayStation
{
    /// <summary>The path a station appends to its configured address to find its server.</summary>
    public const string DiscoveryPath = "/router-info";

    /// <summary>The protocol version a station reports.</summary>
    public const uint ProtocolVersion = 2;

    /// <summary>Reads a frame the radio heard into the message that reports it.</summary>
    /// <param name="frame">The bytes as they arrived, header through integrity code.</param>
    /// <param name="dataRate">The data rate it arrived at.</param>
    /// <param name="frequencyHz">The frequency it arrived on, in hertz.</param>
    /// <param name="levels">How it was heard.</param>
    /// <returns>The message to send up.</returns>
    /// <exception cref="PamojaException">
    /// The frame is shorter than the fields its header names, or carries a kind a station does
    /// not send up.
    /// </exception>
    public static GatewayStationMessage Heard(
        ReadOnlySpan<byte> frame,
        byte dataRate,
        uint frequencyHz,
        GatewayStationLevels? levels = null)
    {
        IntPtr native = NativeMethods.pamoja_gateway_station_heard(
            frame, (nuint)frame.Length, dataRate, frequencyHz, Native(levels));
        return Taken(native, "the frame is not one a station sends up");
    }

    /// <summary>Reads a message that arrived over the websocket.</summary>
    /// <param name="text">The message text.</param>
    /// <returns>The message.</returns>
    /// <exception cref="PamojaException">It is not a message this protocol defines.</exception>
    public static GatewayStationMessage Parse(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        byte[] bytes = Encoding.UTF8.GetBytes(text);
        IntPtr native = NativeMethods.pamoja_gateway_station_message_parse(bytes, (nuint)bytes.Length);
        return Taken(native, "the text is not this protocol");
    }

    /// <summary>Writes a message as the websocket carries it.</summary>
    /// <param name="message">The message, of any kind.</param>
    /// <returns>The text to send.</returns>
    /// <exception cref="ArgumentOutOfRangeException">The kind is not one of <see cref="GatewayStationKind"/>'s values.</exception>
    /// <exception cref="ArgumentException">
    /// An identifier the kind needs is missing or is not eight bytes: <c>JoinEui</c> and
    /// <c>DevEui</c> for a join request, <c>DevEui</c> for a downlink or a report. Or the
    /// message is of <see cref="GatewayStationKind.Other"/> and names no <c>Msgtype</c>.
    /// </exception>
    public static string Encode(GatewayStationMessage message)
    {
        ArgumentNullException.ThrowIfNull(message);
        GatewayStationKind kind = NamedValue.Require(message.Kind, nameof(message.Kind));

        PamojaGatewayStationFields fields = FieldsOf(message, kind);
        IntPtr native = NativeMethods.pamoja_gateway_station_message_new(in fields);
        if (native == IntPtr.Zero)
        {
            throw new PamojaException(NativeStatus.LastError() ?? "the message could not be built");
        }

        try
        {
            Fill(native, message, kind);
            NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_json(native, out IntPtr text));
            return Encoding.UTF8.GetString(OwnedBuffer.Take(text));
        }
        finally
        {
            NativeMethods.pamoja_gateway_station_message_free(native);
        }
    }

    /// <summary>Writes the request a station sends to find its network server.</summary>
    /// <param name="router">The station asking, eight bytes.</param>
    /// <returns>The text to send on <see cref="DiscoveryPath"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="router"/> is not eight bytes.</exception>
    public static string Discovery(ReadOnlySpan<byte> router)
    {
        RequireEui(router, nameof(router));
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_discovery(router, out IntPtr text));
        return Encoding.UTF8.GetString(OwnedBuffer.Take(text));
    }

    /// <summary>Reads the request a station sent to find its network server, as the server does.</summary>
    /// <param name="text">The request text.</param>
    /// <returns>The station asking, eight bytes.</returns>
    /// <exception cref="PamojaException">The text is not this request.</exception>
    public static byte[] DiscoveryParse(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        byte[] bytes = Encoding.UTF8.GetBytes(text);
        byte[] router = new byte[PamojaEui.Length];
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_discovery_parse(
            bytes, (nuint)bytes.Length, router));
        return router;
    }

    /// <summary>Writes the answer that sends a station to the websocket its session runs on.</summary>
    /// <param name="router">The station being answered, eight bytes.</param>
    /// <param name="muxs">The server endpoint that carries the session, eight bytes.</param>
    /// <param name="uri">The websocket address.</param>
    /// <returns>The answer to send back on <see cref="DiscoveryPath"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="router"/> or <paramref name="muxs"/> is not eight bytes.</exception>
    public static string RouterAccepted(ReadOnlySpan<byte> router, ReadOnlySpan<byte> muxs, string uri)
    {
        RequireEui(router, nameof(router));
        RequireEui(muxs, nameof(muxs));
        ArgumentNullException.ThrowIfNull(uri);

        byte[] address = Encoding.UTF8.GetBytes(uri);
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_router_accepted(
            router, muxs, address, (nuint)address.Length, out IntPtr text));
        return Encoding.UTF8.GetString(OwnedBuffer.Take(text));
    }

    /// <summary>Writes the answer that refuses a station, saying why.</summary>
    /// <param name="router">The station being refused, eight bytes.</param>
    /// <param name="error">What is wrong, in words the operator can act on.</param>
    /// <returns>The answer to send back on <see cref="DiscoveryPath"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="router"/> is not eight bytes.</exception>
    public static string RouterRefused(ReadOnlySpan<byte> router, string error)
    {
        RequireEui(router, nameof(router));
        ArgumentNullException.ThrowIfNull(error);

        byte[] why = Encoding.UTF8.GetBytes(error);
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_router_refused(
            router, why, (nuint)why.Length, out IntPtr text));
        return Encoding.UTF8.GetString(OwnedBuffer.Take(text));
    }

    /// <summary>Reads the answer a discovery endpoint gives.</summary>
    /// <param name="text">The answer text.</param>
    /// <returns>Where to connect, or why the station was refused, and whom the answer names.</returns>
    /// <exception cref="PamojaException">The answer is not this protocol.</exception>
    public static GatewayStationRouter RouterParse(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        byte[] bytes = Encoding.UTF8.GetBytes(text);
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_router_parse(
            bytes, (nuint)bytes.Length, out IntPtr uri, out IntPtr error));
        string opened = Encoding.UTF8.GetString(OwnedBuffer.Take(uri));
        string refused = Encoding.UTF8.GetString(OwnedBuffer.Take(error));

        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_router_identities(
            bytes, (nuint)bytes.Length, out PamojaGatewayStationRouterIds ids));
        return new GatewayStationRouter(
            opened.Length == 0 ? null : opened,
            refused.Length == 0 ? null : refused)
        {
            Router = ids.HasRouter == 0 ? null : ids.Router.ToArray(),
            Muxs = ids.HasMuxs == 0 ? null : ids.Muxs.ToArray(),
        };
    }

    /// <summary>
    /// Builds a station clock value from the radio it was read on, the run of the station, and
    /// the microseconds that run had counted.
    /// </summary>
    /// <param name="unit">The radio unit, 0 to 127.</param>
    /// <param name="session">The run of the station, which the reference station never leaves at 0.</param>
    /// <param name="micros">The microseconds the run had counted, below 2^48.</param>
    /// <returns>The value a message carries as its <c>xtime</c>.</returns>
    /// <exception cref="ArgumentOutOfRangeException">
    /// <paramref name="unit"/> is past 127, or <paramref name="micros"/> is negative or does
    /// not fit 48 bits.
    /// </exception>
    public static long Xtime(byte unit, byte session, long micros)
    {
        ArgumentOutOfRangeException.ThrowIfGreaterThan(unit, (byte)127);
        ArgumentOutOfRangeException.ThrowIfNegative(micros);
        ArgumentOutOfRangeException.ThrowIfGreaterThanOrEqual(micros, 1L << 48);
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_xtime(
            unit, session, (ulong)micros, out long value));
        return value;
    }

    /// <summary>Takes a station clock value apart into its radio unit, run, and microseconds.</summary>
    /// <param name="xtime">The value a message carried.</param>
    /// <returns>Its parts.</returns>
    public static GatewayStationXtime XtimeParts(long xtime)
    {
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_xtime_parts(
            xtime, out PamojaGatewayStationXtime parts));
        return new GatewayStationXtime(parts.Unit, parts.Session, (long)parts.Micros);
    }

    /// <summary>Writes an identifier in the ID6 form the protocol prefers.</summary>
    /// <param name="eui">The identifier, eight bytes.</param>
    /// <returns>The identifier, such as <c>1:203:405:607</c>.</returns>
    /// <exception cref="ArgumentException"><paramref name="eui"/> is not eight bytes.</exception>
    public static string Id6(ReadOnlySpan<byte> eui)
    {
        RequireEui(eui, nameof(eui));
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_id6(eui, out IntPtr text));
        return Encoding.UTF8.GetString(OwnedBuffer.Take(text));
    }

    /// <summary>Reads an identifier written in any form the protocol accepts.</summary>
    /// <param name="text">The identifier, in ID6, EUI, or plain hexadecimal form.</param>
    /// <returns>Its eight bytes.</returns>
    /// <exception cref="PamojaException">The text is not an identifier.</exception>
    public static byte[] EuiOf(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        byte[] eui = new byte[PamojaEui.Length];
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_eui_of(text, eui));
        return eui;
    }

    /// <summary>Refuses an identifier that is not eight bytes.</summary>
    /// <param name="eui">The identifier.</param>
    /// <param name="name">The argument's name, for the exception.</param>
    /// <exception cref="ArgumentException"><paramref name="eui"/> is not eight bytes.</exception>
    private static void RequireEui(ReadOnlySpan<byte> eui, string name)
    {
        if (eui.Length != PamojaEui.Length)
        {
            throw new ArgumentException($"{name} must be exactly {PamojaEui.Length} bytes", name);
        }
    }

    /// <summary>Reads a message a native call produced, and releases it.</summary>
    /// <param name="native">The handle, or zero when the call failed.</param>
    /// <param name="why">What to say when the call gave no reason.</param>
    /// <returns>The message.</returns>
    /// <exception cref="PamojaException">The call failed.</exception>
    private static GatewayStationMessage Taken(IntPtr native, string why)
    {
        if (native == IntPtr.Zero)
        {
            throw new PamojaException(NativeStatus.LastError() ?? why);
        }

        try
        {
            return Read(native);
        }
        finally
        {
            NativeMethods.pamoja_gateway_station_message_free(native);
        }
    }

    /// <summary>Writes how a packet was heard for the native call.</summary>
    private static PamojaGatewayStationLevels Native(GatewayStationLevels? levels) => levels is null
        ? default
        : new PamojaGatewayStationLevels
        {
            Rctx = levels.Rctx,
            Xtime = levels.Xtime,
            Gpstime = levels.Gpstime ?? 0,
            HasGpstime = Flag(levels.Gpstime is not null),
            Rssi = levels.Rssi,
            Snr = levels.Snr,
        };

    /// <summary>Writes a window for the native call.</summary>
    private static PamojaGatewayStationWindow Native(GatewayStationWindow? window) => window is null
        ? default
        : new PamojaGatewayStationWindow
        {
            DataRate = window.DataRate,
            FrequencyHz = window.FrequencyHz,
            Present = 1,
        };

    /// <summary>Writes a flag the way the native struct holds one.</summary>
    private static byte Flag(bool set) => set ? (byte)1 : (byte)0;

    /// <summary>Reads an identifier a kind needs, refusing one that is missing.</summary>
    private static PamojaEui Required(byte[]? eui, string name, GatewayStationKind kind) =>
        eui is null
            ? throw new ArgumentException($"a {kind} message needs {name}", name)
            : PamojaEui.From(eui, name);

    /// <summary>Writes the fixed fields of a message for the native call.</summary>
    private static PamojaGatewayStationFields FieldsOf(GatewayStationMessage message, GatewayStationKind kind)
    {
        PamojaEui joinEui = kind == GatewayStationKind.JoinRequest
            ? Required(message.JoinEui, nameof(message.JoinEui), kind)
            : default;
        PamojaEui devEui = kind is GatewayStationKind.JoinRequest
            or GatewayStationKind.Downlink
            or GatewayStationKind.Transmitted
            ? Required(message.DevEui, nameof(message.DevEui), kind)
            : default;

        return new PamojaGatewayStationFields
        {
            Kind = (byte)kind,
            Mhdr = message.Mhdr,
            JoinEui = joinEui,
            DevEui = devEui,
            DevNonce = message.DevNonce,
            DevAddr = message.DevAddr,
            Fctrl = message.Fctrl,
            Fcnt = message.Fcnt,
            Fport = message.Fport ?? 0,
            HasFport = Flag(message.Fport is not null),
            Mic = message.Mic,
            DataRate = message.DataRate,
            FrequencyHz = message.FrequencyHz,
            Levels = Native(message.Levels),
            Class = message.Class,
            Diid = message.Diid,
            RxDelay = message.RxDelay ?? 0,
            HasRxDelay = Flag(message.RxDelay is not null),
            Priority = message.Priority,
            Protocol = message.Protocol ?? ProtocolVersion,
            MaxEirp = message.MaxEirp ?? 0,
            FreqMinHz = message.FreqMinHz ?? 0,
            FreqMaxHz = message.FreqMaxHz ?? 0,
            FiltersNetworks = Flag(message.NetIds is not null),
            Rx1 = Native(message.Rx1),
            Rx2 = Native(message.Rx2),
            PingSlot = Native(message.PingSlot),
            Xtime = message.Xtime ?? 0,
            HasXtime = Flag(message.Xtime is not null),
            Rctx = message.Rctx ?? 0,
            HasRctx = Flag(message.Rctx is not null),
            Gpstime = message.Gpstime ?? 0,
            HasGpstime = Flag(message.Gpstime is not null),
            Txtime = message.Txtime ?? 0,
            HasTxtime = Flag(message.Txtime is not null),
        };
    }

    /// <summary>Adds the text, bytes and entries a kind carries to a message being built.</summary>
    private static void Fill(IntPtr native, GatewayStationMessage message, GatewayStationKind kind)
    {
        switch (kind)
        {
            case GatewayStationKind.Uplink:
                SetBytes(native, message.Payload, NativeMethods.pamoja_gateway_station_message_set_payload);
                SetBytes(native, message.Fopts, NativeMethods.pamoja_gateway_station_message_set_options);
                break;
            case GatewayStationKind.Proprietary:
            case GatewayStationKind.Downlink:
                SetBytes(native, message.Payload, NativeMethods.pamoja_gateway_station_message_set_payload);
                break;
            case GatewayStationKind.Version:
                SetText(native, NativeMethods.GatewayStationTextStation, message.Station);
                SetText(native, NativeMethods.GatewayStationTextFirmware, message.Firmware);
                SetText(native, NativeMethods.GatewayStationTextPackage, message.Package);
                SetText(native, NativeMethods.GatewayStationTextModel, message.Model);
                SetText(native, NativeMethods.GatewayStationTextFeatures, message.Features);
                break;
            case GatewayStationKind.RouterConfig:
                SetText(native, NativeMethods.GatewayStationTextRegion, message.Region);
                SetText(native, NativeMethods.GatewayStationTextHwspec, message.Hwspec);
                foreach (uint netId in message.NetIds ?? [])
                {
                    NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_add_net_id(native, netId));
                }

                foreach (GatewayStationJoinRange range in message.JoinEuiRanges)
                {
                    var bounds = new PamojaGatewayStationJoinRange { First = range.First, Last = range.Last };
                    NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_add_join_range(
                        native, in bounds));
                }

                foreach (GatewayStationDataRate? rate in message.DataRates)
                {
                    var entry = new PamojaGatewayStationDataRate
                    {
                        SpreadingFactor = rate?.SpreadingFactor ?? 0,
                        BandwidthHz = rate?.BandwidthHz ?? 0,
                        DownlinkOnly = Flag(rate?.DownlinkOnly ?? false),
                        Defined = Flag(rate is not null),
                    };
                    NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_add_data_rate(
                        native, in entry));
                }

                break;
            case GatewayStationKind.Schedule:
                foreach (GatewayStationBroadcast frame in message.Schedule)
                {
                    var timing = new PamojaGatewayStationBroadcast
                    {
                        DataRate = frame.DataRate,
                        FrequencyHz = frame.FrequencyHz,
                        Priority = frame.Priority,
                        Gpstime = frame.Gpstime ?? 0,
                        HasGpstime = Flag(frame.Gpstime is not null),
                        Rctx = frame.Rctx ?? 0,
                        HasRctx = Flag(frame.Rctx is not null),
                    };
                    byte[] pdu = frame.Pdu ?? [];
                    NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_add_broadcast(
                        native, in timing, pdu, (nuint)pdu.Length));
                }

                break;
            case GatewayStationKind.Other:
                if (string.IsNullOrEmpty(message.Msgtype))
                {
                    throw new ArgumentException(
                        "a message of a kind this build does not model needs Msgtype", nameof(message));
                }

                SetText(native, NativeMethods.GatewayStationTextMsgtype, message.Msgtype);
                break;
        }
    }

    /// <summary>Sets a piece of text on a message being built.</summary>
    private static void SetText(IntPtr native, byte field, string? text)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(text ?? string.Empty);
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_set_text(
            native, field, bytes, (nuint)bytes.Length));
    }

    /// <summary>A native call that sets bytes on a message being built.</summary>
    private delegate PamojaStatus BytesSetter(IntPtr message, ReadOnlySpan<byte> bytes, nuint length);

    /// <summary>Sets bytes on a message being built.</summary>
    private static void SetBytes(IntPtr native, byte[]? bytes, BytesSetter setter)
    {
        byte[] carried = bytes ?? [];
        NativeStatus.ThrowIfError(setter(native, carried, (nuint)carried.Length));
    }

    /// <summary>Reads a piece of text a message carries.</summary>
    private static string Text(IntPtr native, byte field)
    {
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_text(native, field, out IntPtr text));
        return Encoding.UTF8.GetString(OwnedBuffer.Take(text));
    }

    /// <summary>Reads a window from a native call.</summary>
    private static GatewayStationWindow? Managed(PamojaGatewayStationWindow window) =>
        window.Present == 0 ? null : new GatewayStationWindow(window.DataRate, window.FrequencyHz);

    /// <summary>Reads how a packet was heard from a native call.</summary>
    private static GatewayStationLevels Managed(PamojaGatewayStationLevels levels) =>
        new(levels.Rctx, levels.Xtime)
        {
            Gpstime = levels.HasGpstime == 0 ? null : levels.Gpstime,
            Rssi = levels.Rssi,
            Snr = levels.Snr,
        };

    /// <summary>Reads a message a native call produced.</summary>
    private static GatewayStationMessage Read(IntPtr native)
    {
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_fields(
            native, out PamojaGatewayStationFields fields));
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_payload(
            native, out IntPtr payload));
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_options(
            native, out IntPtr options));

        var kind = (GatewayStationKind)fields.Kind;
        bool heard = kind is GatewayStationKind.JoinRequest
            or GatewayStationKind.Uplink
            or GatewayStationKind.Proprietary;
        bool version = kind == GatewayStationKind.Version;
        bool configuration = kind == GatewayStationKind.RouterConfig;

        return new GatewayStationMessage(kind)
        {
            Msgtype = Text(native, NativeMethods.GatewayStationTextMsgtype),
            Mhdr = fields.Mhdr,
            JoinEui = kind == GatewayStationKind.JoinRequest ? fields.JoinEui.ToArray() : null,
            DevEui = kind is GatewayStationKind.JoinRequest
                or GatewayStationKind.Downlink
                or GatewayStationKind.Transmitted
                ? fields.DevEui.ToArray()
                : null,
            DevNonce = fields.DevNonce,
            DevAddr = fields.DevAddr,
            Fctrl = fields.Fctrl,
            Fcnt = fields.Fcnt,
            Fport = fields.HasFport == 0 ? null : fields.Fport,
            Fopts = OwnedBuffer.Take(options),
            Payload = OwnedBuffer.Take(payload),
            Mic = fields.Mic,
            DataRate = fields.DataRate,
            FrequencyHz = fields.FrequencyHz,
            Levels = heard ? Managed(fields.Levels) : null,
            Class = fields.Class,
            Diid = fields.Diid,
            RxDelay = fields.HasRxDelay == 0 ? null : fields.RxDelay,
            Priority = fields.Priority,
            Station = version ? Text(native, NativeMethods.GatewayStationTextStation) : null,
            Firmware = version ? Text(native, NativeMethods.GatewayStationTextFirmware) : null,
            Package = version ? Text(native, NativeMethods.GatewayStationTextPackage) : null,
            Model = version ? Text(native, NativeMethods.GatewayStationTextModel) : null,
            Protocol = version ? fields.Protocol : null,
            Features = version ? Text(native, NativeMethods.GatewayStationTextFeatures) : null,
            NetIds = configuration && fields.FiltersNetworks != 0 ? NetIds(native, fields.NetIdCount) : null,
            JoinEuiRanges = JoinRanges(native, fields.JoinRangeCount),
            Region = configuration ? Text(native, NativeMethods.GatewayStationTextRegion) : null,
            Hwspec = configuration ? Text(native, NativeMethods.GatewayStationTextHwspec) : null,
            MaxEirp = configuration ? fields.MaxEirp : null,
            FreqMinHz = configuration ? fields.FreqMinHz : null,
            FreqMaxHz = configuration ? fields.FreqMaxHz : null,
            DataRates = DataRates(native, fields.DataRateCount),
            Rx1 = Managed(fields.Rx1),
            Rx2 = Managed(fields.Rx2),
            PingSlot = Managed(fields.PingSlot),
            Xtime = fields.HasXtime == 0 ? null : fields.Xtime,
            Rctx = fields.HasRctx == 0 ? null : fields.Rctx,
            Gpstime = fields.HasGpstime == 0 ? null : fields.Gpstime,
            Txtime = fields.HasTxtime == 0 ? null : fields.Txtime,
            Schedule = Broadcasts(native, fields.BroadcastCount),
        };
    }

    /// <summary>Reads the networks a configuration names.</summary>
    private static uint[] NetIds(IntPtr native, nuint count)
    {
        uint[] ids = new uint[checked((int)count)];
        for (int index = 0; index < ids.Length; index++)
        {
            NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_net_id(
                native, (nuint)index, out ids[index]));
        }

        return ids;
    }

    /// <summary>Reads the join identifier ranges a configuration names.</summary>
    private static GatewayStationJoinRange[] JoinRanges(IntPtr native, nuint count)
    {
        var ranges = new GatewayStationJoinRange[checked((int)count)];
        for (int index = 0; index < ranges.Length; index++)
        {
            NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_join_range(
                native, (nuint)index, out PamojaGatewayStationJoinRange range));
            ranges[index] = new GatewayStationJoinRange(range.First, range.Last);
        }

        return ranges;
    }

    /// <summary>Reads a configuration's data-rate table.</summary>
    private static GatewayStationDataRate?[] DataRates(IntPtr native, nuint count)
    {
        var rates = new GatewayStationDataRate?[checked((int)count)];
        for (int index = 0; index < rates.Length; index++)
        {
            NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_data_rate(
                native, (nuint)index, out PamojaGatewayStationDataRate rate));
            rates[index] = rate.Defined == 0
                ? null
                : new GatewayStationDataRate(rate.SpreadingFactor, rate.BandwidthHz, rate.DownlinkOnly != 0);
        }

        return rates;
    }

    /// <summary>Reads the frames a schedule carries.</summary>
    private static GatewayStationBroadcast[] Broadcasts(IntPtr native, nuint count)
    {
        var frames = new GatewayStationBroadcast[checked((int)count)];
        for (int index = 0; index < frames.Length; index++)
        {
            NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_broadcast(
                native, (nuint)index, out PamojaGatewayStationBroadcast frame, out IntPtr pdu));
            frames[index] = new GatewayStationBroadcast(OwnedBuffer.Take(pdu), frame.DataRate, frame.FrequencyHz)
            {
                Priority = frame.Priority,
                Gpstime = frame.HasGpstime == 0 ? null : frame.Gpstime,
                Rctx = frame.HasRctx == 0 ? null : frame.Rctx,
            };
        }

        return frames;
    }
}
