using Pamoja.Core.Interop;

namespace Pamoja.Core;

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
}

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
/// <param name="StartHz">The first channel's centre frequency in hertz.</param>
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

    /// <summary>Returns the published plan for a region.</summary>
    /// <param name="region">The band to describe.</param>
    /// <returns>The plan, which answers every question about that band.</returns>
    /// <exception cref="PamojaException">
    /// The region is not one this build of the native library carries.
    /// </exception>
    public static LoraChannelPlan ForRegion(LoraRegion region)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_lora_plan_for_region((uint)region, out IntPtr plan));
        return new LoraChannelPlan(plan);
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
        PamojaCore.ThrowIfError(
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
        PamojaCore.ThrowIfError(
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

    /// <summary>Returns the centre frequency of one of the plan's default channels.</summary>
    /// <param name="channel">The channel number, counting across the default blocks.</param>
    /// <returns>
    /// The centre frequency in hertz, or <c>null</c> past the last channel the
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
    /// <param name="which">The join set or the default set.</param>
    /// <returns>The blocks, in the order the plan lists them.</returns>
    public IReadOnlyList<LoraChannelBlock> ChannelBlocks(
        LoraChannelSet which = LoraChannelSet.Default)
    {
        LoraPlanInfo info = Info();
        int count = which == LoraChannelSet.Join
            ? info.JoinChannelBlockCount
            : info.DefaultChannelBlockCount;
        List<LoraChannelBlock> blocks = new(count);
        for (ushort index = 0; index < count; index++)
        {
            PamojaCore.ThrowIfError(
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

    /// <summary>Returns the plan's sub-bands and the transmit limits inside each.</summary>
    /// <returns>The sub-bands, in the order the plan lists them.</returns>
    public IReadOnlyList<LoraSubBand> SubBands()
    {
        int count = Info().SubBandCount;
        List<LoraSubBand> bands = new(count);
        for (ushort index = 0; index < count; index++)
        {
            PamojaCore.ThrowIfError(
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
