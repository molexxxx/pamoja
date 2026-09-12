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

/// <summary>The answer a discovery endpoint gives.</summary>
/// <param name="Uri">The websocket to open, or <c>null</c> when the station was refused.</param>
/// <param name="Error">Why the station was refused, or <c>null</c> when it was not.</param>
public sealed record GatewayStationRouter(string? Uri, string? Error);

/// <summary>A message either side of a session sends.</summary>
/// <param name="Kind">Which kind of message.</param>
/// <remarks>
/// Every message names its kind, and carries the fields that kind uses. The rest read as
/// <c>null</c>.
/// </remarks>
public sealed record GatewayStationMessage(GatewayStationKind Kind)
{
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

    /// <summary>The payload, still encrypted, or the frame a downlink transmits.</summary>
    public byte[] Payload { get; init; } = [];

    /// <summary>The message integrity code.</summary>
    public int Mic { get; init; }

    /// <summary>The data rate it arrived at, or is to be sent at.</summary>
    public byte DataRate { get; init; }

    /// <summary>The frequency in hertz.</summary>
    public uint FrequencyHz { get; init; }

    /// <summary>How it was heard, for the kinds a station sends up.</summary>
    public GatewayStationLevels? Levels { get; init; }

    /// <summary>Which class of downlink this is.</summary>
    public byte Class { get; init; }

    /// <summary>The identifier a transmission report carries back.</summary>
    public long Diid { get; init; }

    /// <summary>The delay before the first receive window, in seconds.</summary>
    public byte? RxDelay { get; init; }

    /// <summary>How urgent a downlink is.</summary>
    public byte Priority { get; init; }

    /// <summary>The message as the websocket carries it.</summary>
    public string Json { get; init; } = string.Empty;
}

/// <summary>The LoRa Basics Station protocol, from either side.</summary>
/// <remarks>
/// A station and its network server exchange JSON messages over a websocket the caller owns,
/// so nothing here opens a socket: split a frame the radio heard with <see cref="Heard"/>,
/// send what it writes, and read whatever arrives with <see cref="Parse"/>.
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
        if (native == IntPtr.Zero)
        {
            throw new PamojaException(NativeStatus.LastError() ?? "the frame is not one a station sends up");
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

    /// <summary>Reads a message that arrived over the websocket.</summary>
    /// <param name="text">The message text.</param>
    /// <returns>The message.</returns>
    /// <exception cref="PamojaException">It is not a message this protocol defines.</exception>
    public static GatewayStationMessage Parse(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        byte[] bytes = Encoding.UTF8.GetBytes(text);
        IntPtr native = NativeMethods.pamoja_gateway_station_message_parse(bytes, (nuint)bytes.Length);
        if (native == IntPtr.Zero)
        {
            throw new PamojaException(NativeStatus.LastError() ?? "the text is not this protocol");
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

    /// <summary>Writes the request a station sends to find its network server.</summary>
    /// <param name="router">The station asking, eight bytes.</param>
    /// <returns>The text to send on <see cref="DiscoveryPath"/>.</returns>
    /// <exception cref="ArgumentException"><paramref name="router"/> is not eight bytes.</exception>
    public static string Discovery(ReadOnlySpan<byte> router)
    {
        if (router.Length != PamojaEui.Length)
        {
            throw new ArgumentException($"router must be exactly {PamojaEui.Length} bytes", nameof(router));
        }

        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_discovery(router, out IntPtr text));
        return Encoding.UTF8.GetString(OwnedBuffer.Take(text));
    }

    /// <summary>Reads the answer a discovery endpoint gives.</summary>
    /// <param name="text">The answer text.</param>
    /// <returns>Where to connect, or why the station was refused.</returns>
    /// <exception cref="PamojaException">The answer is not this protocol.</exception>
    public static GatewayStationRouter RouterParse(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        byte[] bytes = Encoding.UTF8.GetBytes(text);
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_router_parse(
            bytes, (nuint)bytes.Length, out IntPtr uri, out IntPtr error));

        string opened = Encoding.UTF8.GetString(OwnedBuffer.Take(uri));
        string refused = Encoding.UTF8.GetString(OwnedBuffer.Take(error));
        return new GatewayStationRouter(
            opened.Length == 0 ? null : opened,
            refused.Length == 0 ? null : refused);
    }

    /// <summary>Writes an identifier in the ID6 form the protocol prefers.</summary>
    /// <param name="eui">The identifier, eight bytes.</param>
    /// <returns>The identifier, such as <c>1:203:405:607</c>.</returns>
    /// <exception cref="ArgumentException"><paramref name="eui"/> is not eight bytes.</exception>
    public static string Id6(ReadOnlySpan<byte> eui)
    {
        if (eui.Length != PamojaEui.Length)
        {
            throw new ArgumentException($"eui must be exactly {PamojaEui.Length} bytes", nameof(eui));
        }

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

    /// <summary>Writes how a packet was heard for the native call.</summary>
    private static PamojaGatewayStationLevels Native(GatewayStationLevels? levels) => levels is null
        ? default
        : new PamojaGatewayStationLevels
        {
            Rctx = levels.Rctx,
            Xtime = levels.Xtime,
            Gpstime = levels.Gpstime ?? 0,
            HasGpstime = (byte)(levels.Gpstime is null ? 0 : 1),
            Rssi = levels.Rssi,
            Snr = levels.Snr,
        };

    /// <summary>Reads a message a native call produced.</summary>
    private static GatewayStationMessage Read(IntPtr native)
    {
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_fields(
            native, out PamojaGatewayStationFields fields));
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_json(
            native, out IntPtr text));
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_payload(
            native, out IntPtr payload));
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_station_message_options(
            native, out IntPtr options));

        var kind = (GatewayStationKind)fields.Kind;
        bool heard = kind is GatewayStationKind.JoinRequest
            or GatewayStationKind.Uplink
            or GatewayStationKind.Proprietary;

        return new GatewayStationMessage(kind)
        {
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
            Json = Encoding.UTF8.GetString(OwnedBuffer.Take(text)),
        };
    }

    /// <summary>Reads how a packet was heard from a native call.</summary>
    private static GatewayStationLevels Managed(PamojaGatewayStationLevels levels) =>
        new(levels.Rctx, levels.Xtime)
        {
            Gpstime = levels.HasGpstime == 0 ? null : levels.Gpstime,
            Rssi = levels.Rssi,
            Snr = levels.Snr,
        };
}
