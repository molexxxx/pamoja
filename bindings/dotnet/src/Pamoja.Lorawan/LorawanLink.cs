using Pamoja.Native.Interop;

namespace Pamoja.Lorawan;

/// <summary>A revision of the LoRaWAN link layer.</summary>
public enum LorawanVersion
{
    /// <summary>LoRaWAN 1.0.3.</summary>
    V1_0_3 = 3,

    /// <summary>TS001-1.0.4, the LoRaWAN 1.0.4 link layer.</summary>
    V1_0_4 = 4,
}

/// <summary>The timings and counts RP002-1.0.5 section 3.3 recommends for every region.</summary>
public static class LorawanDefaults
{
    /// <summary>How long after an uplink the first receive window opens, in microseconds.</summary>
    public const uint ReceiveDelay1Micros = 1_000_000;

    /// <summary>How long after an uplink the second receive window opens, in microseconds.</summary>
    public const uint ReceiveDelay2Micros = 2_000_000;

    /// <summary>How long after a join request the first join accept window opens.</summary>
    public const uint JoinAcceptDelay1Micros = 5_000_000;

    /// <summary>How long after a join request the second join accept window opens.</summary>
    public const uint JoinAcceptDelay2Micros = 6_000_000;

    /// <summary>How far a receive window may open either side of its time, LoRaWAN 1.0.3 section 3.3.1.</summary>
    public const uint ReceiveWindowToleranceMicros = 20;

    /// <summary>The largest gap a frame counter may jump across and still be accepted.</summary>
    public const uint MaxFcntGap = 16_384;

    /// <summary>How many unanswered uplinks before a device asks the network to answer.</summary>
    public const uint AdrAckLimit = 64;

    /// <summary>How many more before a device starts giving back what adaptive data rate took.</summary>
    public const uint AdrAckDelay = 32;

    /// <summary>The shortest wait before a confirmed uplink is sent again, in microseconds.</summary>
    public const uint RetransmitTimeoutMinMicros = 1_000_000;

    /// <summary>The longest wait before a confirmed uplink is sent again, in microseconds.</summary>
    public const uint RetransmitTimeoutMaxMicros = 3_000_000;
}

/// <summary>What a back-off says to do with one uplink.</summary>
/// <param name="RequestAck">Set the ADRACKReq bit, asking the network to answer.</param>
/// <param name="RestorePower">
/// Go back to the default transmit power before sending. Only TS001-1.0.4 takes this step.
/// </param>
/// <param name="LowerDataRate">Step the data rate down by the region's back-off table before sending.</param>
/// <param name="RestoreChannels">
/// Re-enable the default channels and set the repetition count back to one before sending.
/// Only TS001-1.0.4 takes this step.
/// </param>
public readonly record struct LorawanBackoffStep(
    bool RequestAck,
    bool RestorePower,
    bool LowerDataRate,
    bool RestoreChannels);

