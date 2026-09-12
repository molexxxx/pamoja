using Pamoja.Lora;
using Pamoja.Native.Interop;

using NativeStatus = Pamoja.Native.Interop.Status;

namespace Pamoja.Gateway;

/// <summary>Which kind of datagram.</summary>
public enum GatewayPacketKind : byte
{
    /// <summary>The gateway forwarding what it heard.</summary>
    PushData = 0x00,

    /// <summary>The server acknowledging a PUSH_DATA.</summary>
    PushAck = 0x01,

    /// <summary>The gateway holding its route open through any address translation.</summary>
    PullData = 0x02,

    /// <summary>The server sending a packet to transmit.</summary>
    PullResp = 0x03,

    /// <summary>The server acknowledging a PULL_DATA.</summary>
    PullAck = 0x04,

    /// <summary>The gateway reporting what became of a PULL_RESP.</summary>
    TxAck = 0x05,
}

/// <summary>What the CRC of a received packet said.</summary>
public enum GatewayCrc
{
    /// <summary>The CRC checked.</summary>
    Ok,

    /// <summary>The CRC failed.</summary>
    Failed,

    /// <summary>The packet carried no CRC.</summary>
    Absent,
}

/// <summary>What became of a downlink the server asked for.</summary>
public enum GatewayTxStatus : byte
{
    /// <summary>It was scheduled, which the protocol writes as <c>NONE</c>.</summary>
    None = 0,

    /// <summary>It arrived too late to schedule.</summary>
    TooLate = 1,

    /// <summary>Its timestamp is too far ahead.</summary>
    TooEarly = 2,

    /// <summary>Another packet was already scheduled then.</summary>
    CollisionPacket = 3,

    /// <summary>A beacon was already scheduled then.</summary>
    CollisionBeacon = 4,

    /// <summary>The radio chain cannot reach that frequency.</summary>
    TxFreq = 5,

    /// <summary>The gateway cannot transmit at that power.</summary>
    TxPower = 6,

    /// <summary>A GPS timestamp was asked for while the GPS is unlocked.</summary>
    GpsUnlocked = 7,
}

/// <summary>A packet the gateway heard, with the metadata the protocol carries beside it.</summary>
/// <param name="FrequencyHz">The carrier it arrived on, in hertz.</param>
/// <param name="Payload">The packet itself.</param>
public sealed record GatewayRxpk(uint FrequencyHz, byte[] Payload)
{
    /// <summary>The spreading factor, bandwidth, and coding rate, for a LoRa packet.</summary>
    public LoraLink? Link { get; init; }

    /// <summary>The bitrate in bits per second, for an FSK packet.</summary>
    public uint? BitrateBps { get; init; }

    /// <summary>What the CRC said.</summary>
    public GatewayCrc Crc { get; init; } = GatewayCrc.Ok;

    /// <summary>The received signal strength in dBm.</summary>
    public double RssiDbm { get; init; }

    /// <summary>The signal-to-noise ratio in dB, for a LoRa packet.</summary>
    public double? SnrDb { get; init; }

    /// <summary>The concentrator channel it arrived on.</summary>
    public byte Channel { get; init; }

    /// <summary>The radio chain it arrived on.</summary>
    public byte RfChain { get; init; }

    /// <summary>The concentrator's own timestamp of the reception, in microseconds.</summary>
    public uint? TimestampMicros { get; init; }

    /// <summary>When it arrived, in microseconds since 1970-01-01 UTC.</summary>
    public ulong? ReceivedAtMicros { get; init; }

    /// <summary>When it arrived on the GPS clock, in milliseconds since 6 January 1980.</summary>
    public ulong? GpsMillis { get; init; }
}

/// <summary>A gateway's own status report.</summary>
public sealed record GatewayStat
{
    /// <summary>The gateway's clock, in seconds since 1970-01-01 UTC.</summary>
    public ulong? TimeSeconds { get; init; }

    /// <summary>Its latitude in degrees, north positive.</summary>
    public double? LatitudeDeg { get; init; }

    /// <summary>Its longitude in degrees, east positive.</summary>
    public double? LongitudeDeg { get; init; }

    /// <summary>Its altitude in meters.</summary>
    public int? AltitudeM { get; init; }

    /// <summary>How many packets its radio received.</summary>
    public uint Received { get; init; }

