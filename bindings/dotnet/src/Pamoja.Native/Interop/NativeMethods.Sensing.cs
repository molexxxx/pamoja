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

    /// <summary>The length of a SHT3x measurement frame.</summary>
    public const int Sht3xMeasurementLen = 6;

    /// <summary>The length of a SHT3x word and its checksum.</summary>
    public const int Sht3xWordLen = 3;

    /// <summary>The length of a SCD4x measurement frame.</summary>
    public const int Scd4xMeasurementLen = 9;

    /// <summary>The length of a SCD4x word and its checksum.</summary>
    public const int Scd4xWordLen = 3;

    /// <summary>The length of a SCD4x command frame.</summary>
    public const int Scd4xCommandLen = 2;

    /// <summary>The length of a SCD4x command-and-value frame.</summary>
    public const int Scd4xWriteLen = 5;

    /// <summary>The length of a BMP280 calibration block.</summary>
    public const int Bmp280CalibrationLen = 24;

    /// <summary>The length of a BMP280 burst data read.</summary>
    public const int Bmp280DataLen = 6;

    /// <summary>The length of a TMP117 register.</summary>
    public const int Tmp117RegisterLen = 2;

    /// <summary>The length of a HDC1080 measurement frame.</summary>
    public const int Hdc1080MeasurementLen = 4;

    /// <summary>The length of a OPT3001 register.</summary>
    public const int Opt3001RegisterLen = 2;

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

    /// <summary>Builds the nine bytes a DS18B20 in the given state puts on the bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ds18b20_build_scratchpad(
        float celsius,
        byte bits,
        sbyte alarmHigh,
        sbyte alarmLow,
        Span<byte> outBytes);

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

    /// <summary>Builds the INA219 shunt-voltage register for a shunt voltage.</summary>
    [LibraryImport(Library)]
    public static partial short pamoja_ina219_shunt_register(int microvolts);

    /// <summary>Builds the INA219 bus-voltage register for a bus voltage.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina219_bus_register(uint millivolts);

    /// <summary>Builds the INA219 current register for a current.</summary>
    [LibraryImport(Library)]
    public static partial short pamoja_ina219_current_register(
        int microamps,
        uint currentLsbMicroamps);

    /// <summary>Builds the INA219 power register for a power.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina219_power_register(
        uint microwatts,
        uint currentLsbMicroamps);

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

    /// <summary>
    /// Builds a BMP280 calibration from the 24 bytes read out of its registers. # Returns
    /// [PamojaStatus::Ok] on success, with *out_calibration set to a new handle the caller
    /// must release with [pamoja_bmp280_calibration_free], or
    /// [PamojaStatus::InvalidArgument] if the buffer is not 24 bytes. # Safety bytes must
    /// point to at least bytes_len readable bytes, and out_calibration must point to a
    /// writable *mut PamojaBmp280Calibration.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_calibration_new(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out IntPtr outCalibration);

    /// <summary>
    /// Turns a BMP280 burst read into a compensated reading. # Returns [PamojaStatus::Ok]
    /// on success, with *out_reading filled in, or [PamojaStatus::InvalidArgument] if the
    /// calibration is null or the measurement is not six bytes. # Safety calibration must
    /// be a live handle from [pamoja_bmp280_calibration_new], measurement must point to at
    /// least measurement_len readable bytes, and out_reading must point to a writable
    /// PamojaBmp280Reading.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_compensate(
        IntPtr calibration,
        ReadOnlySpan<byte> measurement,
        nuint measurementLen,
        out PamojaBmp280Reading outReading);

    /// <summary>
    /// Rebuilds the 24 calibration bytes a device holding these coefficients returns. #
    /// Returns [PamojaStatus::Ok] on success, with the 24 bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if either pointer is null. # Safety calibration must
    /// be a live handle from [pamoja_bmp280_calibration_new], and out_bytes must point to
    /// at least 24 writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_calibration_to_bytes(
        IntPtr calibration,
        Span<byte> outBytes);

    /// <summary>
    /// Reads out the trimming coefficients a BMP280 calibration carries. # Returns
    /// [PamojaStatus::Ok] on success, with *out_coefficients filled in, or
    /// [PamojaStatus::InvalidArgument] if either pointer is null. # Safety calibration must
    /// be a live handle from [pamoja_bmp280_calibration_new], and out_coefficients must
    /// point to a writable PamojaBmp280Coefficients.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_calibration_coefficients(
        IntPtr calibration,
        out PamojaBmp280Coefficients outCoefficients);

    /// <summary>
    /// Releases a BMP280 calibration handle. Passing null is a no-op. # Safety calibration
    /// must be a handle from [pamoja_bmp280_calibration_new] that has not already been
    /// freed, or null. After this call it must not be used again.
    /// </summary>
    [LibraryImport(Library)]
    public static partial void pamoja_bmp280_calibration_free(IntPtr calibration);

    /// <summary>
    /// Unpacks the six data bytes a BMP280 burst read returns. # Returns [PamojaStatus::Ok]
    /// on success, with *out_measurement filled in, or [PamojaStatus::InvalidArgument] if
    /// the buffer is not six bytes. # Safety data must point to at least data_len readable
    /// bytes, and out_measurement must point to a writable PamojaBmp280Measurement.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_parse_measurement(
        ReadOnlySpan<byte> data,
        nuint dataLen,
        out PamojaBmp280Measurement outMeasurement);

    /// <summary>
    /// Builds the six data bytes a BMP280 holding these codes would return. # Returns
    /// [PamojaStatus::Ok] on success, with the six bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least six writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_measurement_bytes(
        uint pressure,
        uint temperature,
        Span<byte> outBytes);

    /// <summary>
    /// Reports whether a BMP280 status byte says a conversion is running. # Returns true
    /// while the part is measuring.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_bmp280_measuring(byte status);

    /// <summary>
    /// Reports whether a BMP280 status byte says the calibration image is loading. #
    /// Returns true while the coefficients are being copied out of non-volatile memory.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_bmp280_image_updating(byte status);

    /// <summary>Assembles a BMP280 ctrl_meas register value. # Returns The register value to write.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_bmp280_ctrl_meas_bits(PamojaBmp280CtrlMeas config);

    /// <summary>
    /// Parses a BMP280 ctrl_meas register value. # Returns [PamojaStatus::Ok], with
    /// *out_config filled in. Every register value decodes, so this fails only on a null
    /// pointer. # Safety out_config must point to a writable PamojaBmp280CtrlMeas.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_ctrl_meas_from_bits(
        byte bits,
        out PamojaBmp280CtrlMeas outConfig);

    /// <summary>Assembles a BMP280 config register value. # Returns The register value to write.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_bmp280_config_bits(PamojaBmp280Config config);

    /// <summary>
    /// Parses a BMP280 config register value. # Returns [PamojaStatus::Ok], with
    /// *out_config filled in. Every register value decodes, so this fails only on a null
    /// pointer. # Safety out_config must point to a writable PamojaBmp280Config.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_config_from_bits(
        byte bits,
        out PamojaBmp280Config outConfig);

    /// <summary>
    /// Returns how many samples a BMP280 oversampling code averages. # Returns The
    /// oversampling factor, 1 to 16.
    /// </summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_bmp280_oversampling_factor(byte code);

    /// <summary>
    /// Returns the normal-mode standby period a BMP280 code selects. # Returns The period
    /// in microseconds.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_bmp280_standby_micros(byte code);

    /// <summary>
    /// Computes the CRC-8 an SHT3x appends to every data word. # Returns The checksum over
    /// data, or 0 if the pointer is null with a non-zero length. # Safety data must point
    /// to at least data_len readable bytes, or be null when data_len is 0.
    /// </summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sht3x_crc(
        ReadOnlySpan<byte> data,
        nuint dataLen);

    /// <summary>
    /// Reads a CRC-checked three-byte SHT3x word frame. # Returns [PamojaStatus::Ok] on
    /// success, with *out_word set, or [PamojaStatus::Codec] if the CRC does not match. #
    /// Safety frame must point to at least frame_len readable bytes, and out_word must
    /// point to a writable uint16_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_word(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        out ushort outWord);

    /// <summary>
    /// Builds the three bytes an SHT3x sends for a word: the word then its CRC. # Returns
    /// [PamojaStatus::Ok] on success, with the three bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least three writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_word_bytes(
        ushort value,
        Span<byte> outBytes);

    /// <summary>
    /// Parses and CRC-checks a six-byte SHT3x measurement frame. # Returns
    /// [PamojaStatus::Ok] on success, with *out_measurement filled in, or
    /// [PamojaStatus::Codec] if either word fails its checksum, which means the read was
    /// corrupted on the bus and should be repeated. # Safety frame must point to at least
    /// frame_len readable bytes, and out_measurement must point to a writable
    /// PamojaSht3xMeasurement.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_parse_measurement(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        out PamojaSht3xMeasurement outMeasurement);

    /// <summary>
    /// Builds the six bytes an SHT3x sends for a pair of raw words. # Returns
    /// [PamojaStatus::Ok] on success, with the six bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least six writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_measurement_bytes(
        ushort temperatureRaw,
        ushort humidityRaw,
        Span<byte> outBytes);

    /// <summary>
    /// Converts a raw SHT3x temperature word to milli-degrees Celsius. # Returns The
    /// temperature, exact in integer arithmetic.
    /// </summary>
    [LibraryImport(Library)]
    public static partial int pamoja_sht3x_milli_celsius(ushort raw);

    /// <summary>Converts a raw SHT3x temperature word to degrees Celsius. # Returns The temperature.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_sht3x_celsius(ushort raw);

    /// <summary>
    /// Converts a raw SHT3x temperature word to milli-degrees Fahrenheit. # Returns The
    /// temperature, exact in integer arithmetic.
    /// </summary>
    [LibraryImport(Library)]
    public static partial int pamoja_sht3x_milli_fahrenheit(ushort raw);

    /// <summary>
    /// Converts a raw SHT3x temperature word to degrees Fahrenheit. # Returns The
    /// temperature.
    /// </summary>
    [LibraryImport(Library)]
    public static partial float pamoja_sht3x_fahrenheit(ushort raw);

    /// <summary>
    /// Converts a raw SHT3x humidity word to milli-percent. # Returns The relative
    /// humidity, exact in integer arithmetic.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_sht3x_milli_percent(ushort raw);

    /// <summary>
    /// Converts a raw SHT3x humidity word to a relative humidity percentage. # Returns The
    /// relative humidity.
    /// </summary>
    [LibraryImport(Library)]
    public static partial float pamoja_sht3x_relative_humidity(ushort raw);

    /// <summary>
    /// Builds the SHT3x temperature word that decodes to a temperature. # Returns The raw
    /// word, saturating at the ends of the part's range.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_sht3x_temperature_raw_from_milli_celsius(int milliCelsius);

    /// <summary>
    /// Builds the SHT3x temperature word that decodes to a temperature in Celsius. #
    /// Returns The raw word, saturating at the ends of the part's range.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_sht3x_temperature_raw_from_celsius(float celsius);

    /// <summary>
    /// Builds the SHT3x temperature word that decodes to a temperature in Fahrenheit. #
    /// Returns The raw word, saturating at the ends of the part's range.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_sht3x_temperature_raw_from_milli_fahrenheit(int milliFahrenheit);

    /// <summary>
    /// Builds the SHT3x humidity word that decodes to a relative humidity. # Returns The
    /// raw word, saturating at full scale.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_sht3x_humidity_raw_from_milli_percent(uint milliPercent);

    /// <summary>
    /// Builds the SHT3x humidity word that decodes to a relative humidity percentage. #
    /// Returns The raw word, saturating at full scale.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_sht3x_humidity_raw_from_relative_humidity(float percent);

    /// <summary>
    /// Parses and CRC-checks a three-byte SHT3x status frame. # Returns [PamojaStatus::Ok]
    /// on success, with *out_status filled in, or [PamojaStatus::Codec] if the CRC does not
    /// match. # Safety frame must point to at least frame_len readable bytes, and
    /// out_status must point to a writable PamojaSht3xStatus.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_parse_status(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        out PamojaSht3xStatus outStatus);

    /// <summary>
    /// Splits an SHT3x status word into its flags. # Returns [PamojaStatus::Ok], with
    /// *out_status filled in. Every word decodes, so this fails only on a null pointer. #
    /// Safety out_status must point to a writable PamojaSht3xStatus.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_status_from_bits(
        ushort bits,
        out PamojaSht3xStatus outStatus);

    /// <summary>
    /// Builds the three bytes an SHT3x sends for a status word, CRC last. # Returns
    /// [PamojaStatus::Ok] on success, with the three bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least three writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_status_bytes(
        ushort bits,
        Span<byte> outBytes);

    /// <summary>
    /// Returns the SHT3x single-shot command for a repeatability and clock mode. # Returns
    /// [PamojaStatus::Ok] on success, with *out_command set, or
    /// [PamojaStatus::InvalidArgument] if repeatability is not 0, 1, or 2. # Safety
    /// out_command must point to a writable uint16_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_single_shot(
        byte repeatability,
        [MarshalAs(UnmanagedType.U1)] bool clockStretching,
        out ushort outCommand);

    /// <summary>
    /// Returns the SHT3x periodic-mode command for a repeatability and rate. # Returns
    /// [PamojaStatus::Ok] on success, with *out_command set, or
    /// [PamojaStatus::InvalidArgument] if either code is outside its range. # Safety
    /// out_command must point to a writable uint16_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_periodic(
        byte repeatability,
        byte rate,
        out ushort outCommand);

    /// <summary>
    /// Returns how long an SHT3x measurement may take at a repeatability. # Returns
    /// [PamojaStatus::Ok] on success, with *out_micros set to the datasheet's worst case,
    /// or [PamojaStatus::InvalidArgument] if the code is out of range. # Safety out_micros
    /// must point to a writable uint32_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_max_measurement_micros(
        byte repeatability,
        out uint outMicros);

    /// <summary>
    /// Returns how long an SHT3x measurement typically takes at a repeatability. # Returns
    /// [PamojaStatus::Ok] on success, with *out_micros set, or
    /// [PamojaStatus::InvalidArgument] if the code is out of range. # Safety out_micros
    /// must point to a writable uint32_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_typical_measurement_micros(
        byte repeatability,
        out uint outMicros);

    /// <summary>
    /// Returns the gap between SHT3x periodic measurements at a rate. # Returns
    /// [PamojaStatus::Ok] on success, with *out_micros set, or
    /// [PamojaStatus::InvalidArgument] if the code is out of range. # Safety out_micros
    /// must point to a writable uint32_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_interval_micros(
        byte rate,
        out uint outMicros);

    /// <summary>
    /// Computes the CRC-8 an SCD4x appends to every data word. # Returns The checksum over
    /// data, or 0 if the pointer is null with a non-zero length. # Safety data must point
    /// to at least data_len readable bytes, or be null when data_len is 0.
    /// </summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_scd4x_crc(
        ReadOnlySpan<byte> data,
        nuint dataLen);

    /// <summary>
    /// Reads a CRC-checked three-byte SCD4x word frame. # Returns [PamojaStatus::Ok] on
    /// success, with *out_word set, or [PamojaStatus::Codec] if the CRC does not match. #
    /// Safety frame must point to at least frame_len readable bytes, and out_word must
    /// point to a writable uint16_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_word(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        out ushort outWord);

    /// <summary>
    /// Builds the three bytes an SCD4x sends for a word: the word then its CRC. # Returns
    /// [PamojaStatus::Ok] on success, with the three bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least three writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_word_frame(
        ushort value,
        Span<byte> outBytes);

    /// <summary>
    /// Builds the two bytes that address an SCD4x command, most significant first. #
    /// Returns [PamojaStatus::Ok] on success, with the two bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least two writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_command_frame(
        ushort command,
        Span<byte> outBytes);

    /// <summary>
    /// Builds the five bytes that write a word to an SCD4x: command, word, CRC. # Returns
    /// [PamojaStatus::Ok] on success, with the five bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least five writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_write_frame(
        ushort command,
        ushort value,
        Span<byte> outBytes);

    /// <summary>
    /// Returns how long an SCD4x command may take before its result can be read. # Returns
    /// true when the command has a documented execution time, with *out_millis set to it;
    /// false when it completes as soon as it is acknowledged. # Safety out_millis must
    /// point to a writable uint16_t.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_scd4x_max_duration_ms(
        ushort command,
        out ushort outMillis);

    /// <summary>
    /// Reports whether an SCD4x accepts a command while it is measuring. # Returns true
    /// when the command may be sent without stopping periodic measurements.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_scd4x_allowed_during_measurement(ushort command);

    /// <summary>
    /// Parses and CRC-checks a nine-byte SCD4x measurement frame. # Returns
    /// [PamojaStatus::Ok] on success, with *out_measurement filled in, or
    /// [PamojaStatus::Codec] if any word fails its checksum, which means the read was
    /// corrupted on the bus and should be repeated. # Safety frame must point to at least
    /// frame_len readable bytes, and out_measurement must point to a writable
    /// PamojaScd4xMeasurement.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_parse_measurement(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        out PamojaScd4xMeasurement outMeasurement);

    /// <summary>
    /// Builds the SCD4x measurement a sensor reporting these physical values would send. #
    /// Returns [PamojaStatus::Ok], with *out_measurement filled in, or
    /// [PamojaStatus::InvalidArgument] if the pointer is null. # Safety out_measurement
    /// must point to a writable PamojaScd4xMeasurement.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_measurement_from_physical(
        ushort co2Ppm,
        int milliCelsius,
        uint humidityMilliPercent,
        out PamojaScd4xMeasurement outMeasurement);

    /// <summary>
    /// Builds the nine bytes an SCD4x sends for a set of raw words, each CRC included. #
    /// Returns [PamojaStatus::Ok] on success, with the nine bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least nine writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_measurement_bytes(
        ushort co2Ppm,
        ushort temperatureRaw,
        ushort humidityRaw,
        Span<byte> outBytes);

    /// <summary>
    /// Converts a raw SCD4x temperature word to milli-degrees Celsius. # Returns The
    /// temperature, exact in integer arithmetic.
    /// </summary>
    [LibraryImport(Library)]
    public static partial int pamoja_scd4x_milli_celsius(ushort raw);

    /// <summary>Converts a raw SCD4x temperature word to degrees Celsius. # Returns The temperature.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_scd4x_celsius(ushort raw);

    /// <summary>
    /// Builds the SCD4x temperature word that decodes to a temperature. # Returns The raw
    /// word, saturating at the ends of the part's range.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_scd4x_temperature_raw(int milliCelsius);

    /// <summary>
    /// Converts a raw SCD4x humidity word to milli-percent. # Returns The relative
    /// humidity, exact in integer arithmetic.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_scd4x_humidity_milli_percent(ushort raw);

    /// <summary>
    /// Converts a raw SCD4x humidity word to a relative humidity percentage. # Returns The
    /// relative humidity.
    /// </summary>
    [LibraryImport(Library)]
    public static partial float pamoja_scd4x_relative_humidity_percent(ushort raw);

    /// <summary>
    /// Builds the SCD4x humidity word that decodes to a relative humidity. # Returns The
    /// raw word, saturating at full scale.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_scd4x_humidity_raw(uint milliPercent);

    /// <summary>
    /// Reports whether an SCD4x data-ready word says a fresh result is waiting. # Returns
    /// true when any of the low eleven bits is set.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_scd4x_data_ready(ushort word);

    /// <summary>
    /// Builds the SCD4x temperature-offset word for an offset. # Returns The word to write,
    /// which scales by 2^16 rather than by the 2^16 - 1 the measurement words use.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_scd4x_temperature_offset_word(uint milliCelsius);

    /// <summary>
    /// Reads an SCD4x temperature-offset word back as an offset. # Returns The offset in
    /// milli-degrees Celsius.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_scd4x_temperature_offset_milli_celsius(ushort word);

    /// <summary>
    /// Builds the SCD4x ambient-pressure word for a pressure. # Returns The word to write,
    /// at 100 pascals per count.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_scd4x_ambient_pressure_word(uint pascals);

    /// <summary>
    /// Reads an SCD4x ambient-pressure word back as a pressure. # Returns The pressure in
    /// pascals.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_scd4x_ambient_pressure_pascals(ushort word);

    /// <summary>
    /// Reads the correction a forced recalibration applied. # Returns true when the
    /// recalibration took, with *out_ppm set to the correction in parts per million; false
    /// when the part reported that it failed. # Safety out_ppm must point to a writable
    /// int32_t.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_scd4x_forced_recalibration_correction_ppm(
        ushort word,
        out int outPpm);

    /// <summary>
    /// Builds the word an SCD4x returns for a forced-recalibration outcome. # Returns The
    /// word, which is the failure sentinel when succeeded is false.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_scd4x_forced_recalibration_word(
        [MarshalAs(UnmanagedType.U1)] bool succeeded,
        int correctionPpm);

    /// <summary>
    /// Reports whether an SCD4x word says automatic self-calibration is on. # Returns true
    /// when the part recalibrates itself against clean air.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_scd4x_automatic_self_calibration_enabled(ushort word);

    /// <summary>
    /// Builds the SCD4x word that turns automatic self-calibration on or off. # Returns The
    /// word to write.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_scd4x_automatic_self_calibration_word(
        [MarshalAs(UnmanagedType.U1)] bool enabled);

    /// <summary>
    /// Reports whether an SCD4x self-test word says the part is healthy. # Returns true
    /// when the self test found no malfunction.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_scd4x_self_test_passed(ushort word);

    /// <summary>
    /// Reads the 48-bit serial number out of a nine-byte SCD4x frame. # Returns
    /// [PamojaStatus::Ok] on success, with *out_serial set, or [PamojaStatus::Codec] if any
    /// word fails its checksum. # Safety frame must point to at least frame_len readable
    /// bytes, and out_serial must point to a writable uint64_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_serial_number(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        out ulong outSerial);

    /// <summary>
    /// Builds the nine bytes an SCD4x sends for a serial number, each CRC included. #
    /// Returns [PamojaStatus::Ok] on success, with the nine bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least nine writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_serial_number_frame(
        ulong serial,
        Span<byte> outBytes);

    /// <summary>
    /// Converts a raw TMP117 temperature register to nano-degrees Celsius. # Returns The
    /// temperature, exact in integer arithmetic at the part's 7.8125 m°C step.
    /// </summary>
    [LibraryImport(Library)]
    public static partial long pamoja_tmp117_nano_celsius(short raw);

    /// <summary>
    /// Converts a raw TMP117 temperature register to micro-degrees Celsius. # Returns The
    /// temperature, truncated toward zero at the last digit.
    /// </summary>
    [LibraryImport(Library)]
    public static partial int pamoja_tmp117_micro_celsius(short raw);

    /// <summary>
    /// Converts a raw TMP117 temperature register to degrees Celsius. # Returns The
    /// temperature.
    /// </summary>
    [LibraryImport(Library)]
    public static partial float pamoja_tmp117_celsius(short raw);

    /// <summary>
    /// Builds the TMP117 temperature register that decodes to a temperature. # Returns The
    /// nearest register value, saturating at the ends of the part's range.
    /// </summary>
    [LibraryImport(Library)]
    public static partial short pamoja_tmp117_raw_from_micro_celsius(int microCelsius);

    /// <summary>
    /// Builds the TMP117 temperature register that decodes to a temperature in Celsius. #
    /// Returns The nearest register value, saturating at the ends of the part's range.
    /// </summary>
    [LibraryImport(Library)]
    public static partial short pamoja_tmp117_raw_from_celsius(float celsius);

    /// <summary>
    /// Builds the two bytes a TMP117 sends for a temperature register. # Returns
    /// [PamojaStatus::Ok] on success, with the two bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least two writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_tmp117_temperature_bytes(
        short raw,
        Span<byte> outBytes);

    /// <summary>
    /// Reads the two bytes a TMP117 sends for a temperature register. # Returns
    /// [PamojaStatus::Ok] on success, with *out_raw set, or [PamojaStatus::InvalidArgument]
    /// if the buffer is not two bytes. # Safety bytes must point to at least bytes_len
    /// readable bytes, and out_raw must point to a writable int16_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_tmp117_temperature_from_bytes(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out short outRaw);

    /// <summary>
    /// Reads the device identifier out of a TMP117 device-ID register. # Returns The low
    /// twelve bits, which are 0x117 for a TMP117.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_tmp117_device_id(ushort raw);

    /// <summary>
    /// Reads the die revision out of a TMP117 device-ID register. # Returns The high four
    /// bits.
    /// </summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_tmp117_revision(ushort raw);

    /// <summary>
    /// Reports whether a TMP117 configuration register flags a high alert. # Returns true
    /// when a result went above the high limit.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_tmp117_high_alert(ushort config);

    /// <summary>
    /// Reports whether a TMP117 configuration register flags a low alert. # Returns true
    /// when a result went below the low limit.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_tmp117_low_alert(ushort config);

    /// <summary>
    /// Reports whether a TMP117 configuration register says a result is ready. # Returns
    /// true when a conversion completed since the register was last read.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_tmp117_data_ready(ushort config);

    /// <summary>
    /// Reports whether a TMP117 configuration register says an EEPROM write is running. #
    /// Returns true while the write is in progress.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_tmp117_eeprom_busy(ushort config);

    /// <summary>
    /// Reports whether a TMP117 EEPROM unlock register says a write is running. # Returns
    /// true while the write is in progress.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_tmp117_eeprom_unlock_busy(ushort unlock);

    /// <summary>
    /// Assembles the 16-bit TMP117 configuration register value. # Returns The register
    /// value to write.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_tmp117_config_bits(PamojaTmp117Config config);

    /// <summary>
    /// Parses a 16-bit TMP117 configuration register value. # Returns [PamojaStatus::Ok],
    /// with *out_config filled in. Every register value decodes, so this fails only on a
    /// null pointer. # Safety out_config must point to a writable PamojaTmp117Config.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_tmp117_config_from_bits(
        ushort bits,
        out PamojaTmp117Config outConfig);

    /// <summary>
    /// Returns how many conversions a TMP117 averaging code folds into one result. #
    /// Returns The conversion count: 1, 8, 32, or 64.
    /// </summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_tmp117_averaging_conversions(byte code);

    /// <summary>
    /// Returns how long a TMP117 averaging code takes to convert. # Returns The conversion
    /// time in microseconds.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_tmp117_averaging_micros(byte code);

    /// <summary>
    /// Returns the nominal cycle a TMP117 conversion-cycle code selects. # Returns The
    /// cycle in microseconds, before the averaging setting extends it.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_tmp117_cycle_nominal_micros(byte code);

    /// <summary>
    /// Returns how often a TMP117 updates its result for a cycle and averaging code. #
    /// Returns The longer of the nominal cycle and the time the averaging takes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_tmp117_cycle_micros(
        byte cycle,
        byte averaging);

    /// <summary>
    /// Converts a raw HDC1080 temperature register to milli-degrees Celsius. # Returns The
    /// temperature, exact in integer arithmetic.
    /// </summary>
    [LibraryImport(Library)]
    public static partial int pamoja_hdc1080_milli_celsius(ushort raw);

    /// <summary>
    /// Converts a raw HDC1080 temperature register to degrees Celsius. # Returns The
    /// temperature.
    /// </summary>
    [LibraryImport(Library)]
    public static partial float pamoja_hdc1080_celsius(ushort raw);

    /// <summary>
    /// Converts a raw HDC1080 humidity register to milli-percent. # Returns The relative
    /// humidity, exact in integer arithmetic.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_hdc1080_milli_percent(ushort raw);

    /// <summary>
    /// Converts a raw HDC1080 humidity register to a relative humidity percentage. #
    /// Returns The relative humidity.
    /// </summary>
    [LibraryImport(Library)]
    public static partial float pamoja_hdc1080_relative_humidity(ushort raw);

    /// <summary>
    /// Builds the HDC1080 temperature register that decodes to a temperature. # Returns The
    /// 14-bit code in bits 15:2, clamped to the part's range.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_hdc1080_temperature_register(int milliCelsius);

    /// <summary>
    /// Builds the HDC1080 humidity register that decodes to a relative humidity. # Returns
    /// The 14-bit code in bits 15:2, clamped to full scale.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_hdc1080_humidity_register(uint milliPercent);

    /// <summary>
    /// Joins the three HDC1080 serial-ID registers into the 40-bit serial number. # Returns
    /// The serial number.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_hdc1080_serial_id(
        ushort high,
        ushort mid,
        ushort low);

    /// <summary>
    /// Splits a serial number back into the three HDC1080 serial-ID registers. # Returns
    /// [PamojaStatus::Ok] on success, with the three registers written to out_registers
    /// high word first, or [PamojaStatus::InvalidArgument] if the pointer is null. # Safety
    /// out_registers must point to at least three writable uint16_t values.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_serial_id_registers(
        ulong serial,
        out ushort outRegisters);

    /// <summary>
    /// Parses the four bytes an HDC1080 sequential read returns. # Returns
    /// [PamojaStatus::Ok] on success, with *out_measurement filled in, or
    /// [PamojaStatus::InvalidArgument] if the buffer is not four bytes. The part sends no
    /// checksum, so a well-sized read always decodes. # Safety bytes must point to at least
    /// bytes_len readable bytes, and out_measurement must point to a writable
    /// PamojaHdc1080Measurement.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_parse_measurement(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out PamojaHdc1080Measurement outMeasurement);

    /// <summary>
    /// Builds the HDC1080 measurement a sensor reporting these physical values would send.
    /// # Returns [PamojaStatus::Ok], with *out_measurement filled in, or
    /// [PamojaStatus::InvalidArgument] if the pointer is null. # Safety out_measurement
    /// must point to a writable PamojaHdc1080Measurement.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_measurement_from_physical(
        int milliCelsius,
        uint milliPercent,
        out PamojaHdc1080Measurement outMeasurement);

    /// <summary>
    /// Builds the four bytes an HDC1080 sends for a pair of raw registers. # Returns
    /// [PamojaStatus::Ok] on success, with the four bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least four writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_measurement_bytes(
        ushort temperatureRaw,
        ushort humidityRaw,
        Span<byte> outBytes);

    /// <summary>
    /// Parses an HDC1080 configuration register value. # Returns [PamojaStatus::Ok] on
    /// success, with *out_config filled in, or [PamojaStatus::Codec] if the
    /// humidity-resolution field carries the code the datasheet leaves undefined, which
    /// means the value did not come from a working part. # Safety out_config must point to
    /// a writable PamojaHdc1080Config.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_config_from_register(
        ushort raw,
        out PamojaHdc1080Config outConfig);

    /// <summary>
    /// Assembles an HDC1080 configuration register value. # Returns [PamojaStatus::Ok] on
    /// success, with *out_register set, or [PamojaStatus::InvalidArgument] if either
    /// resolution is not one the part offers. # Safety out_register must point to a
    /// writable uint16_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_config_to_register(
        PamojaHdc1080Config config,
        out ushort outRegister);

    /// <summary>
    /// Returns how long to wait after triggering an HDC1080 in a configuration. # Returns
    /// [PamojaStatus::Ok] on success, with *out_micros set, or
    /// [PamojaStatus::InvalidArgument] if either resolution is not one the part offers. #
    /// Safety out_micros must point to a writable uint32_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_conversion_time_micros(
        PamojaHdc1080Config config,
        out uint outMicros);

    /// <summary>
    /// Returns how long an HDC1080 temperature conversion takes at a resolution. # Returns
    /// [PamojaStatus::Ok] on success, with *out_micros set, or
    /// [PamojaStatus::InvalidArgument] if bits is not 14 or 11. # Safety out_micros must
    /// point to a writable uint32_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_temperature_conversion_micros(
        byte bits,
        out uint outMicros);

    /// <summary>
    /// Returns how long an HDC1080 humidity conversion takes at a resolution. # Returns
    /// [PamojaStatus::Ok] on success, with *out_micros set, or
    /// [PamojaStatus::InvalidArgument] if bits is not 14, 11, or 8. # Safety out_micros
    /// must point to a writable uint32_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_humidity_conversion_micros(
        byte bits,
        out uint outMicros);

    /// <summary>
    /// Returns the illuminance one count carries at an OPT3001 exponent. # Returns true
    /// when the exponent is one the part defines, with *out_milli_lux set to the step;
    /// false for a reserved exponent. # Safety out_milli_lux must point to a writable
    /// uint32_t.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_opt3001_lsb_milli_lux(
        byte exponent,
        out uint outMilliLux);

    /// <summary>
    /// Returns the full scale an OPT3001 range number covers. # Returns true when the range
    /// is one the part defines, with *out_milli_lux set to the full scale; false for a
    /// reserved range number, which has none. # Safety out_milli_lux must point to a
    /// writable uint32_t.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_opt3001_full_scale_milli_lux(
        byte rangeNumber,
        out uint outMilliLux);

    /// <summary>
    /// Converts a raw OPT3001 result register to milli-lux. # Returns The illuminance,
    /// exact in integer arithmetic.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_opt3001_milli_lux(ushort raw);

    /// <summary>Converts a raw OPT3001 result register to lux. # Returns The illuminance.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_opt3001_lux(ushort raw);

    /// <summary>
    /// Builds the OPT3001 result register that decodes to an illuminance. # Returns The
    /// register value, using the smallest exponent that fits and saturating at full scale.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_opt3001_raw_from_milli_lux(uint milliLux);

    /// <summary>
    /// Reads the two bytes an OPT3001 sends for a register, most significant first. #
    /// Returns [PamojaStatus::Ok] on success, with *out_word set, or
    /// [PamojaStatus::InvalidArgument] if the buffer is not two bytes. # Safety bytes must
    /// point to at least bytes_len readable bytes, and out_word must point to a writable
    /// uint16_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_opt3001_word_from_bytes(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out ushort outWord);

    /// <summary>
    /// Builds the two bytes an OPT3001 sends for a register, most significant first. #
    /// Returns [PamojaStatus::Ok] on success, with the two bytes written to out_bytes, or
    /// [PamojaStatus::InvalidArgument] if out_bytes is null. # Safety out_bytes must point
    /// to at least two writable bytes.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_opt3001_word_to_bytes(
        ushort word,
        Span<byte> outBytes);

    /// <summary>
    /// Assembles the 16-bit OPT3001 configuration register value. # Returns The register
    /// value to write, with the read-only status bits written as zero.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_opt3001_config_bits(PamojaOpt3001Config config);

    /// <summary>
    /// Parses a 16-bit OPT3001 configuration register value. # Returns [PamojaStatus::Ok],
    /// with *out_config filled in. Every register value decodes, so this fails only on a
    /// null pointer. # Safety out_config must point to a writable PamojaOpt3001Config.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_opt3001_config_from_bits(
        ushort bits,
        out PamojaOpt3001Config outConfig);

    /// <summary>
    /// Returns the conversion time an OPT3001 setting selects. # Returns The time in
    /// milliseconds: 800 for the long conversion, 100 otherwise.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_opt3001_conversion_millis(
        [MarshalAs(UnmanagedType.U1)] bool longConversion);

    /// <summary>
    /// Returns how many consecutive faults an OPT3001 fault-count code requires. # Returns
    /// The fault count: 1, 2, 4, or 8.
    /// </summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_opt3001_fault_count(byte code);

    /// <summary>
    /// Reports whether an OPT3001 range number sets the full scale automatically. # Returns
    /// true for the automatic range number, which has no fixed full scale.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_opt3001_is_automatic_range(byte rangeNumber);

    /// <summary>
    /// Returns the I2C address an INA226's A1 and A0 pin codes select. # Returns
    /// [PamojaStatus::Ok] on success, with *out_address set to a 7-bit address in
    /// 0x40..=0x4F, or [PamojaStatus::InvalidArgument] if either code is above 3. # Safety
    /// out_address must point to a writable uint8_t.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina226_address(
        byte a1,
        byte a0,
        out byte outAddress);

    /// <summary>
    /// Returns how many samples an INA226 averaging code folds into one result. # Returns
    /// The sample count, from 1 to 1024.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina226_averaging_samples(byte code);

    /// <summary>
    /// Returns the conversion time an INA226 code selects. # Returns The time in
    /// microseconds.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina226_conversion_micros(byte code);

    /// <summary>
    /// Reports whether an INA226 mode code converts the shunt voltage. # Returns true when
    /// the shunt is measured in that mode.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_ina226_measures_shunt(byte code);

    /// <summary>
    /// Reports whether an INA226 mode code converts the bus voltage. # Returns true when
    /// the bus is measured in that mode.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_ina226_measures_bus(byte code);

    /// <summary>
    /// Reports whether an INA226 mode code keeps converting after the first result. #
    /// Returns true for the continuous modes.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_ina226_is_continuous(byte code);

    /// <summary>
    /// Parses an INA226 configuration register value. # Returns [PamojaStatus::Ok], with
    /// *out_config filled in. Every register value decodes, so this fails only on a null
    /// pointer. # Safety out_config must point to a writable PamojaIna226Config.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina226_config_from_register(
        ushort raw,
        out PamojaIna226Config outConfig);

    /// <summary>
    /// Assembles an INA226 configuration register value. # Returns The register value to
    /// write.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina226_config_to_register(PamojaIna226Config config);

    /// <summary>
    /// Returns how often an INA226 in a configuration updates its results. # Returns The
    /// interval in microseconds: every conversion the mode takes, averaged.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina226_update_micros(PamojaIna226Config config);

    /// <summary>
    /// Parses an INA226 Mask/Enable register value. # Returns [PamojaStatus::Ok], with
    /// *out_mask filled in. Every register value decodes, so this fails only on a null
    /// pointer. # Safety out_mask must point to a writable PamojaIna226MaskEnable.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina226_mask_enable_from_register(
        ushort raw,
        out PamojaIna226MaskEnable outMask);

    /// <summary>
    /// Assembles an INA226 Mask/Enable register value. # Returns The register value to
    /// write, with the read-only flags written as zero.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina226_mask_enable_to_register(PamojaIna226MaskEnable mask);

    /// <summary>
    /// Returns the alert function an INA226 pin actually responds to. # Returns true when
    /// one limit function is selected, with *out_function set to it; false when none is, so
    /// the pin only ever signals a completed conversion. # Safety out_function must point
    /// to a writable PamojaIna226AlertFunction.
    /// </summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_ina226_active_alert_function(
        PamojaIna226MaskEnable mask,
        out PamojaIna226AlertFunction outFunction);

    /// <summary>
    /// Splits an INA226 die-ID register into its device and revision fields. # Returns
    /// [PamojaStatus::Ok], with *out_die_id filled in. Every register value decodes, so
    /// this fails only on a null pointer. # Safety out_die_id must point to a writable
    /// PamojaIna226DieId.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina226_die_id(
        ushort raw,
        out PamojaIna226DieId outDieId);

    /// <summary>
    /// Checks that a pair of identification registers belongs to an INA226. # Returns
    /// [PamojaStatus::Ok] on success, with *out_die_id filled in, or [PamojaStatus::Codec]
    /// if either register carries something other than the values the datasheet fixes,
    /// which means a different part, or nothing at all, answered at that address. # Safety
    /// out_die_id must point to a writable PamojaIna226DieId.
    /// </summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina226_identify(
        ushort manufacturerId,
        ushort dieId,
        out PamojaIna226DieId outDieId);

    /// <summary>
    /// Computes the INA226 calibration register for a shunt and current resolution. #
    /// Returns The register value to write.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina226_calibration(
        uint currentLsbMicroamps,
        uint shuntMilliohms);

    /// <summary>
    /// Returns the smallest current resolution that still covers an expected maximum. #
    /// Returns The current LSB in microamps.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina226_minimum_current_lsb_microamps(uint maxExpectedMicroamps);

    /// <summary>
    /// Converts a raw INA226 shunt-voltage register to nanovolts. # Returns The shunt
    /// voltage, exact in integer arithmetic at 2.5 uV per count.
    /// </summary>
    [LibraryImport(Library)]
    public static partial int pamoja_ina226_shunt_nanovolts(short raw);

    /// <summary>
    /// Converts a raw INA226 shunt-voltage register to millivolts. # Returns The shunt
    /// voltage.
    /// </summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ina226_shunt_millivolts(short raw);

    /// <summary>
    /// Converts a raw INA226 bus-voltage register to microvolts. # Returns The bus voltage,
    /// exact in integer arithmetic at 1.25 mV per count.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina226_bus_microvolts(ushort raw);

    /// <summary>Converts a raw INA226 bus-voltage register to volts. # Returns The bus voltage.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ina226_bus_volts(ushort raw);

    /// <summary>
    /// Converts a raw INA226 current register to microamps. # Returns The current, at the
    /// resolution the calibration selected.
    /// </summary>
    [LibraryImport(Library)]
    public static partial int pamoja_ina226_current_microamps(
        short raw,
        uint currentLsbMicroamps);

    /// <summary>Converts a raw INA226 current register to amps. # Returns The current.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ina226_current_amps(
        short raw,
        uint currentLsbMicroamps);

    /// <summary>
    /// Converts a raw INA226 power register to microwatts. # Returns The power. The power
    /// LSB is fixed at twenty-five times the current LSB.
    /// </summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina226_power_microwatts(
        ushort raw,
        uint currentLsbMicroamps);

    /// <summary>Converts a raw INA226 power register to watts. # Returns The power.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ina226_power_watts(
        ushort raw,
        uint currentLsbMicroamps);

    /// <summary>
    /// Builds the INA226 shunt-voltage register a monitor reports for a shunt voltage. #
    /// Returns The signed register value, at 2.5 uV per count.
    /// </summary>
    [LibraryImport(Library)]
    public static partial short pamoja_ina226_shunt_register(int nanovolts);

    /// <summary>
    /// Builds the INA226 bus-voltage register a monitor reports for a bus voltage. #
    /// Returns The register value, at 1.25 mV per count.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina226_bus_register(uint microvolts);

    /// <summary>
    /// Builds the INA226 current register a monitor reports for a current. # Returns The
    /// signed register value, or zero if current_lsb_microamps is zero.
    /// </summary>
    [LibraryImport(Library)]
    public static partial short pamoja_ina226_current_register(
        int microamps,
        uint currentLsbMicroamps);

    /// <summary>
    /// Builds the INA226 power register a monitor reports for a power. # Returns The
    /// register value, or zero if current_lsb_microamps is zero.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina226_power_register(
        uint microwatts,
        uint currentLsbMicroamps);

    /// <summary>
    /// Computes the INA226 current register the chip derives from a shunt reading. #
    /// Returns The signed current register the part's own arithmetic produces.
    /// </summary>
    [LibraryImport(Library)]
    public static partial short pamoja_ina226_current_register_from_shunt(
        short shunt,
        ushort calibration);

    /// <summary>
    /// Computes the INA226 power register the chip derives from a current reading. #
    /// Returns The power register the part's own arithmetic produces.
    /// </summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina226_power_register_from_current(
        short current,
        ushort bus);
}