/// <summary>A device's count of how long the network has been silent.</summary>
/// <remarks>
/// A network running adaptive data rate moves a device to the fastest rate and lowest power
/// that still reach it. Once uplinks go unanswered, this says when to ask the network to
/// answer and which of those settings to give back, a step at a time, as LoRaWAN 1.0.3 or
/// TS001-1.0.4 describes.
/// </remarks>
public sealed class LorawanBackoff : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Starts a count from zero.</summary>
    /// <param name="version">The revision whose steps to follow.</param>
    /// <param name="limit">How many unanswered uplinks before asking.</param>
    /// <param name="delay">
    /// How many more before the first step, and between steps after that. Zero is taken as one.
    /// </param>
    /// <exception cref="PamojaException">The version names no revision.</exception>
    public LorawanBackoff(
        LorawanVersion version = LorawanVersion.V1_0_4,
        uint limit = LorawanDefaults.AdrAckLimit,
        uint delay = LorawanDefaults.AdrAckDelay)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lorawan_backoff_new((byte)version, limit, delay, out IntPtr backoff));
        _handle = new NativeHandle(backoff, NativeMethods.pamoja_lorawan_backoff_free);
    }

    /// <summary>How many uplinks have gone unanswered.</summary>
    public uint Counter => _handle.Use(NativeMethods.pamoja_lorawan_backoff_counter);

    /// <summary>The revision whose steps this count follows.</summary>
    public LorawanVersion Version =>
        (LorawanVersion)_handle.Use(NativeMethods.pamoja_lorawan_backoff_version);

    /// <summary>Counts one new uplink, and says what to do before sending it.</summary>
    /// <param name="atDefaultDataRate">
    /// Whether the device is already at its default data rate, the slowest it uses.
    /// </param>
    /// <returns>Whether to ask for an answer, and which step this uplink takes.</returns>
    /// <remarks>
    /// Call it once per uplink the frame counter moves for; a repeat of the same uplink does
    /// not count.
    /// </remarks>
    public LorawanBackoffStep Uplink(bool atDefaultDataRate)
    {
        return _handle.Use(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_backoff_uplink(
                handle,
                (byte)(atDefaultDataRate ? 1 : 0),
                out PamojaLorawanBackoffStep step));
            return new LorawanBackoffStep(
                step.RequestAck != 0,
                step.RestorePower != 0,
                step.LowerDataRate != 0,
                step.RestoreChannels != 0);
        });
    }

    /// <summary>Counts a Class A downlink, which proves the network still hears the device.</summary>
    public void Downlink() =>
        _handle.Use(handle => Status.ThrowIfError(NativeMethods.pamoja_lorawan_backoff_downlink(handle)));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Which form a channel list takes, from its last byte.</summary>
public enum LorawanCfListKind
{
    /// <summary>Type 0: a list of frequencies.</summary>
    Frequencies = 0,

    /// <summary>Type 1: groups of channel mask bits.</summary>
    ChannelMasks = 1,

    /// <summary>A type the regional parameters reserve, which a device ignores.</summary>
    Reserved = 2,
}

/// <summary>The optional channel list at the end of a join accept.</summary>
/// <remarks>
/// A network that wants a device on more channels than its region's defaults says so in
/// sixteen bytes: five frequencies for a dynamic plan such as EU868, or six groups of channel
/// mask bits for a fixed plan such as US915. The list keeps the bytes as they arrived and
/// reads either form out of them.
/// </remarks>
public sealed class LorawanCfList : IEquatable<LorawanCfList>
{
    private readonly byte[] _bytes;

    private LorawanCfList(byte[] bytes) => _bytes = bytes;

    /// <summary>Builds a type 0 list from frequencies.</summary>
    /// <param name="frequenciesHz">Five frequencies in hertz, with 0 for a slot left unused.</param>
    /// <returns>The channel list.</returns>
    /// <exception cref="PamojaException">
    /// There are not five, or a frequency is not a whole number of hundreds of hertz from
    /// 100 MHz to just under 1.678 GHz.
    /// </exception>
    public static LorawanCfList FromFrequencies(IReadOnlyList<uint> frequenciesHz)
    {
        uint[] slots = [.. frequenciesHz];
        byte[] bytes = new byte[NativeMethods.LorawanCfListLen];
        Status.ThrowIfError(
            NativeMethods.pamoja_lorawan_cflist_from_frequencies(slots, (nuint)slots.Length, bytes));
        return new LorawanCfList(bytes);
    }

    /// <summary>Builds a type 1 list from channel mask groups.</summary>
    /// <param name="masks">Six groups, where bit n of group g enables channel g * 16 + n.</param>
    /// <returns>The channel list.</returns>
    /// <exception cref="PamojaException">There are not six groups.</exception>
    public static LorawanCfList FromChannelMasks(IReadOnlyList<ushort> masks)
    {
        ushort[] groups = [.. masks];
        byte[] bytes = new byte[NativeMethods.LorawanCfListLen];
        Status.ThrowIfError(
            NativeMethods.pamoja_lorawan_cflist_from_channel_masks(groups, (nuint)groups.Length, bytes));
        return new LorawanCfList(bytes);
    }

