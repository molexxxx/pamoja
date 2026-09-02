using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>The radio settings of a LoRa link, and what they cost in airtime.</summary>
/// <remarks>
/// LoRa buys kilometres of range on license-free bands at tiny power, and the
/// price is time: a transmission occupies the channel for a duration these
/// settings fix, and the regional rules cap how much of the time a node may
/// transmit. Values outside the ranges LoRa defines are clamped when used.
/// </remarks>
public sealed class LoraLink
{
    private readonly PamojaLoraLink _link;

    /// <summary>Creates link settings with the LoRa defaults filled in.</summary>
    /// <param name="spreadingFactor">
    /// The spreading factor, clamped to 5 (fastest) to 12 (longest range).
    /// </param>
    /// <param name="bandwidthHz">The channel bandwidth in hertz, such as 125000.</param>
    public LoraLink(byte spreadingFactor, uint bandwidthHz)
        : this(NativeMethods.pamoja_lora_link_default(spreadingFactor, bandwidthHz))
    {
    }

    /// <summary>Wraps the settings the native core reported.</summary>
    /// <param name="link">The settings as the C ABI describes them.</param>
    private LoraLink(PamojaLoraLink link) => _link = link;

    /// <summary>The spreading factor, 7 (fastest) to 12 (longest range).</summary>
    public byte SpreadingFactor => _link.SpreadingFactor;

    /// <summary>The channel bandwidth in hertz.</summary>
    public uint BandwidthHz => _link.BandwidthHz;

    /// <summary>The coding-rate denominator, 5 to 8, for 4/5 to 4/8.</summary>
    public byte CodingRateDenominator => _link.CodingRateDenominator;

    /// <summary>The preamble length in symbols.</summary>
    public ushort PreambleSymbols => _link.PreambleSymbols;

    /// <summary>Whether the frame carries an explicit header.</summary>
    public bool ExplicitHeader => _link.ExplicitHeader != 0;

    /// <summary>Whether the frame carries a CRC.</summary>
    public bool Crc => _link.Crc != 0;

    /// <summary>Returns the same link at a different coding rate.</summary>
    /// <param name="denominator">The denominator, clamped to 5 to 8.</param>
    /// <returns>The adjusted settings.</returns>
    public LoraLink WithCodingRate(byte denominator)
    {
        PamojaLoraLink link = _link;
        link.CodingRateDenominator = denominator;
        return new LoraLink(link);
    }

    /// <summary>Returns the same link with a different preamble length.</summary>
    /// <param name="symbols">The preamble length in symbols.</param>
    /// <returns>The adjusted settings.</returns>
    public LoraLink WithPreamble(ushort symbols)
    {
        PamojaLoraLink link = _link;
        link.PreambleSymbols = symbols;
        return new LoraLink(link);
    }

    /// <summary>Returns the same link with an implicit header, saving its symbols.</summary>
    /// <returns>The adjusted settings.</returns>
    public LoraLink WithImplicitHeader()
    {
        PamojaLoraLink link = _link;
        link.ExplicitHeader = 0;
        return new LoraLink(link);
    }

    /// <summary>Returns the same link with the frame CRC turned off.</summary>
    /// <returns>The adjusted settings.</returns>
    public LoraLink WithoutCrc()
    {
        PamojaLoraLink link = _link;
        link.Crc = 0;
        return new LoraLink(link);
    }

    /// <summary>The duration of one symbol on this link, in microseconds.</summary>
    public ulong SymbolTimeMicros => NativeMethods.pamoja_lora_symbol_time_us(_link);

    /// <summary>Returns the time on air of a payload, in microseconds.</summary>
    /// <param name="payloadLength">The payload length in bytes.</param>
    /// <returns>
    /// The channel occupancy the transmission costs, which sets both the
    /// duty-cycle budget and most of the energy it spends.
    /// </returns>
    public ulong AirtimeMicros(int payloadLength) =>
        NativeMethods.pamoja_lora_airtime_us(_link, (nuint)payloadLength);

    /// <summary>Returns the silence a duty-cycle limit forces after a transmission.</summary>
    /// <param name="payloadLength">The payload length in bytes.</param>
    /// <param name="dutyCyclePermille">
    /// The limit in parts per thousand, so 10 is 1%.
    /// </param>
    /// <returns>
    /// The required off time in microseconds, or <c>null</c> when the limit is
    /// zero, which forbids transmitting at all.
    /// </returns>
    public ulong? MinOffTimeMicros(int payloadLength, uint dutyCyclePermille)
    {
        if (dutyCyclePermille == 0)
        {
            return null;
        }

        return NativeMethods.pamoja_lora_min_off_time_us(
            _link, (nuint)payloadLength, dutyCyclePermille);
    }

    /// <summary>Returns how many transmissions of a payload fit in an hour.</summary>
    /// <param name="payloadLength">The payload length in bytes.</param>
    /// <param name="dutyCyclePermille">
    /// The limit in parts per thousand, so 10 is 1%.
    /// </param>
    /// <returns>
    /// The number of whole transmissions per hour, or 0 when the limit forbids
    /// transmitting. The airtime plus the silence it forces is what one
    /// transmission really costs, so this is the budget a deployment plans
    /// against.
    /// </returns>
    public ulong MessagesPerHour(int payloadLength, uint dutyCyclePermille)
    {
        ulong? offTime = MinOffTimeMicros(payloadLength, dutyCyclePermille);
        if (offTime is null)
        {
            return 0;
        }

        return 3_600_000_000UL / (AirtimeMicros(payloadLength) + offTime.Value);
    }
}
