using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Texas Instruments INA226 current, voltage, and power monitor. Every call goes
/// straight to the pamoja C ABI, which decodes exactly what the manufacturer's
/// datasheet specifies.
/// </summary>
public static class Ina226
{
    /// <summary>Returns the I2C address an INA226's A1 and A0 pin codes select.</summary>
    /// <param name="a1">The a1.</param>
    /// <param name="a0">The a0.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static byte Address(byte a1, byte a0)
    {
        byte value;
        Status.ThrowIfError(NativeMethods.pamoja_ina226_address(a1, a0, out value));
        return value;
    }

    /// <summary>Returns how many samples an INA226 averaging code folds into one result.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort AveragingSamples(byte code) =>
        NativeMethods.pamoja_ina226_averaging_samples(code);

    /// <summary>Returns the conversion time an INA226 code selects.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint ConversionMicros(byte code) =>
        NativeMethods.pamoja_ina226_conversion_micros(code);

    /// <summary>Reports whether an INA226 mode code converts the shunt voltage.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool MeasuresShunt(byte code) =>
        NativeMethods.pamoja_ina226_measures_shunt(code);

    /// <summary>Reports whether an INA226 mode code converts the bus voltage.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool MeasuresBus(byte code) =>
        NativeMethods.pamoja_ina226_measures_bus(code);

    /// <summary>Reports whether an INA226 mode code keeps converting after the first result.</summary>
    /// <param name="code">The code.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static bool IsContinuous(byte code) =>
        NativeMethods.pamoja_ina226_is_continuous(code);

    /// <summary>Parses an INA226 configuration register value.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaIna226Config ConfigFromRegister(ushort raw)
    {
        PamojaIna226Config value;
        Status.ThrowIfError(NativeMethods.pamoja_ina226_config_from_register(raw, out value));
        return value;
    }

    /// <summary>Assembles an INA226 configuration register value.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort ConfigToRegister(PamojaIna226Config config) =>
        NativeMethods.pamoja_ina226_config_to_register(config);

    /// <summary>Returns how often an INA226 in a configuration updates its results.</summary>
    /// <param name="config">The config.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint UpdateMicros(PamojaIna226Config config) =>
        NativeMethods.pamoja_ina226_update_micros(config);

    /// <summary>Parses an INA226 Mask/Enable register value.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaIna226MaskEnable MaskEnableFromRegister(ushort raw)
    {
        PamojaIna226MaskEnable value;
        Status.ThrowIfError(NativeMethods.pamoja_ina226_mask_enable_from_register(raw, out value));
        return value;
    }

    /// <summary>Assembles an INA226 Mask/Enable register value.</summary>
    /// <param name="mask">The mask.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort MaskEnableToRegister(PamojaIna226MaskEnable mask) =>
        NativeMethods.pamoja_ina226_mask_enable_to_register(mask);

    /// <summary>Returns the alert function an INA226 pin actually responds to.</summary>
    /// <param name="mask">The mask.</param>
    /// <returns>The value, or null where the part defines none.</returns>
    public static PamojaIna226AlertFunction? ActiveAlertFunction(PamojaIna226MaskEnable mask)
    {
        PamojaIna226AlertFunction value;
        return NativeMethods.pamoja_ina226_active_alert_function(mask, out value) ? value : null;
    }

    /// <summary>Splits an INA226 die-ID register into its device and revision fields.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaIna226DieId DieId(ushort raw)
    {
        PamojaIna226DieId value;
        Status.ThrowIfError(NativeMethods.pamoja_ina226_die_id(raw, out value));
        return value;
    }

    /// <summary>Checks that a pair of identification registers belongs to an INA226.</summary>
    /// <param name="manufacturerId">The manufacturer id.</param>
    /// <param name="dieId">The die id.</param>
    /// <returns>The decoded value.</returns>
    /// <exception cref="PamojaException">The device data is not valid.</exception>
    public static PamojaIna226DieId Identify(ushort manufacturerId, ushort dieId)
    {
        PamojaIna226DieId value;
        Status.ThrowIfError(NativeMethods.pamoja_ina226_identify(manufacturerId, dieId, out value));
        return value;
    }

    /// <summary>Computes the INA226 calibration register for a shunt and current resolution.</summary>
    /// <param name="currentLsbMicroamps">The current lsb microamps.</param>
    /// <param name="shuntMilliohms">The shunt milliohms.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort Calibration(uint currentLsbMicroamps, uint shuntMilliohms) =>
        NativeMethods.pamoja_ina226_calibration(currentLsbMicroamps, shuntMilliohms);

