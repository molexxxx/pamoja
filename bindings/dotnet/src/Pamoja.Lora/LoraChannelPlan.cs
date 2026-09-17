using Pamoja.Native.Interop;

namespace Pamoja.Lora;

/// <summary>A band with a published channel plan.</summary>
public enum LoraRegion
{
    /// <summary>Europe, 863-870 MHz.</summary>
    Eu868 = 1,

    /// <summary>North America, 902-928 MHz.</summary>
    Us915 = 2,

    /// <summary>Europe, 433 MHz.</summary>
    Eu433 = 3,

    /// <summary>Australia, 915-928 MHz.</summary>
    Au915 = 4,

    /// <summary>China, 470-510 MHz.</summary>
    Cn470 = 5,

    /// <summary>Asia, 923 MHz.</summary>
    As923 = 6,

    /// <summary>South Korea, 920-923 MHz.</summary>
    Kr920 = 7,

    /// <summary>India, 865-867 MHz.</summary>
    In865 = 8,

    /// <summary>Russia, 864-870 MHz.</summary>
    Ru864 = 9,
}

/// <summary>Which direction a data-rate table describes.</summary>
/// <remarks>
/// Most regions number their data rates the same way in both directions and carry
/// one table; the 900 MHz plans do not.
/// </remarks>
public enum LoraDirection
{
    /// <summary>From the device to the network.</summary>
    Uplink = 0,

    /// <summary>From the network to the device.</summary>
    Downlink = 1,
}

/// <summary>Which of a plan's payload tables to read.</summary>
public enum LoraPayloadTable
{
    /// <summary>Uplink, for a device that may sit behind a repeater.</summary>
    UplinkRepeater = 0,

    /// <summary>Uplink, for a device that will not.</summary>
    UplinkDirect = 1,

    /// <summary>Downlink, for a device that may sit behind a repeater.</summary>
    DownlinkRepeater = 2,

    /// <summary>Downlink, for a device that will not.</summary>
    DownlinkDirect = 3,

    /// <summary>The limits that apply under a dwell-time limit.</summary>
    DwellLimited = 4,
}

/// <summary>Which channels of a plan to read.</summary>
public enum LoraChannelSet
{
    /// <summary>The channels a device must use to send a join request.</summary>
    Join = 0,

    /// <summary>The channels a device starts with before a network adds any.</summary>
    Default = 1,

    /// <summary>The numbered downlink channels a fixed plan answers the first receive window on.</summary>
    Downlink = 2,
}

/// <summary>One of the CN470-510 channel plans.</summary>
/// <remarks>
/// RP002-1.0.5 divides the band into four plans, for 20 MHz and 26 MHz antennas, each
/// with a type A and B. A device joining over the air uses the twenty common join
/// channels they share and moves to the plan its join channel names.
/// </remarks>
public enum LoraCn470Plan
{
    /// <summary>A 20 MHz antenna, type A, the plan <see cref="LoraRegion.Cn470"/> names.</summary>
    Antenna20MhzA = 1,

    /// <summary>A 20 MHz antenna, type B.</summary>
    Antenna20MhzB = 2,

    /// <summary>A 26 MHz antenna, type A.</summary>
    Antenna26MhzA = 3,

    /// <summary>A 26 MHz antenna, type B.</summary>
    Antenna26MhzB = 4,

    /// <summary>The 96-channel plan of the LoRaWAN 1.0.3 Regional Parameters revision A.</summary>
    Channels96 = 5,
}

/// <summary>Whether a plan's network creates channels or only switches numbered ones.</summary>
public enum LoraPlanKind
{
    /// <summary>The network creates channels and moves them, as in Europe.</summary>
    Dynamic = 0,

    /// <summary>
    /// The channels are numbered in advance and only enabled or disabled, as in North America.
    /// </summary>
    Fixed = 1,
}

/// <summary>The numbering a dynamic plan reads a type 1 channel list against.</summary>
public enum LoraChannelList
{
    /// <summary>The 800 MHz numbering of RP002-1.0.5 section 3.3.1.1.</summary>
    Mhz800 = 1,

    /// <summary>The 900 MHz numbering of RP002-1.0.5 section 3.3.1.2.</summary>
    Mhz900 = 2,
}

/// <summary>The order a device tries the join channels in.</summary>
public enum LoraJoinSequence
{
    /// <summary>A join channel at random, stepping the data rate down across attempts.</summary>
    Random = 0,

    /// <summary>
    /// Eight 125 kHz channels from successive groups, then a 500 kHz one, with no channel
    /// repeated until all have gone out, RP002-1.0.5 section 3.5.2.
    /// </summary>
    OctetPasses = 1,
}

