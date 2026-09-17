using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for ChirpStack uplink events, mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
public static partial class NativeMethods
{
    /// <summary>The field naming the uplink's deduplication identifier.</summary>
    public const byte ChirpstackDeduplicationId = 0;

    /// <summary>The field naming when the uplink was received.</summary>
    public const byte ChirpstackTime = 1;

    /// <summary>The field naming the application.</summary>
    public const byte ChirpstackApplicationId = 2;

    /// <summary>The field naming the device.</summary>
    public const byte ChirpstackDeviceName = 3;

    /// <summary>Reads an uplink event from the JSON ChirpStack published.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_chirpstack_uplink_parse(
        ReadOnlySpan<byte> text,
        nuint textLen,
        out IntPtr outUplink);

    /// <summary>Releases an uplink event handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_chirpstack_uplink_free(IntPtr uplink);

    /// <summary>Reads the scalar fields of an uplink event.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_chirpstack_uplink_summary(
        IntPtr uplink,
        out PamojaChirpstackUplinkSummary outSummary);

    /// <summary>Reads one of an uplink event's text fields.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_chirpstack_uplink_text(IntPtr uplink, byte field);

    /// <summary>Copies out the application payload an uplink carried.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_chirpstack_uplink_data(IntPtr uplink, out IntPtr outData);

    /// <summary>Reads one gateway that heard an uplink.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_chirpstack_uplink_reception(
        IntPtr uplink,
        uint index,
        out PamojaChirpstackReception outReception);

    /// <summary>Finds the gateway that heard an uplink best.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_chirpstack_uplink_best_reception(IntPtr uplink, out uint outIndex);

    /// <summary>Builds the MQTT topic an application's uplink events are published on.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial IntPtr pamoja_chirpstack_uplink_topic(string applicationId);
}

/// <summary>The scalar fields of an uplink event, mirroring <c>PamojaChirpstackUplinkSummary</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaChirpstackUplinkSummary
{
    /// <summary>The device's address.</summary>
    public uint DevAddr;

    /// <summary>The uplink frame counter.</summary>
    public uint Fcnt;

    /// <summary>The carrier, in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>How many gateways heard it.</summary>
    public uint ReceptionCount;

    /// <summary>The device EUI, most-significant byte first.</summary>
    public PamojaEui DevEui;

    /// <summary><c>1</c> when the event names the device's address.</summary>
    public byte HasDevAddr;

    /// <summary><c>1</c> when adaptive data rate was on.</summary>
    public byte Adr;

    /// <summary>The data rate.</summary>
    public byte DataRate;

    /// <summary><c>1</c> when the frame carried a port.</summary>
    public byte HasFport;

    /// <summary>The application port.</summary>
    public byte Fport;

    /// <summary><c>1</c> for a confirmed uplink.</summary>
    public byte Confirmed;

    /// <summary><c>1</c> when the event names the carrier.</summary>
    public byte HasFrequency;
}

/// <summary>One gateway that heard an uplink, mirroring <c>PamojaChirpstackReception</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaChirpstackReception
{
    /// <summary>The received signal strength, in dBm.</summary>
    public int RssiDbm;

    /// <summary>The signal-to-noise ratio, in dB.</summary>
    public float SnrDb;

    /// <summary>The gateway's EUI, most-significant byte first.</summary>
    public PamojaEui Gateway;
}