    /// <summary>How many of those had a good CRC.</summary>
    public uint ReceivedOk { get; init; }

    /// <summary>How many it forwarded.</summary>
    public uint Forwarded { get; init; }

    /// <summary>What share of its datagrams were acknowledged, as a percentage.</summary>
    public double AcknowledgedPercent { get; init; }

    /// <summary>How many downlink datagrams it received.</summary>
    public uint Downlinks { get; init; }

    /// <summary>How many packets it transmitted.</summary>
    public uint Transmitted { get; init; }
}

/// <summary>A packet the server asks the gateway to transmit.</summary>
/// <param name="FrequencyHz">The carrier to transmit on, in hertz.</param>
/// <param name="Payload">The packet itself.</param>
public sealed record GatewayTxpk(uint FrequencyHz, byte[] Payload)
{
    /// <summary>The spreading factor, bandwidth, and coding rate, for a LoRa packet.</summary>
    public LoraLink? Link { get; init; }

    /// <summary>The bitrate in bits per second, for an FSK packet.</summary>
    public uint? BitrateBps { get; init; }

    /// <summary>Whether to transmit at once, which ignores the timestamps.</summary>
    public bool Immediate { get; init; }

    /// <summary>The concentrator timestamp to transmit at, in microseconds.</summary>
    public uint? TimestampMicros { get; init; }

    /// <summary>The GPS time to transmit at, in milliseconds since 6 January 1980.</summary>
    public ulong? GpsMillis { get; init; }

    /// <summary>The radio chain to transmit from.</summary>
    public byte RfChain { get; init; }

    /// <summary>The power to transmit at, in dBm.</summary>
    public sbyte PowerDbm { get; init; } = 14;

    /// <summary>The FSK frequency deviation in hertz.</summary>
    public uint? FrequencyDeviationHz { get; init; }

    /// <summary>Whether to invert the LoRa polarity, as a LoRaWAN downlink is sent.</summary>
    public bool InvertPolarity { get; init; }

    /// <summary>How long a preamble to send, in symbols.</summary>
    public ushort? PreambleSymbols { get; init; }

    /// <summary>Whether to leave the physical CRC off, as LoRaWAN downlinks are.</summary>
    public bool WithoutCrc { get; init; }
}

/// <summary>One datagram of the protocol, with the fields its kind carries.</summary>
/// <param name="Kind">Which kind of datagram.</param>
/// <param name="Token">The token that pairs a datagram with its answer.</param>
public sealed record GatewayPacket(GatewayPacketKind Kind, ushort Token)
{
    /// <summary>
    /// The gateway's identifier, as sixteen hexadecimal digits, for the kinds that carry one.
    /// </summary>
    public string? GatewayEui { get; init; }

    /// <summary>The packets a PUSH_DATA forwards.</summary>
    public IReadOnlyList<GatewayRxpk> Packets { get; init; } = [];

    /// <summary>The report a PUSH_DATA carries.</summary>
    public GatewayStat? Status { get; init; }

    /// <summary>What a PULL_RESP asks the gateway to transmit.</summary>
    public GatewayTxpk? Transmit { get; init; }

    /// <summary>What a TX_ACK reports.</summary>
    public GatewayTxStatus? TxStatus { get; init; }
}

/// <summary>The Semtech UDP packet forwarder protocol, from either side.</summary>
/// <remarks>
/// A gateway hears packets from every node in range and hands them to a network server, which
/// hands back the packets to transmit. This builds and reads the datagrams between them and
/// leaves the socket, the keepalive, and the scheduling to the program that owns them.
/// </remarks>
public static class Gateway
{
    /// <summary>The port a packet forwarder sends to by convention.</summary>
    public const int DefaultPort = 1700;

    /// <summary>Names a status the way the protocol writes it.</summary>
    /// <param name="status">The status.</param>
    /// <returns>Its word, such as <c>COLLISION_PACKET</c>, or <c>NONE</c> when nothing failed.</returns>
    public static string NameOf(GatewayTxStatus status) => status switch
    {
        GatewayTxStatus.TooLate => "TOO_LATE",
        GatewayTxStatus.TooEarly => "TOO_EARLY",
        GatewayTxStatus.CollisionPacket => "COLLISION_PACKET",
        GatewayTxStatus.CollisionBeacon => "COLLISION_BEACON",
        GatewayTxStatus.TxFreq => "TX_FREQ",
        GatewayTxStatus.TxPower => "TX_POWER",
        GatewayTxStatus.GpsUnlocked => "GPS_UNLOCKED",
        _ => "NONE",
    };