    /// <summary>Returns the smallest current resolution that still covers an expected maximum.</summary>
    /// <param name="maxExpectedMicroamps">The max expected microamps.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint MinimumCurrentLsbMicroamps(uint maxExpectedMicroamps) =>
        NativeMethods.pamoja_ina226_minimum_current_lsb_microamps(maxExpectedMicroamps);

    /// <summary>Converts a raw INA226 shunt-voltage register to nanovolts.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static int ShuntNanovolts(short raw) =>
        NativeMethods.pamoja_ina226_shunt_nanovolts(raw);

    /// <summary>Converts a raw INA226 shunt-voltage register to millivolts.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float ShuntMillivolts(short raw) =>
        NativeMethods.pamoja_ina226_shunt_millivolts(raw);

    /// <summary>Converts a raw INA226 bus-voltage register to microvolts.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint BusMicrovolts(ushort raw) =>
        NativeMethods.pamoja_ina226_bus_microvolts(raw);

    /// <summary>Converts a raw INA226 bus-voltage register to volts.</summary>
    /// <param name="raw">The raw.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float BusVolts(ushort raw) =>
        NativeMethods.pamoja_ina226_bus_volts(raw);

    /// <summary>Converts a raw INA226 current register to microamps.</summary>
    /// <param name="raw">The raw.</param>
    /// <param name="currentLsbMicroamps">The current lsb microamps.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static int CurrentMicroamps(short raw, uint currentLsbMicroamps) =>
        NativeMethods.pamoja_ina226_current_microamps(raw, currentLsbMicroamps);

    /// <summary>Converts a raw INA226 current register to amps.</summary>
    /// <param name="raw">The raw.</param>
    /// <param name="currentLsbMicroamps">The current lsb microamps.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float CurrentAmps(short raw, uint currentLsbMicroamps) =>
        NativeMethods.pamoja_ina226_current_amps(raw, currentLsbMicroamps);

    /// <summary>Converts a raw INA226 power register to microwatts.</summary>
    /// <param name="raw">The raw.</param>
    /// <param name="currentLsbMicroamps">The current lsb microamps.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static uint PowerMicrowatts(ushort raw, uint currentLsbMicroamps) =>
        NativeMethods.pamoja_ina226_power_microwatts(raw, currentLsbMicroamps);

    /// <summary>Converts a raw INA226 power register to watts.</summary>
    /// <param name="raw">The raw.</param>
    /// <param name="currentLsbMicroamps">The current lsb microamps.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static float PowerWatts(ushort raw, uint currentLsbMicroamps) =>
        NativeMethods.pamoja_ina226_power_watts(raw, currentLsbMicroamps);

    /// <summary>Builds the INA226 shunt-voltage register a monitor reports for a shunt voltage.</summary>
    /// <param name="nanovolts">The nanovolts.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static short ShuntRegister(int nanovolts) =>
        NativeMethods.pamoja_ina226_shunt_register(nanovolts);

    /// <summary>Builds the INA226 bus-voltage register a monitor reports for a bus voltage.</summary>
    /// <param name="microvolts">The microvolts.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort BusRegister(uint microvolts) =>
        NativeMethods.pamoja_ina226_bus_register(microvolts);

    /// <summary>Builds the INA226 current register a monitor reports for a current.</summary>
    /// <param name="microamps">The microamps.</param>
    /// <param name="currentLsbMicroamps">The current lsb microamps.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static short CurrentRegister(int microamps, uint currentLsbMicroamps) =>
        NativeMethods.pamoja_ina226_current_register(microamps, currentLsbMicroamps);

    /// <summary>Builds the INA226 power register a monitor reports for a power.</summary>
    /// <param name="microwatts">The microwatts.</param>
    /// <param name="currentLsbMicroamps">The current lsb microamps.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort PowerRegister(uint microwatts, uint currentLsbMicroamps) =>
        NativeMethods.pamoja_ina226_power_register(microwatts, currentLsbMicroamps);

    /// <summary>Computes the INA226 current register the chip derives from a shunt reading.</summary>
    /// <param name="shunt">The shunt.</param>
    /// <param name="calibration">The calibration.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static short CurrentRegisterFromShunt(short shunt, ushort calibration) =>
        NativeMethods.pamoja_ina226_current_register_from_shunt(shunt, calibration);

    /// <summary>Computes the INA226 power register the chip derives from a current reading.</summary>
    /// <param name="current">The current.</param>
    /// <param name="bus">The bus.</param>
    /// <returns>The value the C ABI computes.</returns>
    public static ushort PowerRegisterFromCurrent(short current, ushort bus) =>
        NativeMethods.pamoja_ina226_power_register_from_current(current, bus);
}
