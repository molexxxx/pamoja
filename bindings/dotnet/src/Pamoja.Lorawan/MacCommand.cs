using Pamoja.Native;
using Pamoja.Native.Interop;

namespace Pamoja.Lorawan;

/// <summary>
/// One of the commands a network and a device configure each other with.
/// </summary>
/// <remarks>
/// <para>
/// A frame carries these either in its options field, at most fifteen bytes of them,
/// or as a whole payload on port 0.
/// </para>
/// <para>
/// <see cref="Cid"/> names the command and <see cref="Direction"/> says which way it
/// travels; together they decide which of the other properties carry anything. The
/// same identifier means a different command in each direction, so the direction is
/// always given and never guessed.
/// </para>
/// </remarks>
public sealed class LorawanMacCommand
{
    /// <summary>The identifier this command travels under.</summary>
    public required byte Cid { get; init; }

    /// <summary>Which way it travels.</summary>
    public required LorawanDirection Direction { get; init; }

    /// <summary>How far above the floor a link check arrived, in dB.</summary>
    public byte Margin { get; init; }

    /// <summary>How many gateways heard it.</summary>
    public byte Gateways { get; init; }

    /// <summary>The data rate a network asks a device to use.</summary>
    public byte DataRate { get; init; }

    /// <summary>The transmit power it may use, as a ceiling.</summary>
    public byte TxPower { get; init; }

    /// <summary>Which channels may carry an uplink.</summary>
    public ushort ChannelMask { get; init; }

    /// <summary>Which block of sixteen channels that mask applies to.</summary>
    public byte MaskControl { get; init; }

    /// <summary>How many times to send an unconfirmed uplink.</summary>
    public byte Transmissions { get; init; }

    /// <summary>Whether the power was set.</summary>
    public bool PowerAck { get; init; }

    /// <summary>Whether the data rate was set.</summary>
    public bool DataRateAck { get; init; }

    /// <summary>Whether the channel mask was usable.</summary>
    public bool ChannelMaskAck { get; init; }

    /// <summary>The share of the air a device is held to, as one over two to this.</summary>
    public byte MaxDutyCycle { get; init; }

    /// <summary>How far the first receive window sits below the uplink rate.</summary>
    public byte Rx1Offset { get; init; }

    /// <summary>The rate of the second receive window.</summary>
    public byte Rx2DataRate { get; init; }

    /// <summary>A frequency in hertz, for the windows and the channel commands.</summary>
    public uint FrequencyHz { get; init; }

    /// <summary>Whether the window offset was in range.</summary>
    public bool Rx1OffsetAck { get; init; }

    /// <summary>Whether the window rate was known.</summary>
    public bool Rx2DataRateAck { get; init; }

    /// <summary>Whether the frequency was usable.</summary>
    public bool ChannelAck { get; init; }

    /// <summary>A device battery level: 0 on external power, 255 when it cannot tell.</summary>
    public byte Battery { get; init; }

    /// <summary>The signal-to-noise ratio of the last request, in dB.</summary>
    public sbyte SnrMargin { get; init; }

    /// <summary>Which channel a channel command names.</summary>
    public byte Index { get; init; }

    /// <summary>The fastest rate allowed on it.</summary>
    public byte MaxDataRate { get; init; }

    /// <summary>The slowest rate allowed on it.</summary>
    public byte MinDataRate { get; init; }

    /// <summary>Whether the device can run that range of rates.</summary>
    public bool DataRateRangeOk { get; init; }

    /// <summary>Whether its radio can reach that frequency.</summary>
    public bool FrequencyOk { get; init; }

    /// <summary>How long a device waits before its first receive window, as coded.</summary>
    public byte Delay { get; init; }

    /// <summary>The coded transmit power ceiling a region imposes.</summary>
    public byte MaxEirp { get; init; }

    /// <summary>Whether an uplink is held to 400 ms of air time.</summary>
    public bool UplinkDwell { get; init; }

