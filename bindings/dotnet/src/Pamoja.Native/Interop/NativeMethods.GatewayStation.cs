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