/// <summary>What a plan's transmit power indexes count down from.</summary>
public enum LoraPowerReference
{
    /// <summary>A radiated ceiling.</summary>
    Eirp = 0,

    /// <summary>A conducted ceiling, with an allowance for antenna gain.</summary>
    Conducted = 1,
}

/// <summary>What one <c>ChMaskCntl</c> value of a <c>LinkADRReq</c> does.</summary>
public enum LoraMaskControlKind
{
    /// <summary>The mask sets one group of sixteen channels.</summary>
    Group = 0,

    /// <summary>The ten low bits switch banks of eight channels.</summary>
    Banks = 1,

    /// <summary>
    /// The eight low bits switch banks of eight with their 500 kHz channel, and the ninth
    /// the 500 kHz channels past them.
    /// </summary>
    PairedBanks = 2,

    /// <summary>Every channel turns on or off, then the mask may set a group.</summary>
    All = 3,

    /// <summary>The value is reserved.</summary>
    Reserved = 4,
}

/// <summary>What one <c>ChMaskCntl</c> value does.</summary>
/// <param name="Kind">What the value does.</param>
/// <param name="Group">For <see cref="LoraMaskControlKind.Group"/>, the group the mask sets.</param>
/// <param name="On">For <see cref="LoraMaskControlKind.All"/>, whether every channel turns on.</param>
/// <param name="ThenGroup">For <see cref="LoraMaskControlKind.All"/>, the group the mask then sets.</param>
public readonly record struct LoraMaskControl(
    LoraMaskControlKind Kind,
    byte? Group,
    bool? On,
    byte? ThenGroup)
{
    /// <summary>The mask sets one group of sixteen channels.</summary>
    /// <param name="group">The group.</param>
    /// <returns>The control.</returns>
    public static LoraMaskControl OneGroup(byte group) =>
        new(LoraMaskControlKind.Group, group, null, null);

    /// <summary>The ten low bits of the mask switch banks of eight channels.</summary>
    /// <returns>The control.</returns>
    public static LoraMaskControl Banks() => new(LoraMaskControlKind.Banks, null, null, null);

    /// <summary>The eight low bits switch banks of eight with their 500 kHz channel.</summary>
    /// <returns>The control.</returns>
    public static LoraMaskControl PairedBanks() =>
        new(LoraMaskControlKind.PairedBanks, null, null, null);

    /// <summary>Every channel turns on or off, then the mask sets a group if one is given.</summary>
    /// <param name="on">Whether every channel turns on.</param>
    /// <param name="thenGroup">The group the mask then sets, if any.</param>
    /// <returns>The control.</returns>
    public static LoraMaskControl AllChannels(bool on, byte? thenGroup = null) =>
        new(LoraMaskControlKind.All, null, on, thenGroup);

    /// <summary>The value is reserved.</summary>
    /// <returns>The control.</returns>
    public static LoraMaskControl Reserved() =>
        new(LoraMaskControlKind.Reserved, null, null, null);

    /// <summary>Converts a control that crossed the boundary.</summary>
    /// <param name="control">The control as the C ABI describes it.</param>
    /// <returns>The equivalent record.</returns>
    internal static LoraMaskControl From(PamojaLoraMaskControl control) => control.Kind switch
    {
        NativeMethods.LoraMaskGroup => OneGroup(control.Group),
        NativeMethods.LoraMaskBanks => Banks(),
        NativeMethods.LoraMaskPairedBanks => PairedBanks(),
        NativeMethods.LoraMaskAll => AllChannels(
            control.On != 0,
            control.HasThenGroup != 0 ? control.ThenGroup : null),
        _ => Reserved(),
    };

    /// <summary>Converts the control into the shape that crosses the boundary.</summary>
    /// <returns>The equivalent struct.</returns>
    internal PamojaLoraMaskControl ToNative() => new()
    {
        Kind = (byte)Kind,
        Group = Group ?? 0,
        On = (byte)(On == true ? 1 : 0),
        HasThenGroup = (byte)(ThenGroup.HasValue ? 1 : 0),
        ThenGroup = ThenGroup ?? 0,
    };
}

/// <summary>How a plan defines and uses its channels.</summary>
/// <param name="Kind">Whether the network creates channels or only switches numbered ones.</param>
/// <param name="ChannelList">
/// For a dynamic plan, the numbering it reads a type 1 channel list against, if any.
/// </param>
/// <param name="TxParamSetup">Whether devices on the plan answer <c>TXParamSetupReq</c>.</param>
/// <param name="JoinSequence">The order a device tries the join channels in.</param>
/// <param name="PowerReference">What the transmit power indexes count down from.</param>
/// <param name="GainAllowanceDb">
/// For a conducted ceiling, the antenna gain it already allows for, in dB.
/// </param>
/// <param name="DownlinkChannelBlockCount">How many downlink channel blocks the plan defines.</param>
/// <param name="JoinPlanCount">
/// How many runs of join channels select a plan, which only the published CN470-510 plans carry.
/// </param>
public readonly record struct LoraPlanRules(
    LoraPlanKind Kind,
    LoraChannelList? ChannelList,
    bool TxParamSetup,
    LoraJoinSequence JoinSequence,
    LoraPowerReference PowerReference,
    byte? GainAllowanceDb,
    ushort DownlinkChannelBlockCount,
    ushort JoinPlanCount);

