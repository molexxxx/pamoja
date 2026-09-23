using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the sensor drivers that run over an I2C bus, their simulated
/// parts, and the DS18B20 read through the kernel's 1-Wire files, mirroring <c>pamoja.h</c>
/// one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. The BME280 driver
/// sits with its decoders in the sensing file. Every part must be updated together with the
/// generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The INA219 configuration register's power-on value.</summary>
    public const ushort Ina219ConfigReset = 0x399F;

    /// <summary>Returns the I2C address an INA219's A1 and A0 pin codes select.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina219_address(byte a1, byte a0, out byte outAddress);

    /// <summary>Assembles an INA219 configuration register value.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina219_config_bits(PamojaIna219Config config);

    /// <summary>Parses an INA219 configuration register value.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina219_config_from_bits(
        ushort bits,
        out PamojaIna219Config outConfig);

    /// <summary>Returns how long one INA219 conversion cycle takes, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina219_conversion_micros(PamojaIna219Config config);

    /// <summary>Returns how long one INA219 conversion takes at an ADC code, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina219_adc_conversion_micros(byte code);

    /// <summary>Returns the shunt-voltage range an INA219 gain code selects, in millivolts.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina219_gain_range_millivolts(byte code);

    /// <summary>Returns how long an ADS1115 conversion takes at a data-rate code, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ads1115_conversion_micros(byte dataRate);

    /// <summary>Decodes the text the kernel serves for a DS18B20.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_ds18b20_parse_w1_slave(
        string text,
        out PamojaDs18b20Reading outReading);

    /// <summary>Renders the text the kernel serves for a DS18B20 scratchpad it read cleanly.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ds18b20_w1_slave_text(
        ReadOnlySpan<byte> bytes,
        nuint len,
        out IntPtr outText);

    /// <summary>Returns the settings a BMP280 driver starts with.</summary>
    [LibraryImport(Library)]
    public static partial PamojaBmp280Settings pamoja_bmp280_settings_default();

    /// <summary>Creates a BMP280 driver on a bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_new(
        IntPtr bus,
        byte address,
        PamojaBmp280Settings settings,
        out IntPtr outSensor);

    /// <summary>Resets, identifies, and configures a BMP280.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_init(IntPtr sensor);

    /// <summary>Runs one forced BMP280 measurement and compensates it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_measure(IntPtr sensor, out PamojaBmp280Reading outReading);

    /// <summary>Copies the trimming coefficients a BMP280 driver read at initialization.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_bmp280_coefficients(IntPtr sensor, out PamojaBmp280Coefficients outCoefficients);

    /// <summary>Releases a BMP280 driver. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_bmp280_free(IntPtr sensor);

    /// <summary>Creates a simulated BMP280 holding a real part's trimming and measurement.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_bmp280_sim_part(byte address);

    /// <summary>Creates a simulated BMP280 that reads what it is asked to.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_bmp280_sim_reporting(byte address, float celsius, float hectopascals);

    /// <summary>Copies the 24 trimming bytes a simulated BMP280 holds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_sim_calibration(Span<byte> outCalibration);

    /// <summary>Copies the six data registers a simulated BMP280 holds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_sim_burst(Span<byte> outBurst);

    /// <summary>Builds the six data registers a simulated BMP280 holds for a reading.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_bmp280_sim_burst_for(
        float celsius,
        float hectopascals,
        Span<byte> outBurst);

    /// <summary>Creates a TMP117 driver on a bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_tmp117_new(IntPtr bus, byte address, byte averaging, out IntPtr outSensor);

    /// <summary>Identifies and configures a TMP117.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_tmp117_init(IntPtr sensor);

    /// <summary>Runs one TMP117 conversion and reads the temperature.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_tmp117_measure(IntPtr sensor, out PamojaTmp117Reading outReading);

    /// <summary>Writes the TMP117's high and low limits.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_tmp117_set_alert_limits(IntPtr sensor, float highCelsius, float lowCelsius);

    /// <summary>Reads the TMP117's alert flags.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_tmp117_alerts(IntPtr sensor, out PamojaTmp117Alerts outAlerts);

    /// <summary>Reports the silicon revision a TMP117 driver read at initialization.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_tmp117_silicon_revision(IntPtr sensor, out byte outRevision);

    /// <summary>Releases a TMP117 driver. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_tmp117_free(IntPtr sensor);

    /// <summary>Creates a simulated TMP117 reading its default temperature.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_tmp117_sim_part(byte address);

    /// <summary>Creates a simulated TMP117 that reads what it is asked to.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_tmp117_sim_reporting(byte address, float celsius);

    /// <summary>Returns the settings an OPT3001 driver starts with.</summary>
    [LibraryImport(Library)]
    public static partial PamojaOpt3001Settings pamoja_opt3001_settings_default();

    /// <summary>Creates an OPT3001 driver on a bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_opt3001_new(
        IntPtr bus,
        byte address,
        PamojaOpt3001Settings settings,
        out IntPtr outSensor);

    /// <summary>Identifies and configures an OPT3001.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_opt3001_init(IntPtr sensor);

    /// <summary>Runs one OPT3001 conversion and reads the illuminance.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_opt3001_measure(IntPtr sensor, out PamojaOpt3001Reading outReading);

    /// <summary>Writes the OPT3001's low and high limits, in millilux.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_opt3001_set_limits(IntPtr sensor, uint lowMilliLux, uint highMilliLux);

    /// <summary>Copies the configuration an OPT3001 driver writes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_opt3001_configuration(IntPtr sensor, out PamojaOpt3001Config outConfig);

    /// <summary>Releases an OPT3001 driver. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_opt3001_free(IntPtr sensor);

    /// <summary>Creates a simulated OPT3001 reading its default illuminance.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_opt3001_sim_part(byte address);

    /// <summary>Creates a simulated OPT3001 that reads what it is asked to.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_opt3001_sim_reporting(byte address, float lux);

    /// <summary>Returns the settings an HDC1080 driver starts with.</summary>
    [LibraryImport(Library)]
    public static partial PamojaHdc1080Settings pamoja_hdc1080_settings_default();

    /// <summary>Creates an HDC1080 driver on a bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_new(
        IntPtr bus,
        PamojaHdc1080Settings settings,
        out IntPtr outSensor);

    /// <summary>Identifies and configures an HDC1080.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_init(IntPtr sensor);

    /// <summary>Runs one HDC1080 acquisition of both channels.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_measure(IntPtr sensor, out PamojaHdc1080Measurement outMeasurement);

    /// <summary>Switches the HDC1080's heater on or off.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_heater(IntPtr sensor, [MarshalAs(UnmanagedType.U1)] bool on);

    /// <summary>Copies the configuration an HDC1080 driver writes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_hdc1080_configuration(IntPtr sensor, out PamojaHdc1080Config outConfig);

    /// <summary>Releases an HDC1080 driver. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_hdc1080_free(IntPtr sensor);

    /// <summary>Creates a simulated HDC1080 reading its default temperature and humidity.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_hdc1080_sim_part();

    /// <summary>Creates a simulated HDC1080 that reads what it is asked to.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_hdc1080_sim_reporting(float celsius, float relativeHumidity);

    /// <summary>Returns the settings an INA219 driver starts with.</summary>
    [LibraryImport(Library)]
    public static partial PamojaIna219Settings pamoja_ina219_settings_default();

    /// <summary>Creates an INA219 driver on a bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina219_new(
        IntPtr bus,
        byte address,
        PamojaIna219Settings settings,
        out IntPtr outSensor);

    /// <summary>Resets, configures, and calibrates an INA219.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina219_init(IntPtr sensor);

    /// <summary>Runs one INA219 conversion and reads every result.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina219_measure(IntPtr sensor, out PamojaIna219Reading outReading);

    /// <summary>Returns the current step an INA219 driver programs.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina219_current_lsb(IntPtr sensor);

    /// <summary>Returns the calibration word an INA219 driver programs.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina219_calibration_word(IntPtr sensor);

    /// <summary>Releases an INA219 driver. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_ina219_free(IntPtr sensor);

    /// <summary>Creates a simulated INA219 carrying its default load.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ina219_sim_part(byte address);

    /// <summary>Creates a simulated INA219 that reads what it is asked to.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ina219_sim_reporting(
        byte address,
        uint shuntMilliohms,
        uint maxMicroamps,
        uint busMillivolts,
        int microamps);

    /// <summary>Returns the settings an INA226 driver starts with.</summary>
    [LibraryImport(Library)]
    public static partial PamojaIna226Settings pamoja_ina226_settings_default();

    /// <summary>Creates an INA226 driver on a bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina226_new(
        IntPtr bus,
        byte address,
        PamojaIna226Settings settings,
        out IntPtr outSensor);

    /// <summary>Resets, identifies, configures, and calibrates an INA226.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina226_init(IntPtr sensor);

    /// <summary>Runs one INA226 conversion and reads every result.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina226_measure(IntPtr sensor, out PamojaIna226Reading outReading);

    /// <summary>Programs the INA226's alert pin.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ina226_set_alert(IntPtr sensor, PamojaIna226MaskEnable mask, ushort limit);

    /// <summary>Returns the current step an INA226 driver programs.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_ina226_current_lsb(IntPtr sensor);

    /// <summary>Returns the calibration word an INA226 driver programs.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_ina226_calibration_word(IntPtr sensor);

    /// <summary>Reports the die id an INA226 driver read at initialization.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_ina226_identity(IntPtr sensor, out PamojaIna226DieId outDieId);

    /// <summary>Releases an INA226 driver. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_ina226_free(IntPtr sensor);

    /// <summary>Creates a simulated INA226 carrying its default load.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ina226_sim_part(byte address);

    /// <summary>Creates a simulated INA226 that reads what it is asked to.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ina226_sim_reporting(
        byte address,
        uint shuntMilliohms,
        uint maxMicroamps,
        uint busMicrovolts,
        int microamps);

    /// <summary>Returns the settings an ADS1115 driver starts with.</summary>
    [LibraryImport(Library)]
    public static partial PamojaAds1115Settings pamoja_ads1115_settings_default();

    /// <summary>Creates an ADS1115 driver on a bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ads1115_new(
        IntPtr bus,
        byte address,
        PamojaAds1115Settings settings,
        out IntPtr outSensor);

    /// <summary>Configures an ADS1115 and reads the configuration back.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ads1115_init(IntPtr sensor);

    /// <summary>Runs one ADS1115 conversion of the configured input.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ads1115_sample(IntPtr sensor, out PamojaAds1115Sample outSample);

    /// <summary>Runs one ADS1115 conversion of another input.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ads1115_sample_input(IntPtr sensor, byte mux, out PamojaAds1115Sample outSample);

    /// <summary>Copies the configuration an ADS1115 driver writes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ads1115_config(IntPtr sensor, out PamojaAds1115Config outConfig);

    /// <summary>Releases an ADS1115 driver. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_ads1115_free(IntPtr sensor);

    /// <summary>Creates a simulated ADS1115 reading its default voltage.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ads1115_sim_part(byte address);

    /// <summary>Creates a simulated ADS1115 that reads what it is asked to.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ads1115_sim_reporting(byte address, byte pga, float volts);

    /// <summary>Creates an SHT3x driver on a bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_new(IntPtr bus, byte address, byte repeatability, out IntPtr outSensor);

    /// <summary>Resets an SHT3x and reads its status.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_init(IntPtr sensor);

    /// <summary>Runs one SHT3x single-shot measurement.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_measure(IntPtr sensor, out PamojaSht3xMeasurement outMeasurement);

    /// <summary>Reads the SHT3x status register.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_read_status(IntPtr sensor, out PamojaSht3xStatus outStatus);

    /// <summary>Reports the status an SHT3x driver last read.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_sht3x_last_status(IntPtr sensor, out PamojaSht3xStatus outStatus);

    /// <summary>Switches the SHT3x heater on.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_heater_on(IntPtr sensor);

    /// <summary>Switches the SHT3x heater off.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sht3x_heater_off(IntPtr sensor);

    /// <summary>Releases an SHT3x driver. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_sht3x_free(IntPtr sensor);

    /// <summary>Creates a simulated SHT3x reading its default temperature and humidity.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_sht3x_sim_part(byte address);

    /// <summary>Creates a simulated SHT3x that reads what it is asked to.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_sht3x_sim_reporting(byte address, float celsius, float relativeHumidity);

    /// <summary>Creates an SCD4x driver on a bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_new(IntPtr bus, out IntPtr outSensor);

    /// <summary>Reads the SCD4x serial number and starts periodic measurement.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_init(IntPtr sensor);

    /// <summary>Waits for the next SCD4x periodic result and reads it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_measure(IntPtr sensor, out PamojaScd4xMeasurement outMeasurement);

    /// <summary>Runs one on-demand SCD41 measurement.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_measure_single_shot(
        IntPtr sensor,
        out PamojaScd4xMeasurement outMeasurement);

    /// <summary>Asks an SCD4x whether a periodic result is waiting.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_poll_ready(
        IntPtr sensor,
        [MarshalAs(UnmanagedType.U1)] out bool outReady);

    /// <summary>Stops SCD4x periodic measurement.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_stop(IntPtr sensor);

    /// <summary>Starts SCD4x periodic measurement.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_start(IntPtr sensor);

    /// <summary>Sets the SCD4x temperature offset, in millidegrees.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_set_temperature_offset(IntPtr sensor, uint milliCelsius);

    /// <summary>Sets the altitude an SCD4x corrects for, in meters.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_scd4x_set_sensor_altitude(IntPtr sensor, ushort meters);

    /// <summary>Reports the serial number an SCD4x driver read at initialization.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_scd4x_serial(IntPtr sensor, out ulong outSerial);

    /// <summary>Releases an SCD4x driver. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_scd4x_free(IntPtr sensor);

    /// <summary>Creates a simulated SCD4x reading its default air.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_scd4x_sim_part();

    /// <summary>Creates a simulated SCD4x that reads what it is asked to.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_scd4x_sim_reporting(ushort co2Ppm, float celsius, float relativeHumidity);

    /// <summary>Names a DS18B20 the kernel serves by its serial.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_ds18b20_thermometer_new(string serial, out IntPtr outThermometer);

    /// <summary>Names a DS18B20 by the path of its <c>w1_slave</c> file.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_ds18b20_thermometer_at(string path, out IntPtr outThermometer);

    /// <summary>Returns the path of the file a thermometer reads, as an owned string.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ds18b20_thermometer_path(IntPtr thermometer);

    /// <summary>
    /// Returns the serial in a thermometer's directory name as an owned string, or null when the
    /// file does not sit in a DS18B20's directory.
    /// </summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ds18b20_thermometer_serial(IntPtr thermometer);

    /// <summary>Reads and decodes a thermometer's file.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ds18b20_thermometer_read(
        IntPtr thermometer,
        out PamojaDs18b20Reading outReading);

    /// <summary>Releases a thermometer. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_ds18b20_thermometer_free(IntPtr thermometer);

    /// <summary>Lists every DS18B20 the kernel has found under a directory, or the default one.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_ds18b20_discover(string? devices, out IntPtr outThermometers);

    /// <summary>Returns how many thermometers a list holds.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_ds18b20_thermometers_len(IntPtr thermometers);

    /// <summary>Copies one thermometer out of a list, or returns null.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ds18b20_thermometers_get(IntPtr thermometers, nuint index);

    /// <summary>Releases a list of thermometers. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_ds18b20_thermometers_free(IntPtr thermometers);
}
