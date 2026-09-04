using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the sensing and actuation capabilities of the
/// pamoja C ABI - the sensor and actuator drivers and the windowed helper math -
/// mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The number of readings a windowed helper keeps.</summary>
    public const int WindowCapacity = 32;

    /// <summary>The BME280 temperature and pressure calibration block length.</summary>
    public const int Bme280CalibrationTempPressLen = 26;

    /// <summary>The BME280 humidity calibration block length.</summary>
    public const int Bme280CalibrationHumidityLen = 7;

    /// <summary>The length of a BME280 burst measurement read.</summary>
    public const int Bme280MeasurementLen = 8;

    /// <summary>The length of a DS18B20 scratchpad, the ninth byte its CRC.</summary>
    public const int Ds18b20ScratchpadLen = 9;

    /// <summary>Builds a BME280 calibration from its register bytes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bme280_calibration_new(
        ReadOnlySpan<byte> tempPress,
        nuint tempPressLen,
        ReadOnlySpan<byte> humidity,
        nuint humidityLen,
        out IntPtr outCalibration);

    /// <summary>Turns a BME280 burst read into a compensated reading.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bme280_compensate(
        IntPtr calibration,
        ReadOnlySpan<byte> measurement,
        nuint measurementLen,
        out PamojaBme280Measurement outMeasurement);

    /// <summary>Releases a BME280 calibration handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_bme280_calibration_free(IntPtr calibration);

    /// <summary>Parses and CRC-checks a nine-byte DS18B20 scratchpad.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ds18b20_parse_scratchpad(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out PamojaDs18b20Reading outReading);

    /// <summary>Computes the Maxim CRC-8 a 1-Wire device checks its bytes with.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_ds18b20_crc8(ReadOnlySpan<byte> data, nuint dataLen);

    /// <summary>Converts a raw DS18B20 temperature register to micro-degrees Celsius.</summary>
    [LibraryImport(Library)]
    public static partial int pamoja_ds18b20_micro_celsius(short raw);

    /// <summary>Converts a raw DS18B20 temperature register to degrees Celsius.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ds18b20_celsius(short raw);

    /// <summary>Returns the configuration byte that selects a DS18B20 resolution.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ds18b20_config_byte(byte bits, out byte outByte);

    /// <summary>Returns the resolution a DS18B20 configuration byte selects, in bits.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_ds18b20_resolution_bits(byte configByte);

    /// <summary>Returns the temperature step a DS18B20 resolution resolves.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ds18b20_step_micro_celsius(
        byte bits,
        out uint outMicroCelsius);

    /// <summary>Returns how long a DS18B20 conversion may take at a resolution.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ds18b20_max_conversion_micros(
        byte bits,
        out uint outMicros);

    /// <summary>Computes the INA219 calibration register.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina219_calibration(
        uint currentLsbMicroamps,
        uint shuntMilliohms);

    /// <summary>Returns the smallest current resolution covering an expected maximum.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina219_minimum_current_lsb_microamps(
        uint maxExpectedMicroamps);

    /// <summary>Converts a raw INA219 shunt-voltage register to microvolts.</summary>
    [LibraryImport(Library)]
    public static partial int pamoja_ina219_shunt_microvolts(short raw);

    /// <summary>Converts a raw INA219 bus-voltage register to millivolts.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina219_bus_millivolts(ushort raw);

    /// <summary>Reports whether an INA219 register says a conversion is ready.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_ina219_conversion_ready(ushort raw);

    /// <summary>Reports whether an INA219 register flags a math overflow.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_ina219_math_overflow(ushort raw);

    /// <summary>Converts a raw INA219 current register to microamps.</summary>
    [LibraryImport(Library)]
    public static partial int pamoja_ina219_current_microamps(
        short raw,
        uint currentLsbMicroamps);

    /// <summary>Converts a raw INA219 power register to microwatts.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina219_power_microwatts(
        ushort raw,
        uint currentLsbMicroamps);

    /// <summary>Assembles the 16-bit ADS1115 configuration register value.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ads1115_config_bits(PamojaAds1115Config config);

    /// <summary>Parses a 16-bit ADS1115 configuration register value.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ads1115_config_from_bits(
        ushort bits,
        out PamojaAds1115Config outConfig);

    /// <summary>Returns the full-scale range an ADS1115 gain code selects.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ads1115_full_scale_microvolts(byte pga);

    /// <summary>Returns the sample rate an ADS1115 data-rate code selects.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ads1115_samples_per_second(byte dataRate);

    /// <summary>Converts a raw ADS1115 conversion result to nanovolts.</summary>
    [LibraryImport(Library)]
    public static partial long pamoja_ads1115_to_nanovolts(byte pga, short raw);

    /// <summary>Converts a raw ADS1115 conversion result to volts.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ads1115_to_volts(byte pga, short raw);

    /// <summary>Returns the first of a PCA9685 channel's four registers.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_pca9685_channel_register(
        byte channel,
        out byte outRegister);

    /// <summary>Returns the prescale value that sets a PCA9685 update rate.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_pca9685_prescale_for_frequency(
        uint updateRateHz,
        uint oscHz);

    /// <summary>Returns the update rate a PCA9685 prescale value produces.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_pca9685_frequency_for_prescale(byte prescale, uint oscHz);

    /// <summary>Builds a PWM setting from explicit on and off counts.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPwm pamoja_pwm_from_counts(ushort on, ushort off);

    /// <summary>Builds a PWM setting with no phase delay.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPwm pamoja_pwm_duty(ushort off);

    /// <summary>Builds the PWM setting for a servo pulse width.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPwm pamoja_pwm_servo(uint pulseMicros, uint updateRateHz);

    /// <summary>The setting that holds a channel continuously high.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPwm pamoja_pwm_full_on();

    /// <summary>The setting that holds a channel continuously low.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPwm pamoja_pwm_full_off();

    /// <summary>Creates a stepper at the start of a drive pattern.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_stepper_new(PamojaStepDrive drive);

    /// <summary>Advances a stepper and returns the coil pattern to apply.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_stepper_step(IntPtr stepper, PamojaStepDirection direction);

    /// <summary>Returns the coil pattern a stepper holds, without advancing.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_stepper_coils(IntPtr stepper);

    /// <summary>Returns how many steps a stepper has taken, signed by direction.</summary>
    [LibraryImport(Library)]
    public static partial int pamoja_stepper_steps(IntPtr stepper);

    /// <summary>Returns how many steps one electrical cycle of a drive takes.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_stepper_step_count(PamojaStepDrive drive);

    /// <summary>Releases a stepper handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_stepper_free(IntPtr stepper);

    /// <summary>Returns how many steps a rotation of an angle takes.</summary>
    [LibraryImport(Library)]
    public static partial int pamoja_stepper_steps_for_degrees(
        float degrees,
        uint stepsPerRevolution);

    /// <summary>Creates an empty rolling window.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_window_new();

    /// <summary>Adds a reading to a window, dropping the oldest once full.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_window_push(IntPtr window, float reading);

    /// <summary>Returns how many readings a window holds.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_window_len(IntPtr window);

    /// <summary>Returns how many readings a window holds before dropping.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_window_capacity(IntPtr window);

    /// <summary>Reads the mean of a window's readings, if it has any.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_window_mean(IntPtr window, out float outValue);

    /// <summary>Reads the smallest reading in a window, if it has any.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_window_min(IntPtr window, out float outValue);

    /// <summary>Reads the largest reading in a window, if it has any.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_window_max(IntPtr window, out float outValue);

    /// <summary>Reads the spread across a window's readings, if it has any.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_window_range(IntPtr window, out float outValue);

    /// <summary>Reads the variance of a window's readings, if it has enough.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_window_variance(IntPtr window, out float outValue);

    /// <summary>Releases a rolling window handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_window_free(IntPtr window);

    /// <summary>Creates an empty median filter.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_median_new();

    /// <summary>Folds a reading in and returns the median of the window.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_median_update(IntPtr median, float reading);

    /// <summary>Reads the current median, if the filter has a reading.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_median_value(IntPtr median, out float outValue);

    /// <summary>Releases a median filter handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_median_free(IntPtr median);

    /// <summary>Creates an empty trend estimator.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_trend_new();

    /// <summary>Adds a reading to a trend estimator.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_trend_push(IntPtr trend, float reading);

    /// <summary>Reads the slope a trend estimator has fitted, if it has enough readings.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_trend_slope(IntPtr trend, out float outValue);

    /// <summary>Releases a trend estimator handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_trend_free(IntPtr trend);

    /// <summary>Creates an anomaly detector at a deviation threshold.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_anomaly_new(float sigmas);

    /// <summary>Folds a reading in and reports whether it stands out.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_anomaly_check(IntPtr anomaly, float reading);

    /// <summary>Releases an anomaly detector handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_anomaly_free(IntPtr anomaly);
}