    /// <summary>Whether a downlink is.</summary>
    public bool DownlinkDwell { get; init; }

    /// <summary>Whether the channel already had an uplink frequency to pair with.</summary>
    public bool UplinkFrequencyExists { get; init; }

    /// <summary>Seconds since the GPS epoch.</summary>
    public uint Seconds { get; init; }

    /// <summary>The fraction of that second, in steps of one part in 256.</summary>
    public byte Fraction { get; init; }

    /// <summary>Whether a relay runs.</summary>
    public bool Enabled { get; init; }

    /// <summary>How often a relay scans, as TS011-1.0.1 table 18 codes it.</summary>
    public byte CadPeriodicity { get; init; }

    /// <summary>Which of the region's relay channels is a relay's default one.</summary>
    public byte DefaultChannelIndex { get; init; }

    /// <summary>Whether a relay configuration sets a second channel, 1 for yes.</summary>
    public byte SecondChannelIndex { get; init; }

    /// <summary>The second channel's data rate; its frequency is the frequency field.</summary>
    public byte SecondChannelDataRate { get; init; }

    /// <summary>How far above its frequency the second channel is acknowledged, as table 35 codes it.</summary>
    public byte SecondChannelAckOffset { get; init; }

    /// <summary>Whether the scan period was valid.</summary>
    public bool CadPeriodicityAck { get; init; }

    /// <summary>Whether the default channel was valid.</summary>
    public bool DefaultChannelIndexAck { get; init; }

    /// <summary>Whether the second channel index was valid.</summary>
    public bool SecondChannelIndexAck { get; init; }

    /// <summary>Whether the second channel's data rate was valid.</summary>
    public bool SecondChannelDataRateAck { get; init; }

    /// <summary>Whether its acknowledgment offset was valid.</summary>
    public bool SecondChannelAckOffsetAck { get; init; }

    /// <summary>Whether its frequency was valid.</summary>
    public bool SecondChannelFrequencyAck { get; init; }

    /// <summary>How an end device uses a relay, as TS011-1.0.1 table 40 codes it.</summary>
    public byte RelayMode { get; init; }

    /// <summary>How many unanswered uplinks turn relaying on, as table 41 codes it.</summary>
    public byte SmartEnableLevel { get; init; }

    /// <summary>How many WOR frames without an acknowledgment before an uplink goes anyway.</summary>
    public byte BackOff { get; init; }

    /// <summary>What a join filter rule does, or whether a trusted end device is read or removed.</summary>
    public byte Action { get; init; }

    /// <summary>How many leading bytes of JoinEUI and DevEUI a join filter rule matches.</summary>
    public byte EuiLen { get; init; }

    /// <summary>Those bytes, most significant first, with the rest zero.</summary>
    public byte[] Eui { get; init; } = [];

    /// <summary>Whether a join filter rule was one to create, change or remove.</summary>
    public bool CombinedRulesAck { get; init; }

    /// <summary>Whether its length was valid.</summary>
    public bool EuiLenAck { get; init; }

    /// <summary>Whether its action was valid.</summary>
    public bool ActionAck { get; init; }

    /// <summary>Tokens a trusted end device earns an hour, 63 for no limit.</summary>
    public byte ReloadRate { get; init; }

    /// <summary>Its bucket size multiplier, as TS011-1.0.1 table 55 codes it.</summary>
    public byte BucketSize { get; init; }

    /// <summary>An end device address a relay command names.</summary>
    public uint DevAddr { get; init; }

    /// <summary>A wake-on-radio frame counter.</summary>
    public uint Wfcnt { get; init; }

    /// <summary>An end device's root relay session key.</summary>
    public byte[] RootWorSKey { get; init; } = [];

    /// <summary>Whether a trusted list entry was in use.</summary>
    public bool IndexAck { get; init; }

    /// <summary>What a forwarding limit command does to a relay's token counters, as table 63 codes it.</summary>
    public byte ResetLimitCounters { get; init; }

