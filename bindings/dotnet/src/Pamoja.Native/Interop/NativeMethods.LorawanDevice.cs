using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for a LoRaWAN Class A end device, mirroring <c>pamoja.h</c>
/// one-to-one.
/// </summary>
public static partial class NativeMethods
{
    /// <summary>How many bytes a saved device state takes.</summary>
    public const int LorawanSavedLen = 1589;

    /// <summary>Makes a device that joins over the air.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_new(
        IntPtr plan,
        IntPtr credentials,
        in PamojaLorawanDeviceSettings settings,
        uint fcntUp,
        byte hasFcntDown,
        uint fcntDown,
        out IntPtr outDevice);

    /// <summary>Makes a device activated by personalization.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_personalized(
        IntPtr plan,
        IntPtr session,
        in PamojaLorawanDeviceSettings settings,
        uint fcntUp,
        byte hasFcntDown,
        uint fcntDown,
        out IntPtr outDevice);

    /// <summary>Fills in the settings of a typical node with a radio's output power range.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_device_settings(
        sbyte minOutputDbm,
        sbyte maxOutputDbm,
        out PamojaLorawanDeviceSettings outSettings);

    /// <summary>Releases a device handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_end_device_free(IntPtr device);

    /// <summary>Reads why a device's last call failed.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_error(
        IntPtr device,
        out PamojaLorawanDeviceError outError);

    /// <summary>Builds a join request.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_join(
        IntPtr device,
        ushort devNonce,
        ulong nowUs,
        out IntPtr outFrame,
        out PamojaLorawanTransmission outTransmission);

    /// <summary>Builds an uplink carrying a payload.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_send(
        IntPtr device,
        byte port,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        byte confirmed,
        ulong nowUs,
        out IntPtr outFrame,
        out PamojaLorawanTransmission outTransmission);

    /// <summary>Builds an uplink with no payload.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_send_empty(
        IntPtr device,
        ulong nowUs,
        out IntPtr outFrame,
        out PamojaLorawanTransmission outTransmission);

    /// <summary>Sends the last uplink again.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_repeat(
        IntPtr device,
        ulong nowUs,
        out IntPtr outFrame,
        out PamojaLorawanTransmission outTransmission);

    /// <summary>Reads a frame heard in a receive window.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_heard(
        IntPtr device,
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        sbyte snrDb,
        out PamojaLorawanHeard outHeard,
        out IntPtr outPayload);

    /// <summary>Says what comes next once both receive windows closed empty.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_nothing_heard(
        IntPtr device,
        ulong nowUs,
        out PamojaLorawanNext outNext);

    /// <summary>Sets what the device reports its battery as.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_set_battery(
        IntPtr device,
        byte kind,
        byte level);

    /// <summary>Asks the network, with the next uplink, how well it hears the device.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_request_link_check(IntPtr device);

    /// <summary>Asks the network, with the next uplink, for the time.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_request_device_time(IntPtr device);

    /// <summary>Reads where a device stands.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_status(
        IntPtr device,
        out PamojaLorawanEndDeviceStatus outStatus);

    /// <summary>Reads one of the channels a device may send on.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_channel(
        IntPtr device,
        ushort position,
        out ushort outIndex,
        out PamojaLorawanChannel outChannel);

    /// <summary>Saves a joined device's state.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_save(
        IntPtr device,
        ulong nowUs,
        Span<byte> outSaved,
        nuint len);

    /// <summary>Puts a saved state back on a device.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_end_device_resume(
        IntPtr device,
        ReadOnlySpan<byte> saved,
        nuint savedLen,
        ulong nowUs);
}

/// <summary>What a device's radio can do, mirroring <c>PamojaLorawanDeviceSettings</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanDeviceSettings
{
    /// <summary>The lowest usable frequency, in hertz.</summary>
    public uint LowestHz;

    /// <summary>The highest usable frequency, in hertz.</summary>
    public uint HighestHz;

    /// <summary>The seed for the random choices of channel and retry delay.</summary>
    public uint Seed;

    /// <summary>The link layer revision code.</summary>
    public byte Version;

    /// <summary><c>1</c> to let the network manage the data rate and power.</summary>
    public byte Adr;

    /// <summary>The lowest output power, conducted, in dBm.</summary>
    public sbyte MinOutputDbm;

    /// <summary>The highest output power, conducted, in dBm.</summary>
    public sbyte MaxOutputDbm;

    /// <summary>The antenna gain less cable and connector losses, in dB.</summary>
    public sbyte AntennaGainDb;

    /// <summary><c>1</c> to hold the device to the region's duty cycles.</summary>
    public byte RegionalDutyCycle;

    /// <summary><c>1</c> to size payloads for a path through a relay.</summary>
    public byte BehindRepeater;
}

/// <summary>When and where to listen, mirroring <c>PamojaLorawanWindow</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanWindow
{
    /// <summary>How long after the transmission the window opens, in microseconds.</summary>
    public uint DelayUs;

    /// <summary>The carrier, in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>The LoRa settings to listen with.</summary>
    public PamojaLoraLink Link;

    /// <summary>The downlink data rate.</summary>
    public byte DataRate;
}

