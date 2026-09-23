using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>A TMP117 temperature result.</summary>
/// <param name="Raw">The temperature register, 7.8125 millidegrees per count.</param>
/// <param name="MicroCelsius">The temperature in micro-degrees Celsius, exact in integers.</param>
/// <param name="Celsius">The temperature in degrees Celsius.</param>
public readonly record struct Tmp117Reading(short Raw, int MicroCelsius, float Celsius)
{
    /// <summary>Converts the flat struct the C ABI returns.</summary>
    /// <param name="native">The interop representation.</param>
    /// <returns>The reading.</returns>
    internal static Tmp117Reading From(PamojaTmp117Reading native) =>
        new(native.Raw, native.MicroCelsius, native.Celsius);
}

/// <summary>The TMP117's alert flags: whether a result since they were last read crossed a limit.</summary>
/// <param name="High">A result was above the high limit.</param>
/// <param name="Low">A result was below the low limit.</param>
public readonly record struct Tmp117Alerts(bool High, bool Low);

/// <summary>An OPT3001 illuminance result.</summary>
/// <param name="Raw">The result register: a four-bit exponent over a twelve-bit mantissa.</param>
/// <param name="MilliLux">The illuminance in millilux, exact in integers.</param>
/// <param name="Lux">The illuminance in lux.</param>
public readonly record struct Opt3001Reading(ushort Raw, uint MilliLux, float Lux);

/// <summary>An INA219 configuration register, field by field.</summary>
/// <param name="BusRange">The bus-voltage range.</param>
/// <param name="Gain">The shunt gain, which sets the shunt-voltage range.</param>
/// <param name="BusAdc">The bus converter's resolution or averaging.</param>
/// <param name="ShuntAdc">The shunt converter's resolution or averaging.</param>
/// <param name="Mode">The operating mode.</param>
/// <param name="Reset">Whether writing the register resets the part.</param>
public readonly record struct Ina219Config(
    Ina219.BusRange BusRange,
    Ina219.Gain Gain,
    Ina219.Adc BusAdc,
    Ina219.Adc ShuntAdc,
    Ina219.Mode Mode,
    bool Reset = false)
{
    /// <summary>The power-on configuration: the 32 V range, gain 1/8, 12-bit conversions, shunt and bus continuous.</summary>
    public static Ina219Config PowerOn { get; } = new(
        Ina219.BusRange.V32,
        Ina219.Gain.Div8,
        Ina219.Adc.Bits12,
        Ina219.Adc.Bits12,
        Ina219.Mode.ShuntAndBusContinuous);

    /// <summary>Converts to the flat struct the C ABI takes.</summary>
    /// <returns>The interop representation.</returns>
    internal PamojaIna219Config ToNative() => new()
    {
        Reset = Reset ? (byte)1 : (byte)0,
        BusRange = (byte)BusRange,
        Gain = (byte)Gain,
        BusAdc = (byte)BusAdc,
        ShuntAdc = (byte)ShuntAdc,
        Mode = (byte)Mode,
    };

    /// <summary>Converts the flat struct the C ABI returns.</summary>
    /// <param name="native">The interop representation.</param>
    /// <returns>The configuration.</returns>
    internal static Ina219Config From(PamojaIna219Config native) => new(
        (Ina219.BusRange)native.BusRange,
        (Ina219.Gain)native.Gain,
        (Ina219.Adc)native.BusAdc,
        (Ina219.Adc)native.ShuntAdc,
        (Ina219.Mode)native.Mode,
        native.Reset != 0);
}