/// <summary>A run of join channels that puts a device on a plan.</summary>
/// <param name="Channels">The join channels and the data rates a request may use on them.</param>
/// <param name="AcceptStartHz">Where the accept answering the first channel arrives, in hertz.</param>
/// <param name="AcceptStepHz">How far the accept frequency moves for each next channel.</param>
/// <param name="Rx2StartHz">
/// The second receive window's frequency after joining on the first channel, in hertz.
/// </param>
/// <param name="Rx2StepHz">How far that frequency moves for each next channel.</param>
/// <param name="Plan">The CN470-510 plan a join on these channels selects.</param>
public readonly record struct LoraJoinPlan(
    LoraChannelBlock Channels,
    uint AcceptStartHz,
    uint AcceptStepHz,
    uint Rx2StartHz,
    uint Rx2StepHz,
    LoraCn470Plan? Plan);

/// <summary>The run of join channels one join channel belongs to.</summary>
/// <param name="Index">The run's position, as <see cref="LoraChannelPlan.JoinPlans"/> lists it.</param>
/// <param name="Offset">The channel's place within the run.</param>
/// <param name="AcceptHz">Where the join accept for that channel arrives, in hertz.</param>
/// <param name="Rx2Hz">Where the second receive window listens once joined on it, in hertz.</param>
public readonly record struct LoraJoinPlanPlace(ushort Index, ushort Offset, uint AcceptHz, uint Rx2Hz);

/// <summary>How a data rate is carried on the air.</summary>
public enum LoraModulation
{
    /// <summary>LoRa modulation, described by a spreading factor and bandwidth.</summary>
    Lora = 0,

    /// <summary>Frequency-shift keying, described by its bitrate alone.</summary>
    Fsk = 1,

    /// <summary>Long-range frequency-hopping spread spectrum.</summary>
    LrFhss = 2,

    /// <summary>A data-rate number the region reserves, which carries nothing.</summary>
    Reserved = 3,
}

/// <summary>One data rate: what a number on the wire means for the radio.</summary>
/// <param name="Kind">How this rate is carried.</param>
/// <param name="BitrateBps">The payload bitrate in bits per second.</param>
/// <param name="BandwidthHz">The channel bandwidth in hertz, for a LoRa or LR-FHSS rate.</param>
/// <param name="SpreadingFactor">The spreading factor, for a LoRa rate.</param>
/// <param name="CodingRateNumerator">The coding-rate numerator, for an LR-FHSS rate.</param>
/// <param name="CodingRateDenominator">The coding-rate denominator, for an LR-FHSS rate.</param>
public readonly record struct LoraDataRate(
    LoraModulation Kind,
    uint BitrateBps,
    uint? BandwidthHz,
    byte? SpreadingFactor,
    byte? CodingRateNumerator,
    byte? CodingRateDenominator)
{
    /// <summary>Describes a rate carried by LoRa modulation.</summary>
    /// <param name="spreadingFactor">The spreading factor.</param>
    /// <param name="bandwidthHz">The channel bandwidth in hertz.</param>
    /// <param name="bitrateBps">The payload bitrate in bits per second.</param>
    /// <returns>The data rate.</returns>
    public static LoraDataRate ForLora(byte spreadingFactor, uint bandwidthHz, uint bitrateBps) =>
        new(LoraModulation.Lora, bitrateBps, bandwidthHz, spreadingFactor, null, null);

    /// <summary>Describes a rate carried by frequency-shift keying.</summary>
    /// <param name="bitrateBps">The payload bitrate in bits per second.</param>
    /// <returns>The data rate.</returns>
    public static LoraDataRate ForFsk(uint bitrateBps) =>
        new(LoraModulation.Fsk, bitrateBps, null, null, null, null);

    /// <summary>Describes a rate carried by frequency-hopping spread spectrum.</summary>
    /// <param name="codingRateNumerator">The coding-rate numerator.</param>
    /// <param name="codingRateDenominator">The coding-rate denominator.</param>
    /// <param name="bandwidthHz">The occupied bandwidth in hertz.</param>
    /// <param name="bitrateBps">The payload bitrate in bits per second.</param>
    /// <returns>The data rate.</returns>
    public static LoraDataRate ForLrFhss(
        byte codingRateNumerator,
        byte codingRateDenominator,
        uint bandwidthHz,
        uint bitrateBps) =>
        new(
            LoraModulation.LrFhss,
            bitrateBps,
            bandwidthHz,
            null,
            codingRateNumerator,
            codingRateDenominator);

    /// <summary>Describes a data-rate number the region reserves.</summary>
    /// <returns>The reserved slot, which carries nothing.</returns>
    public static LoraDataRate Reserved() =>
        new(LoraModulation.Reserved, 0, null, null, null, null);

    /// <summary>Converts a data rate that crossed the boundary.</summary>
    /// <param name="rate">The data rate as the C ABI describes it.</param>
    /// <returns>The equivalent record.</returns>
    internal static LoraDataRate From(PamojaLoraDataRate rate) => rate.Kind switch
    {
        NativeMethods.LoraModulationLora =>
            ForLora(rate.SpreadingFactor, rate.BandwidthHz, rate.BitrateBps),
        NativeMethods.LoraModulationFsk => ForFsk(rate.BitrateBps),
        NativeMethods.LoraModulationLrFhss => ForLrFhss(
            rate.CodingRateNumerator,
            rate.CodingRateDenominator,
            rate.BandwidthHz,
            rate.BitrateBps),
        _ => Reserved(),
    };

    /// <summary>Converts the data rate into the shape that crosses the boundary.</summary>
    /// <returns>The equivalent struct.</returns>
    internal PamojaLoraDataRate ToNative() => new()
    {
        BitrateBps = BitrateBps,
        BandwidthHz = BandwidthHz ?? 0,
        Kind = (byte)Kind,
        SpreadingFactor = SpreadingFactor ?? 0,
        CodingRateNumerator = CodingRateNumerator ?? 0,
        CodingRateDenominator = CodingRateDenominator ?? 0,
    };
}

