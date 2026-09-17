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

    /// <summary>Whether a relay runs.</summary>
    public byte Enabled;

    /// <summary>How often a relay scans, as TS011-1.0.1 table 18 codes it.</summary>
    public byte CadPeriodicity;

    /// <summary>Which of the region's relay channels is a relay's default one.</summary>
    public byte DefaultChannelIndex;

    /// <summary>Whether a relay configuration sets a second channel, 1 for yes.</summary>
    public byte SecondChannelIndex;

    /// <summary>The second channel's data rate; its frequency is the frequency field.</summary>
    public byte SecondChannelDataRate;

    /// <summary>How far above its frequency the second channel is acknowledged, as table 35 codes it.</summary>
    public byte SecondChannelAckOffset;

    /// <summary>Whether the scan period was valid.</summary>
    public byte CadPeriodicityAck;

    /// <summary>Whether the default channel was valid.</summary>
    public byte DefaultChannelIndexAck;

    /// <summary>Whether the second channel index was valid.</summary>
    public byte SecondChannelIndexAck;

    /// <summary>Whether the second channel's data rate was valid.</summary>
    public byte SecondChannelDataRateAck;

    /// <summary>Whether its acknowledgment offset was valid.</summary>
    public byte SecondChannelAckOffsetAck;

    /// <summary>Whether its frequency was valid.</summary>
    public byte SecondChannelFrequencyAck;

    /// <summary>How an end device uses a relay, as TS011-1.0.1 table 40 codes it.</summary>
    public byte RelayMode;

    /// <summary>How many unanswered uplinks turn relaying on, as table 41 codes it.</summary>
    public byte SmartEnableLevel;

    /// <summary>How many WOR frames without an acknowledgment before an uplink goes anyway.</summary>
    public byte BackOff;

    /// <summary>What a join filter rule does, or whether a trusted end device is read or removed.</summary>
    public byte Action;

    /// <summary>How many leading bytes of JoinEUI and DevEUI a join filter rule matches.</summary>
    public byte EuiLen;

    /// <summary>Those bytes, most significant first, with the rest zero.</summary>
    public PamojaId Eui;

    /// <summary>Whether a join filter rule was one to create, change or remove.</summary>
    public byte CombinedRulesAck;

    /// <summary>Whether its length was valid.</summary>
    public byte EuiLenAck;

    /// <summary>Whether its action was valid.</summary>
    public byte ActionAck;

    /// <summary>Tokens a trusted end device earns an hour, 63 for no limit.</summary>
    public byte ReloadRate;

    /// <summary>Its bucket size multiplier, as TS011-1.0.1 table 55 codes it.</summary>
    public byte BucketSize;

    /// <summary>An end device address a relay command names.</summary>
    public uint DevAddr;

    /// <summary>A wake-on-radio frame counter.</summary>
    public uint Wfcnt;

    /// <summary>An end device's root relay session key.</summary>
    public PamojaId RootWorSKey;

    /// <summary>Whether a trusted list entry was in use.</summary>
    public byte IndexAck;

    /// <summary>What a forwarding limit command does to a relay's token counters, as table 63 codes it.</summary>
    public byte ResetLimitCounters;

    /// <summary>Join requests a relay forwards an hour, 127 for no limit.</summary>
    public byte JoinRequestReloadRate;

    /// <summary>New end device notifications a relay sends an hour.</summary>
    public byte NotifyReloadRate;

    /// <summary>Uplinks a relay forwards an hour across every trusted end device.</summary>
    public byte GlobalUplinkReloadRate;

    /// <summary>Every message a relay sends an hour.</summary>
    public byte OverallReloadRate;

    /// <summary>The join request bucket size multiplier.</summary>
    public byte JoinRequestBucketSize;

    /// <summary>The notification bucket size multiplier.</summary>
    public byte NotifyBucketSize;

    /// <summary>The global uplink bucket size multiplier.</summary>
    public byte GlobalUplinkBucketSize;

    /// <summary>The overall bucket size multiplier.</summary>
    public byte OverallBucketSize;

    /// <summary>The signal strength of a WOR frame a relay could not verify, in dBm.</summary>
    public short RssiDbm;

    /// <summary>Its signal-to-noise ratio, in dB.</summary>
    public sbyte SnrDb;
}