/// <summary>One INA219 conversion: the four result registers as read, and what they mean.</summary>
/// <param name="Shunt">The shunt-voltage register.</param>
/// <param name="Bus">The bus-voltage register, flags included.</param>
/// <param name="Current">The current register.</param>
/// <param name="Power">The power register.</param>
/// <param name="CurrentLsbMicroamps">The current step the calibration programmed, in microamps per count.</param>
/// <param name="ShuntMicrovolts">The shunt voltage in microvolts.</param>
/// <param name="BusMillivolts">The bus voltage in millivolts.</param>
/// <param name="CurrentMicroamps">The current in microamps; negative flows the other way through the shunt.</param>
/// <param name="PowerMicrowatts">The power in microwatts.</param>
/// <param name="MathOverflow">Whether the part's arithmetic overflowed, leaving current and power meaningless.</param>
public readonly record struct Ina219Reading(
    short Shunt,
    ushort Bus,
    short Current,
    ushort Power,
    uint CurrentLsbMicroamps,
    int ShuntMicrovolts,
    uint BusMillivolts,
    int CurrentMicroamps,
    uint PowerMicrowatts,
    bool MathOverflow)
{
    /// <summary>Converts the flat struct the C ABI returns.</summary>
    /// <param name="native">The interop representation.</param>
    /// <returns>The reading.</returns>
    internal static Ina219Reading From(PamojaIna219Reading native) => new(
        native.Shunt,
        native.Bus,
        native.Current,
        native.Power,
        native.CurrentLsbMicroamps,
        native.ShuntMicrovolts,
        native.BusMillivolts,
        native.CurrentMicroamps,
        native.PowerMicrowatts,
        native.MathOverflow != 0);
}

/// <summary>One INA226 conversion: the four result registers as read, and what they mean.</summary>
/// <param name="Shunt">The shunt-voltage register.</param>
/// <param name="Bus">The bus-voltage register.</param>
/// <param name="Current">The current register.</param>
/// <param name="Power">The power register.</param>
/// <param name="CurrentLsbMicroamps">The current step the calibration programmed, in microamps per count.</param>
/// <param name="ShuntNanovolts">The shunt voltage in nanovolts.</param>
/// <param name="ShuntMillivolts">The shunt voltage in millivolts.</param>
/// <param name="BusMicrovolts">The bus voltage in microvolts.</param>
/// <param name="BusVolts">The bus voltage in volts.</param>
/// <param name="CurrentMicroamps">The current in microamps; negative flows the other way through the shunt.</param>
/// <param name="CurrentAmps">The current in amps.</param>
/// <param name="PowerMicrowatts">The power in microwatts.</param>
/// <param name="PowerWatts">The power in watts.</param>
/// <param name="MathOverflow">Whether the part's arithmetic overflowed, leaving current and power meaningless.</param>
public readonly record struct Ina226Reading(
    short Shunt,
    ushort Bus,
    short Current,
    ushort Power,
    uint CurrentLsbMicroamps,
    int ShuntNanovolts,
    float ShuntMillivolts,
    uint BusMicrovolts,
    float BusVolts,
    int CurrentMicroamps,
    float CurrentAmps,
    uint PowerMicrowatts,
    float PowerWatts,
    bool MathOverflow)
{
    /// <summary>Converts the flat struct the C ABI returns.</summary>
    /// <param name="native">The interop representation.</param>
    /// <returns>The reading.</returns>
    internal static Ina226Reading From(PamojaIna226Reading native) => new(
        native.Shunt,
        native.Bus,
        native.Current,
        native.Power,
        native.CurrentLsbMicroamps,
        native.ShuntNanovolts,
        native.ShuntMillivolts,
        native.BusMicrovolts,
        native.BusVolts,
        native.CurrentMicroamps,
        native.CurrentAmps,
        native.PowerMicrowatts,
        native.PowerWatts,
        native.MathOverflow != 0);
}

/// <summary>One ADS1115 conversion.</summary>
/// <param name="Raw">The conversion register, two's complement.</param>
/// <param name="Gain">The full-scale range the conversion ran at.</param>
/// <param name="Nanovolts">The voltage in nanovolts, exact in integers.</param>
/// <param name="Volts">The voltage in volts.</param>
public readonly record struct Ads1115Sample(short Raw, Ads1115.Pga Gain, long Nanovolts, float Volts)
{
    /// <summary>Converts the flat struct the C ABI returns.</summary>
    /// <param name="native">The interop representation.</param>
    /// <returns>The sample.</returns>
    internal static Ads1115Sample From(PamojaAds1115Sample native) =>
        new(native.Raw, (Ads1115.Pga)native.Pga, native.Nanovolts, native.Volts);
}
