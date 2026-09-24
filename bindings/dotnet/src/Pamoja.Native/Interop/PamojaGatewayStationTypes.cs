using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// How a station heard a packet, mirroring <c>PamojaGatewayStationLevels</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayStationLevels
{
    /// <summary>The radio the packet arrived on, which an answer goes back out on.</summary>
    public long Rctx;

    /// <summary>The station clock, in microseconds.</summary>
    public long Xtime;

    /// <summary>The GPS time, when <c>HasGpstime</c>.</summary>
    public long Gpstime;

    /// <summary><c>1</c> when the station has a GPS time.</summary>
    public byte HasGpstime;

    /// <summary>The received signal strength, in dBm.</summary>
    public double Rssi;

    /// <summary>The signal-to-noise ratio, in dB.</summary>
    public double Snr;
}

/// <summary>
/// A receive window, or the ping slot of a class B downlink, mirroring
/// <c>PamojaGatewayStationWindow</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayStationWindow
{
    /// <summary>The data rate.</summary>
    public byte DataRate;

    /// <summary>The frequency in hertz.</summary>
    public uint FrequencyHz;

    /// <summary><c>1</c> when the message names this window at all.</summary>
    public byte Present;
}

/// <summary>
/// One number of a configuration's data-rate table, mirroring
/// <c>PamojaGatewayStationDataRate</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayStationDataRate
{
    /// <summary>The spreading factor, 0 for FSK.</summary>
    public byte SpreadingFactor;

    /// <summary>The bandwidth in hertz.</summary>
    public uint BandwidthHz;

    /// <summary><c>1</c> when the rate is used only for downlinks.</summary>
    public byte DownlinkOnly;

    /// <summary><c>1</c> when the table defines this number at all.</summary>
    public byte Defined;
}

/// <summary>
/// A range of join identifiers, both ends included, mirroring
/// <c>PamojaGatewayStationJoinRange</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayStationJoinRange
{
    /// <summary>The first identifier, read as a big-endian number.</summary>
    public ulong First;

    /// <summary>The last identifier, read the same way.</summary>
    public ulong Last;
}

/// <summary>
/// One frame of a schedule apart from its bytes, mirroring
/// <c>PamojaGatewayStationBroadcast</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayStationBroadcast
{
    /// <summary>The data rate to transmit at.</summary>
    public byte DataRate;

    /// <summary>The frequency to transmit on, in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>How urgent it is.</summary>
    public byte Priority;

    /// <summary>When to transmit it, in microseconds since the GPS epoch, when <c>HasGpstime</c>.</summary>
    public long Gpstime;

    /// <summary><c>1</c> when it names a GPS time.</summary>
    public byte HasGpstime;

    /// <summary>The radio to transmit on, when <c>HasRctx</c>.</summary>
    public long Rctx;

    /// <summary><c>1</c> when it names a radio.</summary>
    public byte HasRctx;
}

/// <summary>
/// A station clock value taken apart, mirroring <c>PamojaGatewayStationXtime</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayStationXtime
{
    /// <summary>The radio unit the time was read on, 0 to 127.</summary>
    public byte Unit;

    /// <summary>The run of the station the time belongs to.</summary>
    public byte Session;

    /// <summary>The microseconds the run had counted, below 2^48.</summary>
    public ulong Micros;
}

/// <summary>
/// The identities a discovery answer names, mirroring <c>PamojaGatewayStationRouterIds</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayStationRouterIds
{
    /// <summary>The station, as the server read it, when <c>HasRouter</c>.</summary>
    public PamojaEui Router;

    /// <summary><c>1</c> when the answer names the station.</summary>
    public byte HasRouter;

    /// <summary>The server endpoint carrying the session, when <c>HasMuxs</c>.</summary>
    public PamojaEui Muxs;

    /// <summary><c>1</c> when the answer names the endpoint.</summary>
    public byte HasMuxs;
}

