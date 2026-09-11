using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The gains and losses of a LoRa link, mirroring <c>PamojaLoraLinkBudget</c> in
/// <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// Every field is in hundredths of a decibel, so 1400 is 14 dBm and 215 is 2.15 dBi.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraLinkBudget
{
    /// <summary>
    /// The power the transmitting radio delivers at its antenna port, in hundredths of a dBm.
    /// </summary>
    public int TransmitPowerCentiDbm;

    /// <summary>The gain of the transmitting antenna, in hundredths of a dBi.</summary>
    public int TransmitAntennaGainCentiDbi;

    /// <summary>
    /// The loss between the transmitting radio and its antenna, in hundredths of a dB.
    /// </summary>
    public int TransmitCableLossCentiDb;

    /// <summary>The gain of the receiving antenna, in hundredths of a dBi.</summary>
    public int ReceiveAntennaGainCentiDbi;

    /// <summary>
    /// The loss between the receiving antenna and its radio, in hundredths of a dB.
    /// </summary>
    public int ReceiveCableLossCentiDb;

    /// <summary>The noise figure of the receiver, in hundredths of a dB.</summary>
    public int NoiseFigureCentiDb;
}