/// <summary>What one data rate may carry in a single frame.</summary>
/// <param name="MacPayload">The largest MAC payload, frame options included, in bytes.</param>
/// <param name="Application">The largest application payload, in bytes.</param>
public readonly record struct LoraMaxPayload(ushort MacPayload, ushort Application);

/// <summary>A run of evenly spaced channels.</summary>
/// <param name="StartHz">The first channel's center frequency in hertz.</param>
/// <param name="StepHz">The spacing between channels in hertz.</param>
/// <param name="Count">How many channels the block holds.</param>
/// <param name="MinDataRate">The slowest data rate the block allows.</param>
/// <param name="MaxDataRate">The fastest data rate the block allows.</param>
public readonly record struct LoraChannelBlock(
    uint StartHz,
    uint StepHz,
    ushort Count,
    byte MinDataRate,
    byte MaxDataRate);

/// <summary>A slice of a band with its own transmit limits.</summary>
/// <param name="StartHz">The first frequency in the sub-band, in hertz.</param>
/// <param name="EndHz">The last frequency in the sub-band, in hertz.</param>
/// <param name="DutyCyclePermille">
/// The share of time a transmitter may hold the channel, in parts per thousand,
/// so <c>10</c> is one percent and <c>1000</c> is unrestricted.
/// </param>
/// <param name="MaxEirpDbm">The power ceiling in dBm EIRP.</param>
public readonly record struct LoraSubBand(
    uint StartHz,
    uint EndHz,
    uint DutyCyclePermille,
    sbyte MaxEirpDbm);

/// <summary>The Class B beacon settings of a plan.</summary>
/// <param name="FrequencyHz">The frequency the beacon is broadcast on, in hertz.</param>
/// <param name="PingSlotFrequencyHz">The default ping-slot frequency, in hertz.</param>
/// <param name="DataRate">The data rate the beacon is broadcast at.</param>
public readonly record struct LoraBeacon(
    uint FrequencyHz,
    uint PingSlotFrequencyHz,
    byte DataRate);

/// <summary>Where the second receive window listens.</summary>
/// <param name="FrequencyHz">The fixed frequency, in hertz.</param>
/// <param name="DataRate">The data rate.</param>
public readonly record struct LoraRx2(uint FrequencyHz, byte DataRate);