/// <summary>
/// Every fixed field a message carries, mirroring <c>PamojaGatewayStationFields</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaGatewayStationFields
{
    /// <summary>Which kind this is, one of the <c>GatewayStation*</c> constants.</summary>
    public byte Kind;

    /// <summary>The MAC header byte, for a join request or a data frame.</summary>
    public byte Mhdr;

    /// <summary>The application being joined, for a join request.</summary>
    public PamojaEui JoinEui;

    /// <summary>The device, for a join request, a downlink, or a transmission report.</summary>
    public PamojaEui DevEui;

    /// <summary>The nonce a join request used.</summary>
    public ushort DevNonce;

    /// <summary>The address a data frame came from.</summary>
    public int DevAddr;

    /// <summary>The frame control byte.</summary>
    public byte Fctrl;

    /// <summary>The frame counter, as the sixteen bits on the air.</summary>
    public ushort Fcnt;

    /// <summary>The port a data frame was sent on, when <c>HasFport</c>.</summary>
    public byte Fport;

    /// <summary><c>1</c> when the frame carried a port at all.</summary>
    public byte HasFport;

    /// <summary>The message integrity code.</summary>
    public int Mic;

    /// <summary>The data rate it arrived at, or is to be sent at.</summary>
    public byte DataRate;

    /// <summary>The frequency in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>How it was heard, for the kinds a station sends up.</summary>
    public PamojaGatewayStationLevels Levels;

    /// <summary>Which class of downlink this is: 0 for A, 1 for B, 2 for C.</summary>
    public byte Class;

    /// <summary>The identifier a downlink and its transmission report share.</summary>
    public long Diid;

    /// <summary>The delay before the first receive window, in seconds, when <c>HasRxDelay</c>.</summary>
    public byte RxDelay;

    /// <summary><c>1</c> when a downlink named a receive delay.</summary>
    public byte HasRxDelay;

    /// <summary>How urgent a downlink is.</summary>
    public byte Priority;

    /// <summary>The protocol version a version message reports.</summary>
    public uint Protocol;

    /// <summary>The highest radiated power a configuration allows, in dBm.</summary>
    public double MaxEirp;

    /// <summary>The lowest frequency a configuration allows, in hertz.</summary>
    public uint FreqMinHz;

    /// <summary>The highest frequency a configuration allows, in hertz.</summary>
    public uint FreqMaxHz;

    /// <summary><c>1</c> when a configuration names the networks whose frames are forwarded.</summary>
    public byte FiltersNetworks;

    /// <summary>How many networks a configuration names.</summary>
    public nuint NetIdCount;

    /// <summary>How many join identifier ranges a configuration names.</summary>
    public nuint JoinRangeCount;

    /// <summary>How many numbers a configuration's data-rate table holds.</summary>
    public nuint DataRateCount;

    /// <summary>How many frames a schedule carries.</summary>
    public nuint BroadcastCount;

    /// <summary>The first receive window a downlink names.</summary>
    public PamojaGatewayStationWindow Rx1;

    /// <summary>The second receive window a downlink names.</summary>
    public PamojaGatewayStationWindow Rx2;

    /// <summary>The ping slot a class B downlink goes out in.</summary>
    public PamojaGatewayStationWindow PingSlot;

    /// <summary>The station clock in microseconds, when <c>HasXtime</c>.</summary>
    public long Xtime;

    /// <summary><c>1</c> when the message carries a station clock.</summary>
    public byte HasXtime;

    /// <summary>The radio, when <c>HasRctx</c>.</summary>
    public long Rctx;

    /// <summary><c>1</c> when the message names a radio.</summary>
    public byte HasRctx;

    /// <summary>The GPS time in microseconds since the GPS epoch, when <c>HasGpstime</c>.</summary>
    public long Gpstime;

    /// <summary><c>1</c> when the message carries a GPS time.</summary>
    public byte HasGpstime;

    /// <summary>
    /// When a reported frame went out, in seconds, or the station time a time sync carries,
    /// in microseconds, when <c>HasTxtime</c>.
    /// </summary>
    public double Txtime;

    /// <summary><c>1</c> when the message carries that time.</summary>
    public byte HasTxtime;
}
