using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the LoRa Basics Station protocol, mirroring <c>pamoja.h</c>
/// one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. Every part must
/// be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>A join request the station heard.</summary>
    public const byte GatewayStationJoinRequest = 0;

    /// <summary>A data frame the station heard.</summary>
    public const byte GatewayStationUplink = 1;

    /// <summary>A frame of a kind this protocol does not describe, carried whole.</summary>
    public const byte GatewayStationProprietary = 2;

    /// <summary>What the station reports about itself when a session opens.</summary>
    public const byte GatewayStationVersion = 3;

    /// <summary>How the server tells the station to configure its radios.</summary>
    public const byte GatewayStationRouterConfig = 4;

    /// <summary>A frame the server asks the station to transmit.</summary>
    public const byte GatewayStationDownlink = 5;

    /// <summary>Frames the server asks the station to transmit to a group.</summary>
    public const byte GatewayStationSchedule = 6;

    /// <summary>What became of a frame the station was asked to transmit.</summary>
    public const byte GatewayStationTransmitted = 7;

    /// <summary>The clock the two keep between them.</summary>
    public const byte GatewayStationTimeSync = 8;

    /// <summary>A kind this build does not model, readable only as its text.</summary>
    public const byte GatewayStationOther = 9;

    /// <summary>The station software a version reports.</summary>
    public const byte GatewayStationTextStation = 0;

    /// <summary>The firmware a version reports.</summary>
    public const byte GatewayStationTextFirmware = 1;

    /// <summary>The package a version reports.</summary>
    public const byte GatewayStationTextPackage = 2;

    /// <summary>The hardware model a version reports.</summary>
    public const byte GatewayStationTextModel = 3;

    /// <summary>What a version says the station can do.</summary>
    public const byte GatewayStationTextFeatures = 4;

    /// <summary>The region a configuration names.</summary>
    public const byte GatewayStationTextRegion = 5;

    /// <summary>The concentrator a configuration is written for.</summary>
    public const byte GatewayStationTextHwspec = 6;

    /// <summary>The word a message calls itself on the wire.</summary>
    public const byte GatewayStationTextMsgtype = 7;

    /// <summary>Builds a message of any kind from its fixed fields.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_gateway_station_message_new(
        in PamojaGatewayStationFields fields);

    /// <summary>Returns a piece of text a message carries.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_text(
        IntPtr message,
        byte field,
        out IntPtr outText);

    /// <summary>Sets a piece of text a message carries.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_set_text(
        IntPtr message,
        byte field,
        ReadOnlySpan<byte> text,
        nuint textLen);

    /// <summary>Sets the bytes a message carries.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_set_payload(
        IntPtr message,
        ReadOnlySpan<byte> bytes,
        nuint bytesLen);

    /// <summary>Sets the frame options a data frame carries.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_set_options(
        IntPtr message,
        ReadOnlySpan<byte> bytes,
        nuint bytesLen);

    /// <summary>Adds a network whose data frames a configuration forwards.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_add_net_id(
        IntPtr message,
        uint netId);

    /// <summary>Adds a range of join identifiers a configuration forwards.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_add_join_range(
        IntPtr message,
        in PamojaGatewayStationJoinRange range);

    /// <summary>Adds the next number of a configuration's data-rate table.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_add_data_rate(
        IntPtr message,
        in PamojaGatewayStationDataRate rate);

    /// <summary>Adds a frame to a schedule.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_add_broadcast(
        IntPtr message,
        in PamojaGatewayStationBroadcast frame,
        ReadOnlySpan<byte> pdu,
        nuint pduLen);

    /// <summary>Reads one network a configuration names.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_net_id(
        IntPtr message,
        nuint index,
        out uint outNetId);

    /// <summary>Reads one join identifier range a configuration names.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_join_range(
        IntPtr message,
        nuint index,
        out PamojaGatewayStationJoinRange outRange);

    /// <summary>Reads one number of a configuration's data-rate table.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_data_rate(
        IntPtr message,
        nuint index,
        out PamojaGatewayStationDataRate outRate);

    /// <summary>Reads one frame of a schedule.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_broadcast(
        IntPtr message,
        nuint index,
        out PamojaGatewayStationBroadcast outFrame,
        out IntPtr outPdu);

    /// <summary>Reads the request a station sent to find its network server.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_discovery_parse(
        ReadOnlySpan<byte> text,
        nuint textLen,
        Span<byte> outRouter);

    /// <summary>Writes the answer that sends a station to its session's websocket.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_router_accepted(
        ReadOnlySpan<byte> router,
        ReadOnlySpan<byte> muxs,
        ReadOnlySpan<byte> uri,
        nuint uriLen,
        out IntPtr outText);

    /// <summary>Writes the answer that refuses a station, saying why.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_router_refused(
        ReadOnlySpan<byte> router,
        ReadOnlySpan<byte> error,
        nuint errorLen,
        out IntPtr outText);

    /// <summary>Builds a station clock value from its radio unit, run, and microseconds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_xtime(
        byte unit,
        byte session,
        ulong micros,
        out long outValue);

    /// <summary>Takes a station clock value apart.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_xtime_parts(
        long value,
        out PamojaGatewayStationXtime outParts);

    /// <summary>Reads the identities a discovery answer names.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_router_identities(
        ReadOnlySpan<byte> text,
        nuint textLen,
        out PamojaGatewayStationRouterIds outIds);

    /// <summary>Reads a frame the radio heard into the message that reports it.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_gateway_station_heard(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        byte dataRate,
        uint frequencyHz,
        PamojaGatewayStationLevels levels);

    /// <summary>Reads a message that arrived over the websocket.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_gateway_station_message_parse(
        ReadOnlySpan<byte> text,
        nuint textLen);

    /// <summary>Writes a message as the websocket carries it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_json(
        IntPtr message,
        out IntPtr outText);

    /// <summary>Returns which kind a message is.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_gateway_station_message_kind(IntPtr message);

    /// <summary>Reads the fields a message carries.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_fields(
        IntPtr message,
        out PamojaGatewayStationFields outFields);

    /// <summary>Returns the bytes a message carries.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_payload(
        IntPtr message,
        out IntPtr outPayload);

    /// <summary>Returns the frame options a data frame carries.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_message_options(
        IntPtr message,
        out IntPtr outOptions);

    /// <summary>Releases a message.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_gateway_station_message_free(IntPtr message);

    /// <summary>Writes the request a station sends to find its network server.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_discovery(
        ReadOnlySpan<byte> router,
        out IntPtr outText);

    /// <summary>Reads the answer a discovery endpoint gives.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_router_parse(
        ReadOnlySpan<byte> text,
        nuint textLen,
        out IntPtr outUri,
        out IntPtr outError);

    /// <summary>Writes an identifier in the ID6 form the protocol prefers.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_gateway_station_id6(
        ReadOnlySpan<byte> eui,
        out IntPtr outText);

    /// <summary>Reads an identifier written in any form the protocol accepts.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_gateway_station_eui_of(
        string text,
        Span<byte> outEui);
}