    /// <summary>Writes a datagram to send over a socket.</summary>
    /// <param name="packet">The datagram to send.</param>
    /// <returns>Its bytes.</returns>
    /// <exception cref="PamojaException">The datagram is missing a field its kind carries.</exception>
    public static byte[] Encode(GatewayPacket packet)
    {
        ArgumentNullException.ThrowIfNull(packet);

        IntPtr native = Build(packet);
        try
        {
            return OwnedBuffer.Take(NativeMethods.pamoja_gateway_packet_to_buffer(native));
        }
        finally
        {
            NativeMethods.pamoja_gateway_packet_free(native);
        }
    }

    /// <summary>Reads a datagram that arrived.</summary>
    /// <param name="datagram">The bytes as they arrived.</param>
    /// <returns>The datagram.</returns>
    /// <exception cref="PamojaException">It is not a datagram this protocol defines.</exception>
    public static GatewayPacket Parse(ReadOnlySpan<byte> datagram)
    {
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_packet_parse(
            datagram, (nuint)datagram.Length, out IntPtr native));
        try
        {
            return Read(native);
        }
        finally
        {
            NativeMethods.pamoja_gateway_packet_free(native);
        }
    }

    /// <summary>Returns the acknowledgment a server owes a datagram.</summary>
    /// <param name="packet">The datagram that arrived.</param>
    /// <returns>
    /// The PUSH_ACK or PULL_ACK to send back, or <c>null</c> for a datagram that needs none. A
    /// PULL_RESP is answered with a TX_ACK, which names the gateway, so a gateway builds that
    /// one itself.
    /// </returns>
    public static GatewayPacket? Acknowledgment(GatewayPacket packet)
    {
        ArgumentNullException.ThrowIfNull(packet);

        IntPtr native = Build(packet);
        try
        {
            if (!NativeMethods.pamoja_gateway_packet_acknowledgment(native, out IntPtr answer))
            {
                return null;
            }

            try
            {
                return Read(answer);
            }
            finally
            {
                NativeMethods.pamoja_gateway_packet_free(answer);
            }
        }
        finally
        {
            NativeMethods.pamoja_gateway_packet_free(native);
        }
    }

    /// <summary>Builds the native packet a datagram describes.</summary>
    /// <param name="packet">The datagram.</param>
    /// <returns>A native handle the caller releases.</returns>
    private static IntPtr Build(GatewayPacket packet)
    {
        switch (packet.Kind)
        {
            case GatewayPacketKind.PushData:
            {
                IntPtr uplink = NativeMethods.pamoja_gateway_uplink_new();
                try
                {
                    foreach (GatewayRxpk heard in packet.Packets)
                    {
                        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_uplink_add_rxpk(
                            uplink, Native(heard), heard.Payload, (nuint)heard.Payload.Length));
                    }

                    if (packet.Status is GatewayStat status)
                    {
                        NativeStatus.ThrowIfError(
                            NativeMethods.pamoja_gateway_uplink_set_stat(uplink, Native(status)));
                    }

                    PamojaStatus built = NativeMethods.pamoja_gateway_push_data(
                        packet.Token, Identifier(packet), uplink, out IntPtr native);
                    NativeStatus.ThrowIfError(built);
                    uplink = IntPtr.Zero;
                    return native;
                }
                finally
                {
                    if (uplink != IntPtr.Zero)
                    {
                        NativeMethods.pamoja_gateway_uplink_free(uplink);
                    }
                }
            }

            case GatewayPacketKind.PushAck:
                return NativeMethods.pamoja_gateway_push_ack(packet.Token);

            case GatewayPacketKind.PullAck:
                return NativeMethods.pamoja_gateway_pull_ack(packet.Token);

            case GatewayPacketKind.PullData:
            {
                NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_pull_data(
                    packet.Token, Identifier(packet), out IntPtr native));
                return native;
            }

            case GatewayPacketKind.PullResp:
            {
                GatewayTxpk request = packet.Transmit
                    ?? throw new PamojaException("a PullResp carries what to transmit");
                NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_pull_resp(
                    packet.Token,
                    Native(request),
                    request.Payload,
                    (nuint)request.Payload.Length,
                    out IntPtr native));
                return native;
            }

            default:
            {
                NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_tx_ack(
                    packet.Token,
                    Identifier(packet),
                    (byte)(packet.TxStatus ?? GatewayTxStatus.None),
                    out IntPtr native));
                return native;
            }
        }
    }

    /// <summary>Reads a native packet.</summary>
    /// <param name="native">The handle to read.</param>
    /// <returns>The datagram.</returns>
    private static GatewayPacket Read(IntPtr native)
    {
        var kind = (GatewayPacketKind)NativeMethods.pamoja_gateway_packet_kind(native);
        ushort token = NativeMethods.pamoja_gateway_packet_token(native);

        byte[] identifier = new byte[NativeMethods.GatewayEuiLen];
        string? gateway = NativeMethods.pamoja_gateway_packet_gateway(native, identifier)
            ? Convert.ToHexString(identifier).ToLowerInvariant()
            : null;

        var heard = new List<GatewayRxpk>();
        for (nuint index = 0; index < NativeMethods.pamoja_gateway_packet_rxpk_count(native); index++)
        {
            if (!NativeMethods.pamoja_gateway_packet_rxpk(native, index, out PamojaGatewayRxpk packet))
            {
                continue;
            }

            heard.Add(Managed(packet, Payload(
                NativeMethods.pamoja_gateway_packet_rxpk_payload(native, index),
                NativeMethods.pamoja_gateway_packet_rxpk_payload_len(native, index))));
        }

        GatewayStat? status = NativeMethods.pamoja_gateway_packet_stat(native, out PamojaGatewayStat report)
            ? Managed(report)
            : null;
        GatewayTxpk? transmit = NativeMethods.pamoja_gateway_packet_txpk(native, out PamojaGatewayTxpk request)
            ? Managed(request, Payload(
                NativeMethods.pamoja_gateway_packet_txpk_payload(native),
                NativeMethods.pamoja_gateway_packet_txpk_payload_len(native)))
            : null;
        byte reported = NativeMethods.pamoja_gateway_packet_tx_status(native);

        return new GatewayPacket(kind, token)
        {
            GatewayEui = gateway,
            Packets = heard,
            Status = status,
            Transmit = transmit,
            TxStatus = reported == byte.MaxValue ? null : (GatewayTxStatus)reported,
        };
    }

    /// <summary>Reads a gateway identifier a datagram must carry.</summary>
    /// <param name="packet">The datagram.</param>
    /// <returns>Its eight bytes.</returns>
    /// <exception cref="PamojaException">It names no identifier, or not sixteen hex digits.</exception>
    private static byte[] Identifier(GatewayPacket packet)
    {
        string? text = packet.GatewayEui;
        if (text is null)
        {
            throw new PamojaException("this kind of datagram carries the gateway's identifier");
        }

        try
        {
            byte[] bytes = Convert.FromHexString(text);
            if (bytes.Length != NativeMethods.GatewayEuiLen)
            {
                throw new PamojaException(
                    $"a gateway identifier is sixteen hexadecimal digits, not {text}");
            }

            return bytes;
        }
        catch (FormatException error)
        {
            throw new PamojaException(
                $"a gateway identifier is sixteen hexadecimal digits, not {text}", error);
        }
    }

    /// <summary>Copies a payload out of a native packet.</summary>
    /// <param name="data">Where the bytes are.</param>
    /// <param name="length">How many there are.</param>
    /// <returns>The payload.</returns>
    private static byte[] Payload(IntPtr data, nuint length)
    {
        byte[] bytes = new byte[checked((int)length)];
        if (bytes.Length > 0)
        {
            System.Runtime.InteropServices.Marshal.Copy(data, bytes, 0, bytes.Length);
        }

        return bytes;
    }

    /// <summary>Describes a forwarded packet for the C ABI.</summary>
    /// <param name="heard">The packet.</param>
    /// <returns>Its metadata as the C ABI carries it.</returns>
    internal static PamojaGatewayRxpk Native(GatewayRxpk heard) => new()
    {
        ReceivedAtUs = heard.ReceivedAtMicros ?? 0,
        GpsMillis = heard.GpsMillis ?? 0,
        FrequencyHz = heard.FrequencyHz,
        TimestampUs = heard.TimestampMicros ?? 0,
        Link = NativeLink(heard.Link),
        BitrateBps = heard.BitrateBps ?? 0,
        RssiCentiDbm = Centi(heard.RssiDbm),
        SnrCentiDb = Centi(heard.SnrDb ?? 0),
        Channel = heard.Channel,
        RfChain = heard.RfChain,
        Crc = heard.Crc switch
        {
            GatewayCrc.Failed => -1,
            GatewayCrc.Absent => 0,
            _ => 1,
        },
        Modulation = heard.BitrateBps is null
            ? NativeMethods.GatewayModulationLora
            : NativeMethods.GatewayModulationFsk,
        HasReceivedAt = Flag(heard.ReceivedAtMicros is not null),
        HasGpsMillis = Flag(heard.GpsMillis is not null),
        HasTimestamp = Flag(heard.TimestampMicros is not null),
        HasSnr = Flag(heard.SnrDb is not null),
    };

    /// <summary>Reads a forwarded packet from the C ABI.</summary>
    /// <param name="heard">Its metadata.</param>
    /// <param name="payload">Its payload.</param>
    /// <returns>The packet.</returns>
    private static GatewayRxpk Managed(PamojaGatewayRxpk heard, byte[] payload) =>
        new(heard.FrequencyHz, payload)
        {
            Link = heard.Modulation == NativeMethods.GatewayModulationFsk ? null : ManagedLink(heard.Link),
            BitrateBps = heard.Modulation == NativeMethods.GatewayModulationFsk ? heard.BitrateBps : null,
            Crc = heard.Crc switch
            {
                -1 => GatewayCrc.Failed,
                0 => GatewayCrc.Absent,
                _ => GatewayCrc.Ok,
            },
            RssiDbm = heard.RssiCentiDbm / 100.0,
            SnrDb = heard.HasSnr != 0 ? heard.SnrCentiDb / 100.0 : null,
            Channel = heard.Channel,
            RfChain = heard.RfChain,
            TimestampMicros = heard.HasTimestamp != 0 ? heard.TimestampUs : null,
            ReceivedAtMicros = heard.HasReceivedAt != 0 ? heard.ReceivedAtUs : null,
            GpsMillis = heard.HasGpsMillis != 0 ? heard.GpsMillis : null,
        };

    /// <summary>Describes a status report for the C ABI.</summary>
    /// <param name="report">The report.</param>
    /// <returns>It as the C ABI carries it.</returns>
    private static PamojaGatewayStat Native(GatewayStat report) => new()
    {
        TimeS = report.TimeSeconds ?? 0,
        LatitudeDeg = report.LatitudeDeg ?? 0,
        LongitudeDeg = report.LongitudeDeg ?? 0,
        AcknowledgedPercent = report.AcknowledgedPercent,
        AltitudeM = report.AltitudeM ?? 0,
        Received = report.Received,
        ReceivedOk = report.ReceivedOk,
        Forwarded = report.Forwarded,
        Downlinks = report.Downlinks,
        Transmitted = report.Transmitted,
        HasTime = Flag(report.TimeSeconds is not null),
        HasPosition = Flag(report.LatitudeDeg is not null && report.LongitudeDeg is not null),
        HasAltitude = Flag(report.AltitudeM is not null),
    };

    /// <summary>Reads a status report from the C ABI.</summary>
    /// <param name="report">The report as the C ABI carries it.</param>
    /// <returns>The report.</returns>
    private static GatewayStat Managed(PamojaGatewayStat report) => new()
    {
        TimeSeconds = report.HasTime != 0 ? report.TimeS : null,
        LatitudeDeg = report.HasPosition != 0 ? report.LatitudeDeg : null,
        LongitudeDeg = report.HasPosition != 0 ? report.LongitudeDeg : null,
        AltitudeM = report.HasAltitude != 0 ? report.AltitudeM : null,
        Received = report.Received,
        ReceivedOk = report.ReceivedOk,
        Forwarded = report.Forwarded,
        AcknowledgedPercent = report.AcknowledgedPercent,
        Downlinks = report.Downlinks,
        Transmitted = report.Transmitted,
    };

    /// <summary>Describes a transmission request for the C ABI.</summary>
    /// <param name="request">The request.</param>
    /// <returns>It as the C ABI carries it.</returns>
    private static PamojaGatewayTxpk Native(GatewayTxpk request) => new()
    {
        GpsMillis = request.GpsMillis ?? 0,
        FrequencyHz = request.FrequencyHz,
        TimestampUs = request.TimestampMicros ?? 0,
        Link = NativeLink(request.Link),
        BitrateBps = request.BitrateBps ?? 0,
        FrequencyDeviationHz = request.FrequencyDeviationHz ?? 0,
        PreambleSymbols = request.PreambleSymbols ?? 0,
        RfChain = request.RfChain,
        PowerDbm = request.PowerDbm,
        Modulation = request.BitrateBps is null
            ? NativeMethods.GatewayModulationLora
            : NativeMethods.GatewayModulationFsk,
        Immediate = Flag(request.Immediate || request.TimestampMicros is null),
        InvertPolarity = Flag(request.InvertPolarity),
        WithoutCrc = Flag(request.WithoutCrc),
        HasTimestamp = Flag(request.TimestampMicros is not null),
        HasGpsMillis = Flag(request.GpsMillis is not null),
        HasDeviation = Flag(request.FrequencyDeviationHz is not null),
        HasPreamble = Flag(request.PreambleSymbols is not null),
    };

    /// <summary>Reads a transmission request from the C ABI.</summary>
    /// <param name="request">The request as the C ABI carries it.</param>
    /// <param name="payload">Its payload.</param>
    /// <returns>The request.</returns>
    internal static GatewayTxpk Managed(PamojaGatewayTxpk request, byte[] payload) =>
        new(request.FrequencyHz, payload)
        {
            Link = request.Modulation == NativeMethods.GatewayModulationFsk ? null : ManagedLink(request.Link),
            BitrateBps = request.Modulation == NativeMethods.GatewayModulationFsk ? request.BitrateBps : null,
            Immediate = request.Immediate != 0,
            TimestampMicros = request.HasTimestamp != 0 ? request.TimestampUs : null,
            GpsMillis = request.HasGpsMillis != 0 ? request.GpsMillis : null,
            RfChain = request.RfChain,
            PowerDbm = request.PowerDbm,
            FrequencyDeviationHz = request.HasDeviation != 0 ? request.FrequencyDeviationHz : null,
            InvertPolarity = request.InvertPolarity != 0,
            PreambleSymbols = request.HasPreamble != 0 ? request.PreambleSymbols : null,
            WithoutCrc = request.WithoutCrc != 0,
        };

    /// <summary>Describes link settings for the C ABI, defaulting an FSK packet's unused link.</summary>
    /// <param name="link">The settings, or <c>null</c> for an FSK packet.</param>
    /// <returns>The settings as the C ABI carries them.</returns>
    internal static PamojaLoraLink NativeLink(LoraLink? link)
    {
        LoraLink settings = link ?? new LoraLink(7, 125_000);
        return new PamojaLoraLink
        {
            BandwidthHz = settings.BandwidthHz,
            PreambleSymbols = settings.PreambleSymbols,
            SpreadingFactor = settings.SpreadingFactor,
            CodingRateDenominator = settings.CodingRateDenominator,
            ExplicitHeader = Flag(settings.ExplicitHeader),
            Crc = Flag(settings.Crc),
        };
    }

    /// <summary>Reads link settings from the C ABI.</summary>
    /// <param name="link">The settings as the C ABI carries them.</param>
    /// <returns>The settings.</returns>
    internal static LoraLink ManagedLink(PamojaLoraLink link)
    {
        LoraLink settings = new LoraLink(link.SpreadingFactor, link.BandwidthHz)
            .WithCodingRate(link.CodingRateDenominator)
            .WithPreamble(link.PreambleSymbols);
        if (link.ExplicitHeader == 0)
        {
            settings = settings.WithImplicitHeader();
        }

        return link.Crc == 0 ? settings.WithoutCrc() : settings;
    }

    /// <summary>Describes a flag the way the C ABI carries it.</summary>
    /// <param name="set">Whether the flag is set.</param>
    /// <returns><c>1</c> when set, <c>0</c> otherwise.</returns>
    private static byte Flag(bool set) => set ? (byte)1 : (byte)0;

    /// <summary>Resolves a number of decibels to hundredths of a decibel.</summary>
    /// <param name="db">The value in decibels.</param>
    /// <returns>The nearest hundredth.</returns>
    private static int Centi(double db) => (int)Math.Round(db * 100, MidpointRounding.AwayFromZero);
}
