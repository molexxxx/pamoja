using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A decoded DS18B20 scratchpad, mirroring <c>PamojaDs18b20Reading</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaDs18b20Reading
{
    /// <summary>The raw temperature register, 1/16 degree Celsius per count.</summary>
    public short RawTemperature;

    /// <summary>The temperature in micro-degrees Celsius.</summary>
    public int MicroCelsius;

    /// <summary>The high alarm threshold in whole degrees Celsius.</summary>
    public sbyte AlarmHigh;

    /// <summary>The low alarm threshold in whole degrees Celsius.</summary>
    public sbyte AlarmLow;

    /// <summary>The configured resolution in bits: 9, 10, 11, or 12.</summary>
    public byte ResolutionBits;
}
