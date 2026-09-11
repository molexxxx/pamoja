using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the LoRaWAN gateway protocols of the pamoja C ABI,
/// mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. Every part must
/// be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The protocol version every datagram starts with.</summary>
    public const byte GatewayProtocolVersion = 2;

    /// <summary>The length of a gateway's unique identifier.</summary>
    public const int GatewayEuiLen = 8;

    /// <summary>The gateway forwarding what it heard.</summary>
    public const byte GatewayPushData = 0x00;

    /// <summary>The server acknowledging a PUSH_DATA.</summary>
    public const byte GatewayPushAck = 0x01;

    /// <summary>The gateway holding its route open.</summary>
    public const byte GatewayPullData = 0x02;

    /// <summary>The server sending a packet to transmit.</summary>
    public const byte GatewayPullResp = 0x03;

    /// <summary>The server acknowledging a PULL_DATA.</summary>
    public const byte GatewayPullAck = 0x04;

    /// <summary>The gateway reporting what became of a PULL_RESP.</summary>
    public const byte GatewayTxAck = 0x05;

    /// <summary>A LoRa packet, whose settings are in the link.</summary>
    public const byte GatewayModulationLora = 0;

    /// <summary>An FSK packet, whose bitrate is in its bitrate field.</summary>
    public const byte GatewayModulationFsk = 1;

    /// <summary>Creates an empty uplink for a PUSH_DATA to carry.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_gateway_uplink_new();

    /// <summary>Adds a packet the gateway heard.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_uplink_add_rxpk(
        IntPtr uplink,
        PamojaGatewayRxpk packet,
        ReadOnlySpan<byte> payload,
        nuint payloadLen);

    /// <summary>Sets the gateway's status report on an uplink.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_uplink_set_stat(
        IntPtr uplink,
        PamojaGatewayStat status);

    /// <summary>Releases an uplink that will not be sent.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_gateway_uplink_free(IntPtr uplink);

    /// <summary>Builds the PUSH_DATA that forwards an uplink, taking the uplink over.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_push_data(
        ushort token,
        ReadOnlySpan<byte> gateway,
        IntPtr uplink,
        out IntPtr outPacket);

    /// <summary>Builds the PUSH_ACK that answers a PUSH_DATA.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_gateway_push_ack(ushort token);

    /// <summary>Builds the PULL_DATA that holds a route open.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_pull_data(
        ushort token,
        ReadOnlySpan<byte> gateway,
        out IntPtr outPacket);

    /// <summary>Builds the PULL_ACK that answers a PULL_DATA.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_gateway_pull_ack(ushort token);

    /// <summary>Builds the PULL_RESP that asks a gateway to transmit.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_pull_resp(
        ushort token,
        PamojaGatewayTxpk transmit,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out IntPtr outPacket);

    /// <summary>Builds the TX_ACK that reports what became of a PULL_RESP.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_tx_ack(
        ushort token,
        ReadOnlySpan<byte> gateway,
        byte status,
        out IntPtr outPacket);

    /// <summary>Reads a datagram.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_packet_parse(
        ReadOnlySpan<byte> bytes,
        nuint len,
        out IntPtr outPacket);

    /// <summary>Returns which kind of datagram a packet is, or 255 for a null packet.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_gateway_packet_kind(IntPtr packet);

    /// <summary>Returns a packet's token.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_gateway_packet_token(IntPtr packet);

    /// <summary>Writes the gateway's identifier, for the datagrams that carry one.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_gateway_packet_gateway(IntPtr packet, Span<byte> outGateway);

    /// <summary>Returns how many packets a PUSH_DATA forwards.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_gateway_packet_rxpk_count(IntPtr packet);

    /// <summary>Reads one of the packets a PUSH_DATA forwards.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_gateway_packet_rxpk(
        IntPtr packet,
        nuint index,
        out PamojaGatewayRxpk outRxpk);

    /// <summary>Returns a pointer to one forwarded packet's payload.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_gateway_packet_rxpk_payload(IntPtr packet, nuint index);

    /// <summary>Returns the length of one forwarded packet's payload.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_gateway_packet_rxpk_payload_len(IntPtr packet, nuint index);

    /// <summary>Reads the gateway's status report, when a PUSH_DATA carries one.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_gateway_packet_stat(IntPtr packet, out PamojaGatewayStat outStat);

    /// <summary>Reads what a PULL_RESP asks the gateway to transmit.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_gateway_packet_txpk(IntPtr packet, out PamojaGatewayTxpk outTxpk);

    /// <summary>Returns a pointer to the payload a PULL_RESP carries.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_gateway_packet_txpk_payload(IntPtr packet);

    /// <summary>Returns the length of the payload a PULL_RESP carries.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_gateway_packet_txpk_payload_len(IntPtr packet);

    /// <summary>Returns what a TX_ACK reports, or 255 when the packet is not one.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_gateway_packet_tx_status(IntPtr packet);

    /// <summary>Builds the acknowledgment a server owes a datagram.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_gateway_packet_acknowledgment(IntPtr packet, out IntPtr outPacket);

    /// <summary>Writes a packet as the datagram to send.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_gateway_packet_to_buffer(IntPtr packet);

    /// <summary>Releases a packet.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_gateway_packet_free(IntPtr packet);
}