    /// <summary>Join requests a relay forwards an hour, 127 for no limit.</summary>
    public byte JoinRequestReloadRate { get; init; }

    /// <summary>New end device notifications a relay sends an hour.</summary>
    public byte NotifyReloadRate { get; init; }

    /// <summary>Uplinks a relay forwards an hour across every trusted end device.</summary>
    public byte GlobalUplinkReloadRate { get; init; }

    /// <summary>Every message a relay sends an hour.</summary>
    public byte OverallReloadRate { get; init; }

    /// <summary>The join request bucket size multiplier.</summary>
    public byte JoinRequestBucketSize { get; init; }

    /// <summary>The notification bucket size multiplier.</summary>
    public byte NotifyBucketSize { get; init; }

    /// <summary>The global uplink bucket size multiplier.</summary>
    public byte GlobalUplinkBucketSize { get; init; }

    /// <summary>The overall bucket size multiplier.</summary>
    public byte OverallBucketSize { get; init; }

    /// <summary>The signal strength of a WOR frame a relay could not verify, in dBm.</summary>
    public short RssiDbm { get; init; }

    /// <summary>Its signal-to-noise ratio, in dB.</summary>
    public sbyte SnrDb { get; init; }

    /// <summary>The longest command, in bytes: a relay's <c>UpdateUplinkListReq</c>.</summary>
    public const int MaxLength = 27;

    /// <summary>Reads the commands packed into a field.</summary>
    /// <remarks>
    /// A command does not carry its own length, so one this build does not know cannot
    /// be stepped over. Reading stops there and returns what came before it.
    /// </remarks>
    /// <param name="direction">Which way the frame carrying them travels.</param>
    /// <param name="bytes">The options field, or a payload sent on port 0.</param>
    /// <returns>The commands that were readable, in order.</returns>
    /// <exception cref="PamojaException">If the field cannot be read at all.</exception>
    public static IReadOnlyList<LorawanMacCommand> Parse(
        LorawanDirection direction,
        ReadOnlySpan<byte> bytes)
    {
        var native = (PamojaLorawanDirection)direction;
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_mac_count(
            native,
            bytes,
            (nuint)bytes.Length,
            out var count));

