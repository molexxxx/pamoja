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

    /// <summary>The uplink direction, for a table that differs between the two.</summary>
    public const uint LoraDirectionUplink = 0;

    /// <summary>The downlink direction, for a table that differs between the two.</summary>
    public const uint LoraDirectionDownlink = 1;

    /// <summary>Returns the published channel plan for a region.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_for_region(uint region, out IntPtr outPlan);

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

    /// <summary>Returns the centre frequency of one of the plan's default channels.</summary>
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

    /// <summary>Finishes a plan, consuming the builder.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_plan_builder_build(
        IntPtr builder,
        out IntPtr outPlan);
}
