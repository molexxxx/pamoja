using Pamoja.Lora;
using Pamoja.Native.Interop;

namespace Pamoja.Radios;

/// <summary>Describes the LoRa types the way the C ABI carries them.</summary>
internal static class NativeLora
{
    /// <summary>Describes link settings for the C ABI.</summary>
    /// <param name="link">The link settings.</param>
    /// <returns>The settings as the C ABI carries them.</returns>
    public static PamojaLoraLink Link(LoraLink link) => new()
    {
        BandwidthHz = link.BandwidthHz,
        PreambleSymbols = link.PreambleSymbols,
        SpreadingFactor = link.SpreadingFactor,
        CodingRateDenominator = link.CodingRateDenominator,
        ExplicitHeader = link.ExplicitHeader ? (byte)1 : (byte)0,
        Crc = link.Crc ? (byte)1 : (byte)0,
    };

    /// <summary>Reads link settings the C ABI carries.</summary>
    /// <param name="link">The settings as the C ABI carries them.</param>
    /// <returns>The link settings.</returns>
    public static LoraLink FromLink(PamojaLoraLink link)
    {
        LoraLink built = new LoraLink(link.SpreadingFactor, link.BandwidthHz)
            .WithCodingRate(link.CodingRateDenominator)
            .WithPreamble(link.PreambleSymbols);
        if (link.ExplicitHeader == 0)
        {
            built = built.WithImplicitHeader();
        }

        return link.Crc == 0 ? built.WithoutCrc() : built;
    }

    /// <summary>Describes a link budget for the C ABI.</summary>
    /// <param name="budget">The link budget.</param>
    /// <returns>The budget in hundredths of a decibel.</returns>
    public static PamojaLoraLinkBudget Budget(LoraLinkBudget budget) => new()
    {
        TransmitPowerCentiDbm = Centi(budget.TransmitPowerDbm),
        TransmitAntennaGainCentiDbi = Centi(budget.TransmitAntennaGainDbi),
        TransmitCableLossCentiDb = Centi(budget.TransmitCableLossDb),
        ReceiveAntennaGainCentiDbi = Centi(budget.ReceiveAntennaGainDbi),
        ReceiveCableLossCentiDb = Centi(budget.ReceiveCableLossDb),
        NoiseFigureCentiDb = Centi(budget.NoiseFigureDb),
    };

    /// <summary>Resolves a number of decibels to hundredths of a decibel.</summary>
    /// <param name="db">The value in decibels.</param>
    /// <returns>The nearest hundredth of a decibel.</returns>
    public static int Centi(double db) => (int)Math.Round(db * 100, MidpointRounding.AwayFromZero);

    /// <summary>Returns hundredths of a decibel as a number of decibels.</summary>
    /// <param name="centi">The value in hundredths of a decibel.</param>
    /// <returns>The value in decibels.</returns>
    public static double Db(int centi) => centi / 100.0;
}