/// <summary>The scalar facts of a plan, read in one call.</summary>
/// <param name="Name">The specification's name for the band, such as EU863-870.</param>
/// <param name="UplinkDataRateCount">How many uplink data-rate numbers the plan defines.</param>
/// <param name="DownlinkDataRateCount">How many downlink data-rate numbers the plan defines.</param>
/// <param name="DefaultChannelCount">How many channels the plan starts a device with.</param>
/// <param name="JoinChannelBlockCount">How many join channel blocks the plan defines.</param>
/// <param name="DefaultChannelBlockCount">How many default channel blocks the plan defines.</param>
/// <param name="SubBandCount">How many sub-bands the plan defines.</param>
/// <param name="Beacon">The Class B beacon settings.</param>
/// <param name="Rx2">Where the second receive window listens.</param>
/// <param name="DefaultMaxEirpDbm">The ceiling assumed when no sub-band says otherwise.</param>
/// <param name="TxPowerStepDb">The step between transmit-power settings, in dB.</param>
/// <param name="MaxTxPowerIndex">The highest transmit-power index the plan defines.</param>
/// <param name="MaxRx1DataRateOffset">The highest RX1 data-rate offset the plan allows.</param>
/// <param name="HasDwellTimeLimit">Whether the plan limits how long a transmission may hold a channel.</param>
/// <param name="HasDwellLimitedPayloads">Whether the plan publishes a dwell-limited payload table.</param>
/// <param name="HasDwellLimitedRx1">Whether the plan publishes a dwell-limited RX1 mapping.</param>
public readonly record struct LoraPlanInfo(
    string Name,
    ushort UplinkDataRateCount,
    ushort DownlinkDataRateCount,
    ushort DefaultChannelCount,
    ushort JoinChannelBlockCount,
    ushort DefaultChannelBlockCount,
    ushort SubBandCount,
    LoraBeacon Beacon,
    LoraRx2 Rx2,
    sbyte DefaultMaxEirpDbm,
    byte TxPowerStepDb,
    byte MaxTxPowerIndex,
    byte MaxRx1DataRateOffset,
    bool HasDwellTimeLimit,
    bool HasDwellLimitedPayloads,
    bool HasDwellLimitedRx1);