/// <summary>A frame to put on the air, mirroring <c>PamojaLorawanTransmission</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanTransmission
{
    /// <summary>How long the frame holds the air, in microseconds.</summary>
    public ulong AirtimeUs;

    /// <summary>The carrier, in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>The first receive window.</summary>
    public PamojaLorawanWindow Rx1;

    /// <summary>The second receive window.</summary>
    public PamojaLorawanWindow Rx2;

    /// <summary>The LoRa settings.</summary>
    public PamojaLoraLink Link;

    /// <summary>The data rate.</summary>
    public byte DataRate;

    /// <summary>The power to ask of the radio, conducted, in dBm.</summary>
    public sbyte OutputDbm;

    /// <summary><c>1</c> if the payload went out in this frame.</summary>
    public byte CarriesPayload;
}

/// <summary>What a heard frame was, mirroring <c>PamojaLorawanHeard</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanHeard
{
    /// <summary>The address the device is on the network by.</summary>
    public uint DevAddr;

    /// <summary>For a time answer, whole seconds since the GPS epoch.</summary>
    public uint GpsSeconds;

    /// <summary><c>0</c> for a join, <c>1</c> for data.</summary>
    public byte Kind;

    /// <summary><c>1</c> if the payload arrived on an application port.</summary>
    public byte HasPort;

    /// <summary>The application port.</summary>
    public byte Port;

    /// <summary><c>1</c> if the network acknowledged the confirmed uplink.</summary>
    public byte Acknowledged;

    /// <summary><c>1</c> if the network asked for an acknowledgment.</summary>
    public byte Confirmed;

    /// <summary><c>1</c> if the network has more waiting.</summary>
    public byte MorePending;

    /// <summary><c>1</c> if the downlink answered a link check.</summary>
    public byte HasLinkCheck;

    /// <summary>The link check margin, in dB.</summary>
    public byte MarginDb;

    /// <summary>How many gateways heard the check.</summary>
    public byte Gateways;

    /// <summary><c>1</c> if the downlink answered a time request.</summary>
    public byte HasDeviceTime;

    /// <summary>The fraction of a second, in 256ths.</summary>
    public byte Fraction;
}

/// <summary>What to do next, mirroring <c>PamojaLorawanNext</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanNext
{
    /// <summary>The earliest time to send, in microseconds.</summary>
    public ulong NotBeforeUs;

    /// <summary>One of the next-step codes.</summary>
    public byte Kind;
}

/// <summary>Why a device's last call failed, mirroring <c>PamojaLorawanDeviceError</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanDeviceError
{
    /// <summary>For a wait, the earliest time to try again, in microseconds.</summary>
    public ulong UntilUs;

    /// <summary>For a payload too long, the most the frame carries.</summary>
    public uint Max;

    /// <summary>One of the device error codes.</summary>
    public byte Kind;

    /// <summary>For a data rate error, the data rate.</summary>
    public byte DataRate;

    /// <summary>For a state error, why.</summary>
    public byte State;

    /// <summary>For a format error, the format.</summary>
    public byte Format;
}

/// <summary>Where a device stands, mirroring <c>PamojaLorawanEndDeviceStatus</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanEndDeviceStatus
{
    /// <summary>The device address.</summary>
    public uint DevAddr;

    /// <summary>The next uplink frame counter.</summary>
    public uint FcntUp;

    /// <summary>The last downlink frame counter accepted.</summary>
    public uint FcntDown;

    /// <summary>Where the second receive window listens, in hertz.</summary>
    public uint Rx2FrequencyHz;

    /// <summary>The delay to the first receive window, in microseconds.</summary>
    public uint ReceiveDelayUs;

    /// <summary>The lowest frequency used, in hertz.</summary>
    public uint LowestHz;

    /// <summary>The highest frequency used, in hertz.</summary>
    public uint HighestHz;

    /// <summary>How many channels are enabled.</summary>
    public ushort ChannelCount;

    /// <summary><c>1</c> once joined.</summary>
    public byte Joined;

    /// <summary>The data rate of the next uplink.</summary>
    public byte DataRate;

    /// <summary><c>1</c> once a downlink was accepted.</summary>
    public byte HasFcntDown;

    /// <summary>NbTrans.</summary>
    public byte Transmissions;

    /// <summary>The second receive window's data rate.</summary>
    public byte Rx2DataRate;
}

/// <summary>A channel a device may send on, mirroring <c>PamojaLorawanChannel</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanChannel
{
    /// <summary>Where uplinks go out, in hertz.</summary>
    public uint UplinkHz;

    /// <summary>Where the first receive window listens, in hertz.</summary>
    public uint DownlinkHz;

    /// <summary>The slowest data rate.</summary>
    public byte MinDataRate;

    /// <summary>The fastest data rate.</summary>
    public byte MaxDataRate;
}
