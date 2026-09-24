using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the network side of one site, mirroring <c>pamoja.h</c>
/// one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. Every part must
/// be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>A device joined, and its accept is in the event.</summary>
    public const byte GatewayNetworkJoined = 0;

    /// <summary>A session frame arrived, decrypted into the caller buffer.</summary>
    public const byte GatewayNetworkData = 1;

    /// <summary>The frame belongs to a device this site never granted.</summary>
    public const byte GatewayNetworkForeign = 2;

    /// <summary>The first receive window answers on the frequency the uplink arrived on.</summary>
    public const byte GatewayNetworkRx1Same = 0;

    /// <summary>The first receive window answers on a run of downlink channels.</summary>
    public const byte GatewayNetworkRx1Downstream = 1;

    /// <summary>Returns the windows a network answers in by default.</summary>
    [LibraryImport(Library)]
    public static partial PamojaGatewayNetworkWindows pamoja_gateway_network_windows_default();

    /// <summary>Opens the network side of a site on a channel plan.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_network_open(
        IntPtr plan,
        uint netId,
        PamojaGatewayNetworkWindows windows,
        uint firstDevAddr,
        out IntPtr outNetwork);

    /// <summary>Admits a device, so a join request signed with its key is accepted.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_network_register(
        IntPtr network,
        ReadOnlySpan<byte> devEui,
        ReadOnlySpan<byte> appEui,
        ReadOnlySpan<byte> appKey,
        byte version);

    /// <summary>Reads a packet the gateway forwarded.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_network_uplink(
        IntPtr network,
        PamojaGatewayRxpk packet,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        Span<byte> buffer,
        nuint capacity,
        out PamojaGatewayNetworkEvent outEvent);

    /// <summary>Builds a downlink for a device, encrypted with its session.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_network_answer(
        IntPtr network,
        uint devAddr,
        PamojaGatewayNetworkSlot slot,
        byte fport,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        Span<byte> buffer,
        nuint capacity,
        out PamojaGatewayTxpk outTxpk,
        out nuint outLen);

    /// <summary>Takes the devices relays reported hearing and could not verify.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_network_notices(
        IntPtr network,
        Span<PamojaGatewayNetworkNotice> outNotices,
        nuint capacity,
        out nuint outLen);

    /// <summary>Builds a downlink carrying MAC commands.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_network_command(
        IntPtr network,
        uint devAddr,
        PamojaGatewayNetworkSlot slot,
        ReadOnlySpan<PamojaLorawanMacCommand> commands,
        nuint commandsLen,
        Span<byte> buffer,
        nuint capacity,
        out PamojaGatewayTxpk outTxpk,
        out nuint outLen);

    /// <summary>Builds the command that tells a relay to trust a device.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_network_trust_command(
        IntPtr network,
        uint devAddr,
        byte index,
        byte reloadRate,
        byte bucketSize,
        out PamojaLorawanMacCommand outCommand);

    /// <summary>Releases a network.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_gateway_network_free(IntPtr network);
}

/// <summary>A device a relay heard and could not verify, mirroring <c>PamojaGatewayNetworkNotice</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayNetworkNotice
{
    /// <summary>The relay that heard it.</summary>
    public uint Relay;

    /// <summary>The address the wake-on-radio frame named.</summary>
    public uint DevAddr;

    /// <summary>The frame's signal strength in dBm.</summary>
    public short RssiDbm;

    /// <summary>Its signal-to-noise ratio in dB.</summary>
    public sbyte SnrDb;
}
