using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// The radio settings of a LoRa link, mirroring <c>PamojaLoraLink</c> in
/// <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// Values outside the ranges LoRa defines are clamped when the link is used: the
/// spreading factor to 7-12 and the coding-rate denominator to 5-8.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLoraLink
{
    /// <summary>The channel bandwidth in hertz, such as 125000.</summary>
    public uint BandwidthHz;

    /// <summary>The preamble length in symbols; the LoRa default is 8.</summary>
    public ushort PreambleSymbols;

    /// <summary>The spreading factor, 7 (fastest) to 12 (longest range).</summary>
    public byte SpreadingFactor;

    /// <summary>The coding-rate denominator, 5 to 8, for 4/5 to 4/8.</summary>
    public byte CodingRateDenominator;

    /// <summary><c>1</c> for an explicit header, <c>0</c> to omit the header symbols.</summary>
    public byte ExplicitHeader;

    /// <summary><c>1</c> to append the frame CRC, <c>0</c> to leave it off.</summary>
    public byte Crc;
}
