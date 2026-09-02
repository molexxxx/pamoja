using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// One data rate, mirroring <c>PamojaLoraDataRate</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// <see cref="Kind"/> selects which fields carry meaning. A LoRa rate uses
/// <see cref="SpreadingFactor"/> and <see cref="BandwidthHz"/>; an LR-FHSS rate
/// uses the coding-rate pair and <see cref="BandwidthHz"/>; an FSK rate uses
/// <see cref="BitrateBps"/> alone. A reserved number leaves every field zero.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraDataRate
{
    /// <summary>The payload bitrate in bits per second.</summary>
    public uint BitrateBps;

    /// <summary>The channel bandwidth in hertz, or zero for FSK.</summary>
    public uint BandwidthHz;

    /// <summary>One of the <c>PAMOJA_LORA_MODULATION_*</c> constants.</summary>
    public byte Kind;

    /// <summary>The spreading factor, for a LoRa rate.</summary>
    public byte SpreadingFactor;

    /// <summary>The coding-rate numerator, for an LR-FHSS rate.</summary>
    public byte CodingRateNumerator;

    /// <summary>The coding-rate denominator, for an LR-FHSS rate.</summary>
    public byte CodingRateDenominator;
}

/// <summary>
/// What one data rate may carry in a single frame, mirroring
/// <c>PamojaLoraMaxPayload</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraMaxPayload
{
    /// <summary>The largest MAC payload, frame options included, in bytes.</summary>
    public ushort MacPayload;

    /// <summary>The largest application payload, in bytes.</summary>
    public ushort Application;
}

/// <summary>
/// A run of evenly spaced channels, mirroring <c>PamojaLoraChannelBlock</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraChannelBlock
{
    /// <summary>The first channel's centre frequency in hertz.</summary>
    public uint StartHz;

    /// <summary>The spacing between channels in hertz.</summary>
    public uint StepHz;

    /// <summary>How many channels the block holds.</summary>
    public ushort Count;

    /// <summary>The slowest data rate the block allows.</summary>
    public byte MinDataRate;

    /// <summary>The fastest data rate the block allows.</summary>
    public byte MaxDataRate;
}

/// <summary>
/// A slice of a band with its own transmit limits, mirroring
/// <c>PamojaLoraSubBand</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraSubBand
{
    /// <summary>The first frequency in the sub-band, in hertz.</summary>
    public uint StartHz;

    /// <summary>The last frequency in the sub-band, in hertz.</summary>
    public uint EndHz;

    /// <summary>
    /// The share of time a transmitter may hold the channel, in parts per
    /// thousand, so <c>10</c> is one percent and <c>1000</c> is unrestricted.
    /// </summary>
    public uint DutyCyclePermille;

    /// <summary>The power ceiling in dBm EIRP.</summary>
    public sbyte MaxEirpDbm;
}

/// <summary>
/// The Class B beacon settings of a plan, mirroring <c>PamojaLoraBeacon</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraBeacon
{
    /// <summary>The frequency the beacon is broadcast on, in hertz.</summary>
    public uint FrequencyHz;

    /// <summary>The default ping-slot frequency, in hertz.</summary>
    public uint PingSlotFrequencyHz;

    /// <summary>The data rate the beacon is broadcast at.</summary>
    public byte DataRate;
}

/// <summary>
/// The scalar facts of a plan, mirroring <c>PamojaLoraPlanInfo</c> in
/// <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// The three flags cross as bytes rather than <see cref="bool"/> because runtime
/// marshalling is disabled, which leaves a <see cref="bool"/> field non-blittable.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraPlanInfo
{
    /// <summary>The fixed frequency the second receive window listens on, in hertz.</summary>
    public uint Rx2FrequencyHz;

    /// <summary>How many uplink data-rate numbers the plan defines, reserved included.</summary>
    public ushort UplinkDataRateCount;

    /// <summary>How many downlink data-rate numbers the plan defines.</summary>
    public ushort DownlinkDataRateCount;

    /// <summary>How many channels the plan starts a device with.</summary>
    public ushort DefaultChannelCount;

    /// <summary>How many join channel blocks the plan defines.</summary>
    public ushort JoinChannelBlockCount;

    /// <summary>How many default channel blocks the plan defines.</summary>
    public ushort DefaultChannelBlockCount;

    /// <summary>How many sub-bands the plan defines.</summary>
    public ushort SubBandCount;

    /// <summary>The Class B beacon settings.</summary>
    public PamojaLoraBeacon Beacon;

    /// <summary>The data rate the second receive window listens at.</summary>
    public byte Rx2DataRate;

    /// <summary>The power ceiling assumed when no sub-band says otherwise, in dBm.</summary>
    public sbyte DefaultMaxEirpDbm;

    /// <summary>The step between transmit-power settings, in dB.</summary>
    public byte TxPowerStepDb;

    /// <summary>The highest transmit-power index the plan defines.</summary>
    public byte MaxTxPowerIndex;

    /// <summary>The highest RX1 data-rate offset the plan allows.</summary>
    public byte MaxRx1DataRateOffset;

    /// <summary><c>1</c> if the plan limits how long one transmission may hold a channel.</summary>
    public byte HasDwellTimeLimit;

    /// <summary><c>1</c> if the plan publishes a payload table for a dwell-limited device.</summary>
    public byte HasDwellLimitedPayloads;

    /// <summary><c>1</c> if the plan publishes a second RX1 mapping for a dwell-limited downlink.</summary>
    public byte HasDwellLimitedRx1;
}