        var commands = new List<LorawanMacCommand>((int)count);
        for (nuint at = 0; at < count; at++)
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_mac_at(
                native,
                bytes,
                (nuint)bytes.Length,
                at,
                out var one));
            commands.Add(From(one));
        }
        return commands;
    }

    /// <summary>Writes this command out.</summary>
    /// <returns>The bytes it goes out as.</returns>
    /// <exception cref="PamojaException">
    /// If the identifier and direction name no command, or a field will not fit what
    /// carries it.
    /// </exception>
    public byte[] Encode()
    {
        var flat = ToNative();
        var buffer = new byte[MaxLength];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_mac_encode(
            ref flat,
            buffer,
            (nuint)buffer.Length,
            out var written));
        return buffer[..(int)written];
    }

    /// <summary>Reads a command out of the record the C ABI hands back.</summary>
    /// <param name="flat">The record.</param>
    /// <returns>The command.</returns>
    public static LorawanMacCommand From(PamojaLorawanMacCommand flat) => new()
    {
        Cid = flat.Cid,
        Direction = (LorawanDirection)flat.Direction,
        Margin = flat.Margin,
        Gateways = flat.Gateways,
        DataRate = flat.DataRate,
        TxPower = flat.TxPower,
        ChannelMask = flat.ChannelMask,
        MaskControl = flat.MaskControl,
        Transmissions = flat.Transmissions,
        PowerAck = flat.PowerAck != 0,
        DataRateAck = flat.DataRateAck != 0,
        ChannelMaskAck = flat.ChannelMaskAck != 0,
        MaxDutyCycle = flat.MaxDutyCycle,
        Rx1Offset = flat.Rx1Offset,
        Rx2DataRate = flat.Rx2DataRate,
        FrequencyHz = flat.FrequencyHz,
        Rx1OffsetAck = flat.Rx1OffsetAck != 0,
        Rx2DataRateAck = flat.Rx2DataRateAck != 0,
        ChannelAck = flat.ChannelAck != 0,
        Battery = flat.Battery,
        SnrMargin = flat.SnrMargin,
        Index = flat.Index,
        MaxDataRate = flat.MaxDataRate,
        MinDataRate = flat.MinDataRate,
        DataRateRangeOk = flat.DataRateRangeOk != 0,
        FrequencyOk = flat.FrequencyOk != 0,
        Delay = flat.Delay,
        MaxEirp = flat.MaxEirp,
        UplinkDwell = flat.UplinkDwell != 0,
        DownlinkDwell = flat.DownlinkDwell != 0,
        UplinkFrequencyExists = flat.UplinkFrequencyExists != 0,
        Seconds = flat.Seconds,
        Fraction = flat.Fraction,
        Enabled = flat.Enabled != 0,
        CadPeriodicity = flat.CadPeriodicity,
        DefaultChannelIndex = flat.DefaultChannelIndex,
        SecondChannelIndex = flat.SecondChannelIndex,
        SecondChannelDataRate = flat.SecondChannelDataRate,
        SecondChannelAckOffset = flat.SecondChannelAckOffset,
        CadPeriodicityAck = flat.CadPeriodicityAck != 0,
        DefaultChannelIndexAck = flat.DefaultChannelIndexAck != 0,
        SecondChannelIndexAck = flat.SecondChannelIndexAck != 0,
        SecondChannelDataRateAck = flat.SecondChannelDataRateAck != 0,
        SecondChannelAckOffsetAck = flat.SecondChannelAckOffsetAck != 0,
        SecondChannelFrequencyAck = flat.SecondChannelFrequencyAck != 0,
        RelayMode = flat.RelayMode,
        SmartEnableLevel = flat.SmartEnableLevel,
        BackOff = flat.BackOff,
        Action = flat.Action,
        EuiLen = flat.EuiLen,
        Eui = flat.Eui.ToArray(),
        CombinedRulesAck = flat.CombinedRulesAck != 0,
        EuiLenAck = flat.EuiLenAck != 0,
        ActionAck = flat.ActionAck != 0,
        ReloadRate = flat.ReloadRate,
        BucketSize = flat.BucketSize,
        DevAddr = flat.DevAddr,
        Wfcnt = flat.Wfcnt,
        RootWorSKey = flat.RootWorSKey.ToArray(),
        IndexAck = flat.IndexAck != 0,
        ResetLimitCounters = flat.ResetLimitCounters,
        JoinRequestReloadRate = flat.JoinRequestReloadRate,
        NotifyReloadRate = flat.NotifyReloadRate,
        GlobalUplinkReloadRate = flat.GlobalUplinkReloadRate,
        OverallReloadRate = flat.OverallReloadRate,
        JoinRequestBucketSize = flat.JoinRequestBucketSize,
        NotifyBucketSize = flat.NotifyBucketSize,
        GlobalUplinkBucketSize = flat.GlobalUplinkBucketSize,
        OverallBucketSize = flat.OverallBucketSize,
        RssiDbm = flat.RssiDbm,
        SnrDb = flat.SnrDb,
    };

    /// <summary>Renders this command as the record the C ABI takes.</summary>
    /// <returns>The record.</returns>
    public PamojaLorawanMacCommand ToNative() => new()
    {
        Cid = Cid,
        Direction = (PamojaLorawanDirection)Direction,
        Margin = Margin,
        Gateways = Gateways,
        DataRate = DataRate,
        TxPower = TxPower,
        ChannelMask = ChannelMask,
        MaskControl = MaskControl,
        Transmissions = Transmissions,
        PowerAck = PowerAck ? (byte)1 : (byte)0,
        DataRateAck = DataRateAck ? (byte)1 : (byte)0,
        ChannelMaskAck = ChannelMaskAck ? (byte)1 : (byte)0,
        MaxDutyCycle = MaxDutyCycle,
        Rx1Offset = Rx1Offset,
        Rx2DataRate = Rx2DataRate,
        FrequencyHz = FrequencyHz,
        Rx1OffsetAck = Rx1OffsetAck ? (byte)1 : (byte)0,
        Rx2DataRateAck = Rx2DataRateAck ? (byte)1 : (byte)0,
        ChannelAck = ChannelAck ? (byte)1 : (byte)0,
        Battery = Battery,
        SnrMargin = SnrMargin,
        Index = Index,
        MaxDataRate = MaxDataRate,
        MinDataRate = MinDataRate,
        DataRateRangeOk = DataRateRangeOk ? (byte)1 : (byte)0,
        FrequencyOk = FrequencyOk ? (byte)1 : (byte)0,
        Delay = Delay,
        MaxEirp = MaxEirp,
        UplinkDwell = UplinkDwell ? (byte)1 : (byte)0,
        DownlinkDwell = DownlinkDwell ? (byte)1 : (byte)0,
        UplinkFrequencyExists = UplinkFrequencyExists ? (byte)1 : (byte)0,
        Seconds = Seconds,
        Fraction = Fraction,
        Enabled = Enabled ? (byte)1 : (byte)0,
        CadPeriodicity = CadPeriodicity,
        DefaultChannelIndex = DefaultChannelIndex,
        SecondChannelIndex = SecondChannelIndex,
        SecondChannelDataRate = SecondChannelDataRate,
        SecondChannelAckOffset = SecondChannelAckOffset,
        CadPeriodicityAck = CadPeriodicityAck ? (byte)1 : (byte)0,
        DefaultChannelIndexAck = DefaultChannelIndexAck ? (byte)1 : (byte)0,
        SecondChannelIndexAck = SecondChannelIndexAck ? (byte)1 : (byte)0,
        SecondChannelDataRateAck = SecondChannelDataRateAck ? (byte)1 : (byte)0,
        SecondChannelAckOffsetAck = SecondChannelAckOffsetAck ? (byte)1 : (byte)0,
        SecondChannelFrequencyAck = SecondChannelFrequencyAck ? (byte)1 : (byte)0,
        RelayMode = RelayMode,
        SmartEnableLevel = SmartEnableLevel,
        BackOff = BackOff,
        Action = Action,
        EuiLen = EuiLen,
        Eui = Sixteen(Eui, nameof(Eui)),
        CombinedRulesAck = CombinedRulesAck ? (byte)1 : (byte)0,
        EuiLenAck = EuiLenAck ? (byte)1 : (byte)0,
        ActionAck = ActionAck ? (byte)1 : (byte)0,
        ReloadRate = ReloadRate,
        BucketSize = BucketSize,
        DevAddr = DevAddr,
        Wfcnt = Wfcnt,
        RootWorSKey = Sixteen(RootWorSKey, nameof(RootWorSKey)),
        IndexAck = IndexAck ? (byte)1 : (byte)0,
        ResetLimitCounters = ResetLimitCounters,
        JoinRequestReloadRate = JoinRequestReloadRate,
        NotifyReloadRate = NotifyReloadRate,
        GlobalUplinkReloadRate = GlobalUplinkReloadRate,
        OverallReloadRate = OverallReloadRate,
        JoinRequestBucketSize = JoinRequestBucketSize,
        NotifyBucketSize = NotifyBucketSize,
        GlobalUplinkBucketSize = GlobalUplinkBucketSize,
        OverallBucketSize = OverallBucketSize,
        RssiDbm = RssiDbm,
        SnrDb = SnrDb,
    };

    private static PamojaId Sixteen(byte[] bytes, string name) =>
        bytes.Length == 0 ? default : PamojaId.From(bytes, name);
}
