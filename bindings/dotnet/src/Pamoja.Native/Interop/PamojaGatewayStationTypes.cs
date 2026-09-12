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
/// The fields a message carries, mirroring <c>PamojaGatewayStationFields</c> in
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

    /// <summary>Which class of downlink this is.</summary>
    public byte Class;

    /// <summary>The identifier a transmission report carries back.</summary>
    public long Diid;

    /// <summary>The delay before the first receive window, in seconds, when <c>HasRxDelay</c>.</summary>
    public byte RxDelay;

    /// <summary><c>1</c> when a downlink named a receive delay.</summary>
    public byte HasRxDelay;

    /// <summary>How urgent a downlink is.</summary>
    public byte Priority;
}
