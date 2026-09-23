using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// How a BMP280 driver measures. Mirrors <c>PamojaBmp280Settings</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBmp280Settings
{
    /// <summary>The temperature oversampling code, 0 to 5, where 0 skips the measurement.</summary>
    public byte Temperature;
    /// <summary>The pressure oversampling code, 0 to 5, where 0 skips the measurement.</summary>
    public byte Pressure;
    /// <summary>The IIR filter code, 0 for the filter off, written as given.</summary>
    public byte Filter;
}

/// <summary>
/// A TMP117 temperature result. Mirrors <c>PamojaTmp117Reading</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaTmp117Reading
{
    /// <summary>The temperature register, 7.8125 millidegrees per count.</summary>
    public short Raw;
    /// <summary>The temperature in micro-degrees Celsius.</summary>
    public int MicroCelsius;
    /// <summary>The temperature in degrees Celsius.</summary>
    public float Celsius;
}

/// <summary>
/// The TMP117's alert flags. Mirrors <c>PamojaTmp117Alerts</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaTmp117Alerts
{
    /// <summary>1 when a result was above the high limit.</summary>
    public byte High;
    /// <summary>1 when a result was below the low limit.</summary>
    public byte Low;
}

/// <summary>
/// How an OPT3001 driver measures. Mirrors <c>PamojaOpt3001Settings</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaOpt3001Settings
{
    /// <summary>1 integrates each conversion for 800 ms, 0 for 100 ms.</summary>
    public byte LongConversion;
    /// <summary>The full-scale range number, 0 to 11, or 12 to let the part choose.</summary>
    public byte RangeNumber;
}

/// <summary>
/// An OPT3001 illuminance result. Mirrors <c>PamojaOpt3001Reading</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaOpt3001Reading
{
    /// <summary>The result register: a four-bit exponent over a twelve-bit mantissa.</summary>
    public ushort Raw;
    /// <summary>The illuminance in millilux.</summary>
    public uint MilliLux;
    /// <summary>The illuminance in lux.</summary>
    public float Lux;
}

/// <summary>
/// How an HDC1080 driver measures. Mirrors <c>PamojaHdc1080Settings</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaHdc1080Settings
{
    /// <summary>The temperature resolution in bits: 14 or 11.</summary>
    public byte TemperatureResolutionBits;
    /// <summary>The humidity resolution in bits: 14, 11, or 8.</summary>
    public byte HumidityResolutionBits;
}

/// <summary>
/// An INA219 configuration register, field by field. Mirrors <c>PamojaIna219Config</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaIna219Config
{
    /// <summary>1 resets the part when the register is written.</summary>
    public byte Reset;
    /// <summary>The bus-voltage range code: 0 for 16 V, 1 for 32 V.</summary>
    public byte BusRange;
    /// <summary>The shunt gain code, 0 to 3.</summary>
    public byte Gain;
    /// <summary>The bus ADC code, 0 to 15.</summary>
    public byte BusAdc;
    /// <summary>The shunt ADC code, 0 to 15.</summary>
    public byte ShuntAdc;
    /// <summary>The operating-mode code, 0 to 7.</summary>
    public byte Mode;
}

/// <summary>
/// How an INA219 driver measures. Mirrors <c>PamojaIna219Settings</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaIna219Settings
{
    /// <summary>The shunt resistance in milliohms.</summary>
    public uint ShuntMilliohms;
    /// <summary>The largest current the shunt will carry, in microamps.</summary>
    public uint MaxMicroamps;
    /// <summary>A current step in microamps per count, or 0 for the finest for the maximum.</summary>
    public uint CurrentLsbMicroamps;
    /// <summary>The range, gain, and converter settings.</summary>
    public PamojaIna219Config Config;
}

/// <summary>
/// One INA219 conversion. Mirrors <c>PamojaIna219Reading</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaIna219Reading
{
    /// <summary>The shunt-voltage register.</summary>
    public short Shunt;
    /// <summary>The bus-voltage register, flags included.</summary>
    public ushort Bus;
    /// <summary>The current register.</summary>
    public short Current;
    /// <summary>The power register.</summary>
    public ushort Power;
    /// <summary>The current step the calibration programmed, in microamps per count.</summary>
    public uint CurrentLsbMicroamps;
    /// <summary>The shunt voltage in microvolts.</summary>
    public int ShuntMicrovolts;
    /// <summary>The bus voltage in millivolts.</summary>
    public uint BusMillivolts;
    /// <summary>The current in microamps.</summary>
    public int CurrentMicroamps;
    /// <summary>The power in microwatts.</summary>
    public uint PowerMicrowatts;
    /// <summary>1 when the part's arithmetic overflowed.</summary>
    public byte MathOverflow;
}

/// <summary>
/// How an INA226 driver measures. Mirrors <c>PamojaIna226Settings</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaIna226Settings
{
    /// <summary>The shunt resistance in milliohms.</summary>
    public uint ShuntMilliohms;
    /// <summary>The largest current the shunt will carry, in microamps.</summary>
    public uint MaxMicroamps;
    /// <summary>A current step in microamps per count, or 0 for the finest for the maximum.</summary>
    public uint CurrentLsbMicroamps;
    /// <summary>The averaging and conversion times.</summary>
    public PamojaIna226Config Config;
}

/// <summary>
/// One INA226 conversion. Mirrors <c>PamojaIna226Reading</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaIna226Reading
{
    /// <summary>The shunt-voltage register.</summary>
    public short Shunt;
    /// <summary>The bus-voltage register.</summary>
    public ushort Bus;
    /// <summary>The current register.</summary>
    public short Current;
    /// <summary>The power register.</summary>
    public ushort Power;
    /// <summary>The current step the calibration programmed, in microamps per count.</summary>
    public uint CurrentLsbMicroamps;
    /// <summary>The shunt voltage in nanovolts.</summary>
    public int ShuntNanovolts;
    /// <summary>The shunt voltage in millivolts.</summary>
    public float ShuntMillivolts;
    /// <summary>The bus voltage in microvolts.</summary>
    public uint BusMicrovolts;
    /// <summary>The bus voltage in volts.</summary>
    public float BusVolts;
    /// <summary>The current in microamps.</summary>
    public int CurrentMicroamps;
    /// <summary>The current in amps.</summary>
    public float CurrentAmps;
    /// <summary>The power in microwatts.</summary>
    public uint PowerMicrowatts;
    /// <summary>The power in watts.</summary>
    public float PowerWatts;
    /// <summary>1 when the part's arithmetic overflowed.</summary>
    public byte MathOverflow;
}

/// <summary>
/// How an ADS1115 driver converts. Mirrors <c>PamojaAds1115Settings</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaAds1115Settings
{
    /// <summary>The input multiplexer code, 0 to 7.</summary>
    public byte Mux;
    /// <summary>The gain code, 0 to 7.</summary>
    public byte Pga;
    /// <summary>The data-rate code, 0 to 7.</summary>
    public byte DataRate;
}

/// <summary>
/// One ADS1115 conversion. Mirrors <c>PamojaAds1115Sample</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaAds1115Sample
{
    /// <summary>The conversion register, two's complement.</summary>
    public short Raw;
    /// <summary>The gain code the conversion ran at.</summary>
    public byte Pga;
    /// <summary>The voltage in nanovolts.</summary>
    public long Nanovolts;
    /// <summary>The voltage in volts.</summary>
    public float Volts;
    /// <summary>1 when the conversion sits at an end code, where the output clips.</summary>
    public byte Clipped;
}
