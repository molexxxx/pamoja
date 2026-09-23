using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A BME280 ctrl_meas register, field by field. Mirrors <c>PamojaBme280CtrlMeas</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBme280CtrlMeas
{
    /// <summary>The temperature oversampling code, 0 to 5, where 0 skips the measurement.</summary>
    public byte Temperature;
    /// <summary>The pressure oversampling code, 0 to 5, where 0 skips the measurement.</summary>
    public byte Pressure;
    /// <summary>The power mode code: 0 sleep, 1 forced, 3 normal.</summary>
    public byte Mode;
}

/// <summary>
/// A BME280 config register, field by field. Mirrors <c>PamojaBme280Config</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBme280Config
{
    /// <summary>The normal-mode standby code, 0 to 7.</summary>
    public byte Standby;
    /// <summary>The IIR filter code, 0 to 4, where 0 is off.</summary>
    public byte Filter;
    /// <summary>1 enables the 3-wire SPI interface.</summary>
    public byte Spi3Wire;
}

/// <summary>
/// How a BME280 driver measures. Mirrors <c>PamojaBme280Settings</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBme280Settings
{
    /// <summary>The temperature oversampling code, 0 to 5, where 0 skips the measurement.</summary>
    public byte Temperature;
    /// <summary>The pressure oversampling code, 0 to 5, where 0 skips the measurement.</summary>
    public byte Pressure;
    /// <summary>The humidity oversampling code, 0 to 5, where 0 skips the measurement.</summary>
    public byte Humidity;
    /// <summary>The IIR filter code, 0 to 4, where 0 is off.</summary>
    public byte Filter;
}