/// <summary>A regional channel plan, published or private.</summary>
/// <remarks>
/// A channel plan is what a regulator and the LoRa Alliance publish about one
/// band: which data rates exist, what each carries, how much of the time a node
/// may hold a frequency, and where it listens for a downlink. The plan reports
/// those facts and costs a transmission out against them; it never refuses one,
/// because a deployment may hold licensed spectrum or be working under emergency
/// provisions and only the operator knows which.
/// <para>
/// A plan assembled by <see cref="LoraPlanBuilder"/> is the same kind of thing as
/// a published region, not a lesser one, and answers every question it does.
/// </para>
/// </remarks>
public sealed class LoraChannelPlan : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps a plan the native core produced.</summary>
    /// <param name="handle">The plan pointer.</param>
    internal LoraChannelPlan(IntPtr handle) =>
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_lora_plan_free, "channel plan");

    /// <summary>Returns the native plan pointer, for a package that builds on a plan.</summary>
    /// <returns>The pointer, valid until this plan is disposed.</returns>
    /// <remarks>
    /// A plan is one of the few things another pamoja package needs to hand back to the
    /// native core, which is why this is here; nothing else should reach for it, and the
    /// pointer must never outlive the plan it came from.
    /// </remarks>
    public IntPtr DangerousGetHandle() => _handle.DangerousGetHandle();

    /// <summary>Returns the published plan for a region.</summary>
    /// <param name="region">The band to describe.</param>
    /// <returns>The plan, which answers every question about that band.</returns>
    /// <exception cref="PamojaException">
    /// The region is not one this build of the native library carries.
    /// </exception>
    public static LoraChannelPlan ForRegion(LoraRegion region)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_for_region((uint)region, out IntPtr plan));
        return new LoraChannelPlan(plan);
    }

    /// <summary>Returns one of the CN470-510 channel plans.</summary>
    /// <param name="plan">The plan to describe.</param>
    /// <returns>
    /// The plan, which also carries the runs of join channels that select each of the others.
    /// </returns>
    /// <exception cref="PamojaException">
    /// CN470-510 is not compiled into this build of the native library.
    /// </exception>
    public static LoraChannelPlan ForCn470(LoraCn470Plan plan)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_for_cn470((uint)plan, out IntPtr handle));
        return new LoraChannelPlan(handle);
    }

    /// <summary>Reports whether a region is compiled into this build.</summary>
    /// <param name="region">The band to check.</param>
    /// <returns><c>true</c> if the region is available.</returns>
    /// <remarks>
    /// A library trimmed for a device carries only the bands it operates in, so a
    /// host offering a choice asks this before offering one.
    /// </remarks>
    public static bool IsAvailable(LoraRegion region) =>
        NativeMethods.pamoja_lora_region_is_available((uint)region) != 0;

    /// <summary>The specification's name for the band, such as EU863-870.</summary>
    public string Name =>
        OwnedString.ReadOrNull(NativeMethods.pamoja_lora_plan_name(_handle.DangerousGetHandle()))
        ?? throw new PamojaException("the plan reported no name");

    /// <summary>Returns the scalar facts of the plan.</summary>
    /// <returns>The plan's scalars.</returns>
    public LoraPlanInfo Info()
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_info(
                _handle.DangerousGetHandle(),
                out PamojaLoraPlanInfo info));
        return new LoraPlanInfo(
            Name,
            info.UplinkDataRateCount,
            info.DownlinkDataRateCount,
            info.DefaultChannelCount,
            info.JoinChannelBlockCount,
            info.DefaultChannelBlockCount,
            info.SubBandCount,
            new LoraBeacon(
                info.Beacon.FrequencyHz,
                info.Beacon.PingSlotFrequencyHz,
                info.Beacon.DataRate),
            new LoraRx2(info.Rx2FrequencyHz, info.Rx2DataRate),
            info.DefaultMaxEirpDbm,
            info.TxPowerStepDb,
            info.MaxTxPowerIndex,
            info.MaxRx1DataRateOffset,
            info.HasDwellTimeLimit != 0,
            info.HasDwellLimitedPayloads != 0,
            info.HasDwellLimitedRx1 != 0);
    }

    /// <summary>Returns the data rate a number selects.</summary>
    /// <param name="dataRate">The data-rate number.</param>
    /// <param name="direction">Which table to read; the 900 MHz plans differ.</param>
    /// <returns>
    /// The data rate, or <c>null</c> past the end of the plan's table. A number
    /// the region reserves is a rate of kind <see cref="LoraModulation.Reserved"/>,
    /// which is different from one the plan never defines.
    /// </returns>
    public LoraDataRate? DataRate(byte dataRate, LoraDirection direction = LoraDirection.Uplink)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_data_rate(
            _handle.DangerousGetHandle(),
            (uint)direction,
            dataRate,
            out PamojaLoraDataRate rate);
        return status == PamojaStatus.Ok ? LoraDataRate.From(rate) : null;
    }

    /// <summary>Returns the radio settings an uplink data rate selects.</summary>
    /// <param name="dataRate">The uplink data-rate number.</param>
    /// <returns>
    /// The settings, ready to hand to <see cref="LoraLink.AirtimeMicros"/>, or
    /// <c>null</c> if the number is reserved or not carried by LoRa.
    /// </returns>
    public LoraLink? LinkSettings(byte dataRate)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_link_settings(
            _handle.DangerousGetHandle(),
            dataRate,
            out PamojaLoraLink link);
        return status == PamojaStatus.Ok ? LoraLink.FromNative(link) : null;
    }

    /// <summary>Returns what a data rate may carry in one frame.</summary>
    /// <param name="dataRate">The data-rate number.</param>
    /// <param name="table">Which of the plan's payload tables to read.</param>
    /// <returns>The limits, or <c>null</c> where the plan publishes none.</returns>
    public LoraMaxPayload? MaxPayload(
        byte dataRate,
        LoraPayloadTable table = LoraPayloadTable.UplinkDirect)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_max_payload(
            _handle.DangerousGetHandle(),
            (uint)table,
            dataRate,
            out PamojaLoraMaxPayload payload);
        return status == PamojaStatus.Ok
            ? new LoraMaxPayload(payload.MacPayload, payload.Application)
            : null;
    }

    /// <summary>Returns the share of time a transmitter may hold a frequency.</summary>
    /// <param name="frequencyHz">The frequency in hertz.</param>
    /// <returns>
    /// The limit in parts per thousand, where <c>1000</c> means unrestricted, or
    /// <c>null</c> if the frequency falls in no sub-band this plan describes.
    /// </returns>
    /// <remarks>
    /// This reports the limit; it does not impose it. Pair it with
    /// <see cref="LoraLink.MinOffTimeMicros"/> to turn the limit into the silence a
    /// frame costs.
    /// </remarks>
    public uint? DutyCyclePermille(uint frequencyHz)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_duty_cycle_permille(
            _handle.DangerousGetHandle(),
            frequencyHz,
            out uint permille);
        return status == PamojaStatus.Ok ? permille : null;
    }

    /// <summary>Returns the power ceiling that applies at a frequency, in dBm EIRP.</summary>
    /// <param name="frequencyHz">The frequency in hertz.</param>
    /// <returns>
    /// The ceiling, falling back to the plan's default where no sub-band says
    /// otherwise.
    /// </returns>
    public sbyte MaxEirpDbm(uint frequencyHz)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_max_eirp_dbm(
                _handle.DangerousGetHandle(),
                frequencyHz,
                out sbyte dbm));
        return dbm;
    }

    /// <summary>Returns the radiated power a transmit-power index selects, in dBm.</summary>
    /// <param name="index">The transmit-power index, where zero is the ceiling.</param>
    /// <param name="maxEirpDbm">The ceiling the index steps down from.</param>
    /// <returns>
    /// The radiated power, or <c>null</c> past the highest index the plan defines.
    /// </returns>
    public sbyte? TxPowerDbm(byte index, sbyte maxEirpDbm)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_tx_power_dbm(
            _handle.DangerousGetHandle(),
            index,
            maxEirpDbm,
            out sbyte dbm);
        return status == PamojaStatus.Ok ? dbm : null;
    }

    /// <summary>Returns the downlink data rate the first receive window listens at.</summary>
    /// <param name="uplinkDataRate">The data rate the uplink was sent at.</param>
    /// <param name="offset">The RX1 data-rate offset the network assigned.</param>
    /// <param name="dwellLimited">Whether to use the dwell-limited mapping.</param>
    /// <returns>
    /// The downlink data rate, or <c>null</c> if the uplink data rate or offset is
    /// outside what the plan defines.
    /// </returns>
    public byte? Rx1DataRate(byte uplinkDataRate, byte offset, bool dwellLimited = false)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_rx1_data_rate(
            _handle.DangerousGetHandle(),
            uplinkDataRate,
            offset,
            (byte)(dwellLimited ? 1 : 0),
            out byte dataRate);
        return status == PamojaStatus.Ok ? dataRate : null;
    }

    /// <summary>Returns where the second receive window listens.</summary>
    /// <returns>The frequency and data rate.</returns>
    public LoraRx2 Rx2() => Info().Rx2;

    /// <summary>Returns the next lower data rate for adaptive back-off.</summary>
    /// <param name="dataRate">The data rate currently in use.</param>
    /// <returns>
    /// The next lower data rate, or <c>null</c> at the slowest the plan has.
    /// </returns>
    /// <remarks>
    /// A device that has lost the network steps down this chain, trading airtime
    /// for range until it is heard again.
    /// </remarks>
    public byte? NextBackoffDataRate(byte dataRate)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_next_backoff_data_rate(
            _handle.DangerousGetHandle(),
            dataRate,
            out byte lower);
        return status == PamojaStatus.Ok ? lower : null;
    }

    /// <summary>Returns the center frequency of one of the plan's default channels.</summary>
    /// <param name="channel">The channel number, counting across the default blocks.</param>
    /// <returns>
    /// The center frequency in hertz, or <c>null</c> past the last channel the
    /// plan starts a device with.
    /// </returns>
    public uint? ChannelFrequencyHz(ushort channel)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_channel_frequency_hz(
            _handle.DangerousGetHandle(),
            channel,
            out uint frequency);
        return status == PamojaStatus.Ok ? frequency : null;
    }

    /// <summary>Returns the plan's channel blocks.</summary>
    /// <param name="which">The join set, the default set, or the numbered downlink channels.</param>
    /// <returns>The blocks, in the order the plan lists them.</returns>
    public IReadOnlyList<LoraChannelBlock> ChannelBlocks(
        LoraChannelSet which = LoraChannelSet.Default)
    {
        int count = which switch
        {
            LoraChannelSet.Join => Info().JoinChannelBlockCount,
            LoraChannelSet.Downlink => Rules().DownlinkChannelBlockCount,
            _ => Info().DefaultChannelBlockCount,
        };
        List<LoraChannelBlock> blocks = new(count);
        for (ushort index = 0; index < count; index++)
        {
            Status.ThrowIfError(
                NativeMethods.pamoja_lora_plan_channel_block(
                    _handle.DangerousGetHandle(),
                    (uint)which,
                    index,
                    out PamojaLoraChannelBlock block));
            blocks.Add(new LoraChannelBlock(
                block.StartHz,
                block.StepHz,
                block.Count,
                block.MinDataRate,
                block.MaxDataRate));
        }

        return blocks;
    }

    /// <summary>Returns how the plan defines and uses its channels.</summary>
    /// <returns>The plan's channel rules.</returns>
    public LoraPlanRules Rules()
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_rules(
                _handle.DangerousGetHandle(),
                out PamojaLoraPlanRules rules));
        bool conducted = rules.PowerReference == NativeMethods.LoraPowerConducted;
        return new LoraPlanRules(
            (LoraPlanKind)rules.Kind,
            rules.ChannelList == NativeMethods.LoraChannelListNone
                ? null
                : (LoraChannelList)rules.ChannelList,
            rules.TxParamSetup != 0,
            (LoraJoinSequence)rules.JoinSequence,
            (LoraPowerReference)rules.PowerReference,
            conducted ? rules.GainAllowanceDb : null,
            rules.DownlinkChannelBlockCount,
            rules.JoinPlanCount);
    }

    /// <summary>Returns what a <c>ChMaskCntl</c> value does.</summary>
    /// <param name="value">The value, 0 to 7.</param>
    /// <returns>The control, or <c>null</c> past 7.</returns>
    public LoraMaskControl? MaskControl(byte value)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_mask_control(
            _handle.DangerousGetHandle(),
            value,
            out PamojaLoraMaskControl control);
        return status == PamojaStatus.Ok ? LoraMaskControl.From(control) : null;
    }

    /// <summary>Returns where the first receive window listens after an uplink on a channel.</summary>
    /// <param name="uplinkChannel">The channel number the uplink went out on.</param>
    /// <param name="uplinkHz">The frequency it went out on.</param>
    /// <returns>
    /// The uplink's own frequency on a plan with no numbered downlink channels, otherwise the
    /// downlink channel the uplink channel maps to, or <c>null</c> where the plan leaves the
    /// window undefined.
    /// </returns>
    public uint? Rx1FrequencyHz(ushort uplinkChannel, uint uplinkHz)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_rx1_frequency_hz(
            _handle.DangerousGetHandle(),
            uplinkChannel,
            uplinkHz,
            out uint frequency);
        return status == PamojaStatus.Ok ? frequency : null;
    }

    /// <summary>Returns the frequency of a numbered downlink channel.</summary>
    /// <param name="channel">The downlink channel number, counting across the downlink blocks.</param>
    /// <returns>The frequency in hertz, or <c>null</c> past the last downlink channel.</returns>
    public uint? DownlinkChannelFrequencyHz(ushort channel)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_downlink_channel_frequency_hz(
            _handle.DangerousGetHandle(),
            channel,
            out uint frequency);
        return status == PamojaStatus.Ok ? frequency : null;
    }

    /// <summary>Returns the runs of join channels that select a plan.</summary>
    /// <returns>
    /// The runs, in join channel order; empty for every plan but the published CN470-510 ones.
    /// </returns>
    public IReadOnlyList<LoraJoinPlan> JoinPlans()
    {
        int count = Rules().JoinPlanCount;
        List<LoraJoinPlan> runs = new(count);
        for (ushort index = 0; index < count; index++)
        {
            Status.ThrowIfError(
                NativeMethods.pamoja_lora_plan_join_plan(
                    _handle.DangerousGetHandle(),
                    index,
                    out PamojaLoraJoinPlan run));
            runs.Add(new LoraJoinPlan(
                new LoraChannelBlock(
                    run.Channels.StartHz,
                    run.Channels.StepHz,
                    run.Channels.Count,
                    run.Channels.MinDataRate,
                    run.Channels.MaxDataRate),
                run.AcceptStartHz,
                run.AcceptStepHz,
                run.Rx2StartHz,
                run.Rx2StepHz,
                run.Cn470Plan == NativeMethods.LoraCn470None ? null : (LoraCn470Plan)run.Cn470Plan));
        }

        return runs;
    }

    /// <summary>Returns the run of join channels a join channel belongs to.</summary>
    /// <param name="joinChannel">The join channel, counted through the runs in order.</param>
    /// <returns>
    /// Where the channel sits and where its accept and second receive window fall, or
    /// <c>null</c> if no run holds it.
    /// </returns>
    public LoraJoinPlanPlace? JoinPlanForChannel(ushort joinChannel)
    {
        PamojaStatus status = NativeMethods.pamoja_lora_plan_join_plan_for_channel(
            _handle.DangerousGetHandle(),
            joinChannel,
            out ushort index,
            out ushort offset);
        if (status != PamojaStatus.Ok)
        {
            return null;
        }

        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_join_plan(
                _handle.DangerousGetHandle(),
                index,
                out PamojaLoraJoinPlan run));
        return new LoraJoinPlanPlace(
            index,
            offset,
            run.AcceptStartHz + (run.AcceptStepHz * offset),
            run.Rx2StartHz + (run.Rx2StepHz * offset));
    }

    /// <summary>Returns the plan's sub-bands and the transmit limits inside each.</summary>
    /// <returns>The sub-bands, in the order the plan lists them.</returns>
    public IReadOnlyList<LoraSubBand> SubBands()
    {
        int count = Info().SubBandCount;
        List<LoraSubBand> bands = new(count);
        for (ushort index = 0; index < count; index++)
        {
            Status.ThrowIfError(
                NativeMethods.pamoja_lora_plan_sub_band(
                    _handle.DangerousGetHandle(),
                    index,
                    out PamojaLoraSubBand band));
            bands.Add(new LoraSubBand(
                band.StartHz,
                band.EndHz,
                band.DutyCyclePermille,
                band.MaxEirpDbm));
        }

        return bands;
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
