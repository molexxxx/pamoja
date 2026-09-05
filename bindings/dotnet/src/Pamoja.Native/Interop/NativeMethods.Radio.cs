using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the radio capabilities of the pamoja C ABI - the
/// LoRa link budget, LoRaWAN framing, mesh packets, and mesh routing - mirroring
/// <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>A duplicate-cache size for a caller with no reason to choose one.</summary>
    public const int MeshSeenDefaultCapacity = 64;

    /// <summary>A routing table size for a caller with no reason to choose one.</summary>
    public const int RoutingDefaultCapacity = 64;

    /// <summary>The largest payload a single mesh frame can carry, in bytes.</summary>
    public const int MeshPayloadMax = 236;

    /// <summary>The largest mesh frame, in bytes, header and checksum included.</summary>
    public const int MeshFrameMax = 250;

    /// <summary>How many bytes of a mesh frame are header.</summary>
    public const int MeshHeaderLen = 12;

    /// <summary>The destination address that means every node.</summary>
    public const uint MeshBroadcast = 0xFFFFFFFF;

    /// <summary>The hop limit a mesh frame starts with unless one is set.</summary>
    public const byte MeshDefaultHopLimit = 3;

    /// <summary>The largest LoRaWAN application payload, in bytes.</summary>
    public const int LorawanPayloadMax = 243;

    /// <summary>The length of a LoRaWAN key, in bytes.</summary>
    public const int LorawanKeyLen = 16;

    /// <summary>The length of a LoRaWAN EUI, in bytes.</summary>
    public const int LorawanEuiLen = 8;

    /// <summary>Returns LoRa link settings with the LoRa defaults filled in.</summary>
    [LibraryImport(Library)]
    public static partial PamojaLoraLink pamoja_lora_link_default(
        byte spreadingFactor,
        uint bandwidthHz);

    /// <summary>Returns the duration of one symbol on a link, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_lora_symbol_time_us(PamojaLoraLink link);

    /// <summary>Returns the time on air of a payload, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_lora_airtime_us(PamojaLoraLink link, nuint payloadLen);

    /// <summary>Returns the silence a duty-cycle limit forces after a transmission.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_lora_min_off_time_us(
        PamojaLoraLink link,
        nuint payloadLen,
        uint dutyCyclePermille);

    /// <summary>Builds a mesh frame addressed to one node.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mesh_frame_new(
        uint src,
        uint dst,
        ushort id,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out IntPtr outFrame);

    /// <summary>Builds a mesh frame addressed to every node.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mesh_frame_broadcast(
        uint src,
        ushort id,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out IntPtr outFrame);

    /// <summary>Parses a mesh frame received off a radio.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mesh_frame_parse(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out IntPtr outFrame);

    /// <summary>Sets the number of relays a frame may still take.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mesh_frame_set_hop_limit(IntPtr frame, byte hopLimit);

    /// <summary>Returns the protocol version a frame declares.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_mesh_frame_version(IntPtr frame);

    /// <summary>Returns the address of the node a frame came from.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_mesh_frame_src(IntPtr frame);

    /// <summary>Returns the address a frame is addressed to.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_mesh_frame_dst(IntPtr frame);

    /// <summary>Returns the sequence number a frame carries.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_mesh_frame_id(IntPtr frame);

    /// <summary>Returns how many further relays a frame may take.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_mesh_frame_hop_limit(IntPtr frame);

    /// <summary>Reports whether a frame is addressed to every node.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_mesh_frame_is_broadcast(IntPtr frame);

    /// <summary>Returns a pointer to the payload a frame carries.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mesh_frame_payload(IntPtr frame);

    /// <summary>Returns the length of the payload a frame carries.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_mesh_frame_payload_len(IntPtr frame);

    /// <summary>Returns a pointer to the whole frame as it goes on the air.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mesh_frame_bytes(IntPtr frame);

    /// <summary>Returns the length of the whole frame.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_mesh_frame_bytes_len(IntPtr frame);

    /// <summary>Returns the same frame with one hop spent, ready to forward.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_mesh_frame_relayed(IntPtr frame, out IntPtr outFrame);

    /// <summary>Releases a mesh frame handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mesh_frame_free(IntPtr frame);

    /// <summary>Computes the CRC-16 a mesh frame carries.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_mesh_crc16(ReadOnlySpan<byte> data, nuint dataLen);

    /// <summary>Creates an empty duplicate cache of the size given.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mesh_seen_new(nuint capacity);

    /// <summary>Reports whether a packet is remembered, without recording it.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_mesh_seen_contains(IntPtr cache, uint src, ushort id);

    /// <summary>Records a packet and reports whether it was new.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_mesh_seen_record(IntPtr cache, uint src, ushort id);

    /// <summary>Returns how many packets a duplicate cache remembers.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_mesh_seen_capacity(IntPtr cache);

    /// <summary>Releases a duplicate cache handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mesh_seen_free(IntPtr cache);

    /// <summary>Creates an empty routing table of the size given.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_router_new(uint address, nuint capacity);

    /// <summary>Returns the address a router answers for.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_router_address(IntPtr router);

    /// <summary>Learns a route from a packet that arrived.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_router_observe(IntPtr router, uint origin, uint via, ushort cost);

    /// <summary>Returns the neighbour on the way to a node.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_router_next_hop(IntPtr router, uint dst, out uint outNextHop);

    /// <summary>Returns what the known route to a node costs.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_router_cost(IntPtr router, uint dst, out ushort outCost);

    /// <summary>Returns the whole route to a node.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_router_route(IntPtr router, uint dst, out PamojaRoute outRoute);

    /// <summary>Decides what to do with a packet bound for a node.</summary>
    [LibraryImport(Library)]
    public static partial PamojaForward pamoja_router_forward(
        IntPtr router,
        uint dst,
        out uint outNextHop);

    /// <summary>Forgets the route to a node.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_router_forget(IntPtr router, uint dst);

    /// <summary>Returns how many routes a table currently holds.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_router_len(IntPtr router);

    /// <summary>Returns how many routes a table can hold.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_router_capacity(IntPtr router);

    /// <summary>Releases a routing table handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_router_free(IntPtr router);

    /// <summary>Creates a LoRaWAN session from a device address and its keys.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_session_new(
        uint devAddr,
        ReadOnlySpan<byte> nwkSKey,
        nuint nwkSKeyLen,
        ReadOnlySpan<byte> appSKey,
        nuint appSKeyLen,
        out IntPtr outSession);

    /// <summary>Returns the device address a session is bound to.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_lorawan_session_dev_addr(IntPtr session);

    /// <summary>Encodes an uplink data frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_session_encode_uplink(
        IntPtr session,
        uint fcnt,
        byte fport,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        ReadOnlySpan<byte> fopts,
        nuint foptsLen,
        PamojaLorawanFlags flags,
        out IntPtr outFrame);

    /// <summary>Encodes a downlink data frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_session_encode_downlink(
        IntPtr session,
        uint fcnt,
        byte fport,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        ReadOnlySpan<byte> fopts,
        nuint foptsLen,
        PamojaLorawanFlags flags,
        out IntPtr outFrame);

    /// <summary>Verifies a received frame, then decrypts it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_session_decode(
        IntPtr session,
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        uint fcnt,
        out IntPtr outRx);

    /// <summary>Releases a session handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_session_free(IntPtr session);

    /// <summary>Returns the direction a decoded frame travelled.</summary>
    [LibraryImport(Library)]
    public static partial PamojaLorawanDirection pamoja_lorawan_rx_direction(IntPtr rx);

    /// <summary>Returns the device address a decoded frame carries.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_lorawan_rx_dev_addr(IntPtr rx);

    /// <summary>Returns the low 16 bits of the frame counter.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_lorawan_rx_fcnt(IntPtr rx);

    /// <summary>Reports whether a decoded frame asks to be acknowledged.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_lorawan_rx_confirmed(IntPtr rx);

    /// <summary>Reports whether a decoded frame takes part in adaptive data rate.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_lorawan_rx_adr(IntPtr rx);

    /// <summary>Reports whether a decoded frame acknowledges the last confirmed one.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_lorawan_rx_ack(IntPtr rx);

    /// <summary>Reports whether the network has more downlink data waiting.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_lorawan_rx_fpending(IntPtr rx);

    /// <summary>Returns the port a decoded frame was sent on.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_lorawan_rx_fport(IntPtr rx, out byte outFport);

    /// <summary>Returns a pointer to the frame options a decoded frame carries.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_lorawan_rx_fopts(IntPtr rx);

    /// <summary>Returns the length of the frame options a decoded frame carries.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_lorawan_rx_fopts_len(IntPtr rx);

    /// <summary>Returns a pointer to the decrypted payload of a decoded frame.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_lorawan_rx_payload(IntPtr rx);

    /// <summary>Returns the length of the decrypted payload of a decoded frame.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_lorawan_rx_payload_len(IntPtr rx);

    /// <summary>Releases a decoded frame handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_rx_free(IntPtr rx);

    /// <summary>Creates a device from the root credentials activation uses.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_device_new(
        ReadOnlySpan<byte> devEui,
        nuint devEuiLen,
        ReadOnlySpan<byte> appEui,
        nuint appEuiLen,
        ReadOnlySpan<byte> appKey,
        nuint appKeyLen,
        out IntPtr outDevice);

    /// <summary>Builds the join request a device broadcasts to activate.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_device_join_request(
        IntPtr device,
        ushort devNonce,
        out IntPtr outFrame);

    /// <summary>Turns the join accept a network sent into the settings it grants.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_device_accept_join(
        IntPtr device,
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        ushort devNonce,
        out IntPtr outAccept);

    /// <summary>Releases a device handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_device_free(IntPtr device);

    /// <summary>Returns the device address a join grants.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_lorawan_join_accept_dev_addr(IntPtr accept);

    /// <summary>Returns the identifier of the network that accepted a join.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_lorawan_join_accept_net_id(IntPtr accept);

    /// <summary>Returns the downlink settings byte a join grants.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_lorawan_join_accept_dl_settings(IntPtr accept);

    /// <summary>Returns the delay before the first receive window, in seconds.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_lorawan_join_accept_rx_delay(IntPtr accept);

    /// <summary>Takes the activated session a join grants.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_join_accept_session(
        IntPtr accept,
        out IntPtr outSession);

    /// <summary>Releases an accepted join handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_join_accept_free(IntPtr accept);

    /// <summary>Reads a frame far enough to route it, without any key.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_header_parse(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out PamojaLorawanHeader outHeader);

    /// <summary>Verifies a join-request and reads the identifiers out of it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_join_request_parse(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        ReadOnlySpan<byte> appKey,
        nuint appKeyLen,
        out IntPtr outRequest);

    /// <summary>Copies the device identifier out of a verified join-request.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_lorawan_join_request_dev_eui(
        IntPtr request,
        Span<byte> outDevEui);

    /// <summary>Copies the application identifier out of a verified join-request.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_lorawan_join_request_app_eui(
        IntPtr request,
        Span<byte> outAppEui);

    /// <summary>Returns the nonce a verified join-request carried.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_lorawan_join_request_dev_nonce(IntPtr request);

    /// <summary>Releases a verified join-request handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_join_request_free(IntPtr request);

    /// <summary>Builds the signed join-accept a network sends to admit a device.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_grant_accept(
        PamojaLorawanGrant grant,
        ReadOnlySpan<byte> cflist,
        nuint cflistLen,
        ReadOnlySpan<byte> appKey,
        nuint appKeyLen,
        ushort devNonce,
        out IntPtr outFrame);

    /// <summary>Derives the session a grant activates.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_grant_session(
        PamojaLorawanGrant grant,
        ReadOnlySpan<byte> cflist,
        nuint cflistLen,
        ReadOnlySpan<byte> appKey,
        nuint appKeyLen,
        ushort devNonce,
        out IntPtr outSession);
}
