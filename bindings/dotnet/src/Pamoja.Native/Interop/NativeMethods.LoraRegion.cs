using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for LoRaWAN regional channel plans, mirroring
/// <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The EU863-870 band.</summary>
    public const uint LoraRegionEu868 = 1;

    /// <summary>The US902-928 band.</summary>
    public const uint LoraRegionUs915 = 2;

    /// <summary>The EU433 band.</summary>
    public const uint LoraRegionEu433 = 3;

    /// <summary>The AU915-928 band.</summary>
    public const uint LoraRegionAu915 = 4;

    /// <summary>The CN470-510 band.</summary>
    public const uint LoraRegionCn470 = 5;

    /// <summary>The AS923 band.</summary>
    public const uint LoraRegionAs923 = 6;

    /// <summary>The KR920-923 band.</summary>
    public const uint LoraRegionKr920 = 7;

    /// <summary>The IN865-867 band.</summary>
    public const uint LoraRegionIn865 = 8;

    /// <summary>The RU864-870 band.</summary>
    public const uint LoraRegionRu864 = 9;

    /// <summary>A data rate carried by LoRa modulation.</summary>
    public const byte LoraModulationLora = 0;

    /// <summary>A data rate carried by FSK modulation.</summary>
    public const byte LoraModulationFsk = 1;

    /// <summary>A data rate carried by long-range frequency-hopping spread spectrum.</summary>
    public const byte LoraModulationLrFhss = 2;

    /// <summary>A data-rate number the region reserves, which carries nothing.</summary>
    public const byte LoraModulationReserved = 3;

    /// <summary>The uplink payload limits for a device that may sit behind a repeater.</summary>
    public const uint LoraPayloadTableUplinkRepeater = 0;

    /// <summary>The uplink payload limits for a device that will not.</summary>
    public const uint LoraPayloadTableUplinkDirect = 1;

    /// <summary>The downlink payload limits for a device that may sit behind a repeater.</summary>
    public const uint LoraPayloadTableDownlinkRepeater = 2;

    /// <summary>The downlink payload limits for a device that will not.</summary>
    public const uint LoraPayloadTableDownlinkDirect = 3;

    /// <summary>The payload limits that apply under a dwell-time limit.</summary>
    public const uint LoraPayloadTableDwellLimited = 4;

    /// <summary>The channels a device must use to send a join request.</summary>
    public const uint LoraChannelsJoin = 0;

    /// <summary>The channels a device starts with before a network adds any.</summary>
    public const uint LoraChannelsDefault = 1;

    /// <summary>The numbered downlink channels a fixed plan answers the first receive window on.</summary>
    public const uint LoraChannelsDownlink = 2;

    /// <summary>A plan whose network creates channels and moves them.</summary>
    public const byte LoraPlanKindDynamic = 0;

    /// <summary>A plan whose channels are numbered in advance and only enabled or disabled.</summary>
    public const byte LoraPlanKindFixed = 1;

    /// <summary>A plan that reads a type 1 channel list against no numbering.</summary>
    public const byte LoraChannelListNone = 0;

    /// <summary>The 800 MHz channel list numbering.</summary>
    public const byte LoraChannelListMhz800 = 1;

    /// <summary>The 900 MHz channel list numbering.</summary>
    public const byte LoraChannelListMhz900 = 2;

    /// <summary>Join channels chosen at random.</summary>
    public const byte LoraJoinRandom = 0;

    /// <summary>Join channels tried in octet passes.</summary>
    public const byte LoraJoinOctetPasses = 1;

    /// <summary>Power indexes that count down from a radiated ceiling.</summary>
    public const byte LoraPowerEirp = 0;

    /// <summary>Power indexes that count down from a conducted ceiling.</summary>
    public const byte LoraPowerConducted = 1;

    /// <summary>A mask control that sets one group of sixteen channels.</summary>
    public const byte LoraMaskGroup = 0;

    /// <summary>A mask control whose ten low bits switch banks of eight.</summary>
    public const byte LoraMaskBanks = 1;

    /// <summary>A mask control that switches banks of eight with their 500 kHz channel.</summary>
    public const byte LoraMaskPairedBanks = 2;

    /// <summary>A mask control that turns every channel on or off, then sets a group.</summary>
    public const byte LoraMaskAll = 3;

    /// <summary>A mask control the region reserves.</summary>
    public const byte LoraMaskReserved = 4;

    /// <summary>No CN470-510 plan.</summary>
    public const uint LoraCn470None = 0;

    /// <summary>The CN470-510 plan for a 20 MHz antenna, type A.</summary>
    public const uint LoraCn470Antenna20MhzA = 1;

    /// <summary>The CN470-510 plan for a 20 MHz antenna, type B.</summary>
    public const uint LoraCn470Antenna20MhzB = 2;

    /// <summary>The CN470-510 plan for a 26 MHz antenna, type A.</summary>
    public const uint LoraCn470Antenna26MhzA = 3;

    /// <summary>The CN470-510 plan for a 26 MHz antenna, type B.</summary>
    public const uint LoraCn470Antenna26MhzB = 4;

    /// <summary>The 96-channel CN470-510 plan.</summary>
    public const uint LoraCn470Channels96 = 5;

    /// <summary>The uplink direction, for a table that differs between the two.</summary>
    public const uint LoraDirectionUplink = 0;

    /// <summary>The downlink direction, for a table that differs between the two.</summary>
    public const uint LoraDirectionDownlink = 1;

    /// <summary>Returns the published channel plan for a region.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_for_region(uint region, out IntPtr outPlan);

    /// <summary>Returns one of the CN470-510 channel plans.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_for_cn470(uint which, out IntPtr outPlan);

    /// <summary>Reads how a plan defines and uses its channels.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_rules(
        IntPtr plan,
        out PamojaLoraPlanRules outRules);

    /// <summary>Reads what one <c>ChMaskCntl</c> value does on a plan.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_mask_control(
        IntPtr plan,
        byte value,
        out PamojaLoraMaskControl outControl);

    /// <summary>Returns where the first receive window listens after an uplink.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_rx1_frequency_hz(
        IntPtr plan,
        ushort uplinkChannel,
        uint uplinkHz,
        out uint outFrequencyHz);

    /// <summary>Returns the frequency of one of the plan's numbered downlink channels.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_downlink_channel_frequency_hz(
        IntPtr plan,
        ushort channel,
        out uint outFrequencyHz);

    /// <summary>Returns one run of join channels that selects a plan.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_join_plan(
        IntPtr plan,
        ushort index,
        out PamojaLoraJoinPlan outJoinPlan);

    /// <summary>Finds the run of join channels a join channel belongs to.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_join_plan_for_channel(
        IntPtr plan,
        ushort joinChannel,
        out ushort outIndex,
        out ushort outOffset);

    /// <summary>Reports whether a region is compiled into this build.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_lora_region_is_available(uint region);

    /// <summary>Releases a channel plan.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lora_plan_free(IntPtr plan);

    /// <summary>Returns the plan's name as an owned string.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_lora_plan_name(IntPtr plan);

    /// <summary>Reads the scalar facts of a plan in one call.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_info(
        IntPtr plan,
        out PamojaLoraPlanInfo outInfo);

    /// <summary>Returns the data rate a number selects.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_data_rate(
        IntPtr plan,
        uint direction,
        byte dataRate,
        out PamojaLoraDataRate outRate);

    /// <summary>Returns the radio settings an uplink data rate selects.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_link_settings(
        IntPtr plan,
        byte dataRate,
        out PamojaLoraLink outLink);

    /// <summary>Returns what a data rate may carry in one frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_max_payload(
        IntPtr plan,
        uint table,
        byte dataRate,
        out PamojaLoraMaxPayload outPayload);

    /// <summary>Returns the share of time a transmitter may hold a frequency.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_duty_cycle_permille(
        IntPtr plan,
        uint frequencyHz,
        out uint outPermille);

    /// <summary>Returns the power ceiling that applies at a frequency, in dBm EIRP.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_max_eirp_dbm(
        IntPtr plan,
        uint frequencyHz,
        out sbyte outDbm);

    /// <summary>Returns the radiated power a transmit-power index selects, in dBm.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_tx_power_dbm(
        IntPtr plan,
        byte index,
        sbyte maxEirpDbm,
        out sbyte outDbm);

    /// <summary>Returns the downlink data rate the first receive window listens at.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_rx1_data_rate(
        IntPtr plan,
        byte uplinkDataRate,
        byte offset,
        byte dwellLimited,
        out byte outDataRate);

    /// <summary>Returns the next lower data rate for adaptive back-off.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_next_backoff_data_rate(
        IntPtr plan,
        byte dataRate,
        out byte outDataRate);

    /// <summary>Returns the center frequency of one of the plan's default channels.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_channel_frequency_hz(
        IntPtr plan,
        ushort channel,
        out uint outFrequencyHz);

    /// <summary>Returns one of the plan's channel blocks.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_channel_block(
        IntPtr plan,
        uint which,
        ushort index,
        out PamojaLoraChannelBlock outBlock);

    /// <summary>Returns one of the plan's sub-bands.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_sub_band(
        IntPtr plan,
        ushort index,
        out PamojaLoraSubBand outBand);

    /// <summary>Creates an empty plan builder.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_lora_plan_builder_new(
        string name,
        out IntPtr outBuilder);

    /// <summary>Releases a plan builder that will not be built.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lora_plan_builder_free(IntPtr builder);

    /// <summary>Appends a data rate to the end of a direction's table.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_push_data_rate(
        IntPtr builder,
        uint direction,
        in PamojaLoraDataRate rate);

    /// <summary>Appends a payload limit to the end of one of the plan's tables.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_push_max_payload(
        IntPtr builder,
        uint table,
        byte present,
        ushort macPayload,
        ushort application);

    /// <summary>Appends a run of evenly spaced channels to the plan.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_push_channel_block(
        IntPtr builder,
        uint which,
        in PamojaLoraChannelBlock block);

    /// <summary>Appends a sub-band and its transmit limits to the plan.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_push_sub_band(
        IntPtr builder,
        in PamojaLoraSubBand band);

    /// <summary>Appends one uplink data rate's row of RX1 downlink data rates.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_push_rx1_row(
        IntPtr builder,
        byte dwellLimited,
        ReadOnlySpan<byte> offsets,
        nuint offsetsLen);

    /// <summary>Appends the next entry in the adaptive back-off chain.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_push_backoff(
        IntPtr builder,
        byte hasLower,
        byte dataRate);

    /// <summary>Sets the plan's transmit-power ladder.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_set_power(
        IntPtr builder,
        sbyte defaultMaxEirpDbm,
        byte txPowerStepDb,
        byte maxTxPowerIndex);

    /// <summary>Sets the plan's receive windows.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_set_rx(
        IntPtr builder,
        uint rx2FrequencyHz,
        byte rx2DataRate,
        byte maxRx1DataRateOffset);

    /// <summary>Sets the plan's Class B beacon and whether it limits dwell time.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_set_beacon(
        IntPtr builder,
        in PamojaLoraBeacon beacon,
        byte hasDwellTimeLimit);

    /// <summary>Sets whether the plan's network creates channels, and its channel list numbering.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_set_kind(
        IntPtr builder,
        byte kind,
        byte channelList);

    /// <summary>Sets whether devices on the plan answer <c>TXParamSetupReq</c>.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_set_tx_param_setup(
        IntPtr builder,
        byte answered);

    /// <summary>Sets what each <c>ChMaskCntl</c> value does.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_set_mask_controls(
        IntPtr builder,
        ReadOnlySpan<PamojaLoraMaskControl> controls,
        nuint len);

    /// <summary>Sets the order a device tries the join channels in.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_set_join_sequence(
        IntPtr builder,
        byte sequence);

    /// <summary>Sets what the plan's transmit power indexes count down from.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_set_power_reference(
        IntPtr builder,
        byte reference,
        byte gainAllowanceDb);

    /// <summary>Finishes a plan, consuming the builder.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_build(
        IntPtr builder,
        out IntPtr outPlan);
}