    /// <summary>Keeps a channel list exactly as it arrived, whatever its type byte says.</summary>
    /// <param name="bytes">The sixteen CFList bytes.</param>
    /// <returns>The channel list.</returns>
    /// <exception cref="PamojaException">There are not sixteen bytes.</exception>
    public static LorawanCfList FromBytes(ReadOnlySpan<byte> bytes)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lorawan_cflist_type(bytes, (nuint)bytes.Length, out _));
        return new LorawanCfList(bytes.ToArray());
    }

    /// <summary>The sixteen bytes, as a join accept carries them.</summary>
    public byte[] Bytes => (byte[])_bytes.Clone();

    /// <summary>The CFListType byte the list ends with.</summary>
    public byte TypeByte => _bytes[^1];

    /// <summary>Which form the list takes.</summary>
    public LorawanCfListKind Kind => TypeByte switch
    {
        NativeMethods.LorawanCfListTypeFrequencies => LorawanCfListKind.Frequencies,
        NativeMethods.LorawanCfListTypeChannelMasks => LorawanCfListKind.ChannelMasks,
        _ => LorawanCfListKind.Reserved,
    };

    /// <summary>Reads the frequencies out of a type 0 list.</summary>
    /// <returns>
    /// Five frequencies in hertz, 0 for an unused slot, or <c>null</c> for a list of any other type.
    /// </returns>
    public uint[]? FrequenciesHz()
    {
        uint[] slots = new uint[NativeMethods.LorawanCfListFrequencies];
        return NativeMethods.pamoja_lorawan_cflist_frequencies(
            _bytes, (nuint)_bytes.Length, slots, (nuint)slots.Length) == PamojaStatus.Ok
            ? slots
            : null;
    }

    /// <summary>Reads the mask groups out of a type 1 list.</summary>
    /// <returns>Six groups, or <c>null</c> for a list of any other type.</returns>
    public ushort[]? ChannelMaskGroups()
    {
        ushort[] groups = new ushort[NativeMethods.LorawanCfListMaskGroups];
        return NativeMethods.pamoja_lorawan_cflist_channel_masks(
            _bytes, (nuint)_bytes.Length, groups, (nuint)groups.Length) == PamojaStatus.Ok
            ? groups
            : null;
    }

    /// <summary>Reports whether a type 1 list enables a channel.</summary>
    /// <param name="channel">The channel number, group * 16 + bit.</param>
    /// <returns>
    /// Whether its bit is set, or <c>null</c> for a list of any other type or a channel past
    /// the 96 the groups cover.
    /// </returns>
    public bool? Enables(int channel)
    {
        if (channel is < 0 or > byte.MaxValue)
        {
            return null;
        }

        return NativeMethods.pamoja_lorawan_cflist_enables(
            _bytes, (nuint)_bytes.Length, (byte)channel, out byte enabled) == PamojaStatus.Ok
            ? enabled != 0
            : null;
    }

    /// <summary>Lists the channels a type 1 list enables.</summary>
    /// <returns>The channel numbers, lowest first, which is empty for a list of any other type.</returns>
    public IReadOnlyList<int> EnabledChannels()
    {
        List<int> channels = [];
        if (ChannelMaskGroups() is not { } groups)
        {
            return channels;
        }

        for (int channel = 0; channel < groups.Length * 16; channel++)
        {
            if ((groups[channel / 16] & (1 << (channel % 16))) != 0)
            {
                channels.Add(channel);
            }
        }

        return channels;
    }

    /// <inheritdoc/>
    public bool Equals(LorawanCfList? other) =>
        other is not null && _bytes.AsSpan().SequenceEqual(other._bytes);

    /// <inheritdoc/>
    public override bool Equals(object? obj) => Equals(obj as LorawanCfList);

    /// <inheritdoc/>
    public override int GetHashCode()
    {
        HashCode hash = default;
        hash.AddBytes(_bytes);
        return hash.ToHashCode();
    }

    /// <inheritdoc/>
    public override string ToString() => $"LorawanCfList({Convert.ToHexString(_bytes)})";
}
