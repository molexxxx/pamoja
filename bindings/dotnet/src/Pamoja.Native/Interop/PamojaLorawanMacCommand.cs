using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// One of the commands a network and a device configure each other with, mirroring
/// <c>PamojaLorawanMacCommand</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// <see cref="Cid"/> names the command and <see cref="Direction"/> says which way it
/// travels; together they decide which of the other fields carry anything. The rest
/// read as zero.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanMacCommand
{
    /// <summary>Which command this is.</summary>
    public byte Cid;

    /// <summary>Which way it travels.</summary>
    public PamojaLorawanDirection Direction;

    /// <summary>How far above the floor a link check arrived, in dB.</summary>
    public byte Margin;

    /// <summary>How many gateways heard it.</summary>
    public byte Gateways;

    /// <summary>The data rate a network asks a device to use.</summary>
    public byte DataRate;

    /// <summary>The transmit power it may use, as a ceiling.</summary>
    public byte TxPower;

    /// <summary>Which channels may carry an uplink.</summary>
    public ushort ChannelMask;

    /// <summary>Which block of sixteen channels that mask applies to.</summary>
    public byte MaskControl;

    /// <summary>How many times to send an unconfirmed uplink.</summary>
    public byte Transmissions;

    /// <summary>Whether the power was set, <c>1</c> for yes.</summary>
    public byte PowerAck;

    /// <summary>Whether the data rate was set.</summary>
    public byte DataRateAck;

    /// <summary>Whether the channel mask was usable.</summary>
    public byte ChannelMaskAck;

    /// <summary>The share of the air a device is held to, as one over two to this.</summary>
    public byte MaxDutyCycle;

    /// <summary>How far the first receive window sits below the uplink rate.</summary>
    public byte Rx1Offset;

    /// <summary>The rate of the second receive window.</summary>
    public byte Rx2DataRate;

    /// <summary>A frequency in hertz, for the windows and the channel commands.</summary>
    public uint FrequencyHz;

    /// <summary>Whether the window offset was in range.</summary>
    public byte Rx1OffsetAck;

    /// <summary>Whether the window rate was known.</summary>
    public byte Rx2DataRateAck;

    /// <summary>Whether the frequency was usable.</summary>
    public byte ChannelAck;

    /// <summary>A device battery level: 0 on external power, 255 when it cannot tell.</summary>
    public byte Battery;

    /// <summary>The signal-to-noise ratio of the last request, in dB.</summary>
    public sbyte SnrMargin;

    /// <summary>Which channel a channel command names.</summary>
    public byte Index;

    /// <summary>The fastest rate allowed on it.</summary>
    public byte MaxDataRate;

    /// <summary>The slowest rate allowed on it.</summary>
    public byte MinDataRate;

    /// <summary>Whether the device can run that range of rates.</summary>
    public byte DataRateRangeOk;

    /// <summary>Whether its radio can reach that frequency.</summary>
    public byte FrequencyOk;

    /// <summary>How long a device waits before its first receive window, as coded.</summary>
    public byte Delay;

    /// <summary>The coded transmit power ceiling a region imposes.</summary>
    public byte MaxEirp;

    /// <summary>Whether an uplink is held to 400 ms of air time.</summary>
    public byte UplinkDwell;

    /// <summary>Whether a downlink is.</summary>
    public byte DownlinkDwell;

    /// <summary>Whether the channel already had an uplink frequency to pair with.</summary>
    public byte UplinkFrequencyExists;

    /// <summary>Seconds since the GPS epoch.</summary>
    public uint Seconds;

    /// <summary>The fraction of that second, in steps of one part in 256.</summary>
    public byte Fraction;
}
