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
        var buffer = new byte[16];
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
    internal static LorawanMacCommand From(PamojaLorawanMacCommand flat) => new()
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
    };

    /// <summary>Renders this command as the record the C ABI takes.</summary>
    /// <returns>The record.</returns>
    internal PamojaLorawanMacCommand ToNative() => new()
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
    };
}
