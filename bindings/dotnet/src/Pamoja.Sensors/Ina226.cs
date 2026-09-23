using Pamoja.Hal;
using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>
/// A Texas Instruments INA226 current, voltage, and power monitor, and a driver for one on an
/// I2C bus.
/// </summary>
/// <remarks>
/// <para>
/// The static members are the part's datasheet, decoded exactly as the manufacturer
/// specifies, and <see cref="Sim"/>, a part that is not there.
/// </para>
/// <para>
/// An instance drives one part over an <see cref="I2cBus"/>: it resets the part, checks its
/// identity, programs the calibration for the shunt and the current it is sized for, and
/// triggers one shunt and bus conversion per <see cref="Measure"/>. Nothing is sent until
/// <see cref="Init"/> or the first <see cref="Measure"/>.
/// </para>
/// </remarks>
public sealed class Ina226 : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a driver for the part at <paramref name="address"/> on <paramref name="bus"/>.</summary>
    /// <param name="bus">The bus the part is on.</param>
    /// <param name="address">The address A1 and A0 select, which <see cref="Address"/> works out.</param>
    /// <param name="shuntMilliohms">The shunt resistance in milliohms.</param>
    /// <param name="maxMicroamps">The largest current the shunt will carry, which sets the finest current step.</param>
    /// <param name="currentLsbMicroamps">A current step to use instead, such as a round 100, or 0 for the finest.</param>
    /// <param name="config">The averaging and conversion times, or null for the power-on settings.</param>
    /// <exception cref="PamojaException">The native driver could not be created.</exception>
    public Ina226(
        I2cBus bus,
        byte address,
        uint shuntMilliohms = 100,
        uint maxMicroamps = 3_200_000,
        uint currentLsbMicroamps = 0,
        PamojaIna226Config? config = null)
    {
        ArgumentNullException.ThrowIfNull(bus);
        PamojaIna226Settings settings = NativeMethods.pamoja_ina226_settings_default();
        settings.ShuntMilliohms = shuntMilliohms;
        settings.MaxMicroamps = maxMicroamps;
        settings.CurrentLsbMicroamps = currentLsbMicroamps;
        settings.Config = config ?? settings.Config;
        IntPtr sensor = IntPtr.Zero;
        Status.ThrowIfError(bus.Use(held =>
            NativeMethods.pamoja_ina226_new(held, address, settings, out sensor)));
        _handle = new NativeHandle(sensor, NativeMethods.pamoja_ina226_free);
    }

    /// <summary>The current step the driver programs, in microamps per count.</summary>
    public uint CurrentLsbMicroamps => _handle.Use(NativeMethods.pamoja_ina226_current_lsb);

    /// <summary>The calibration word the driver programs.</summary>
    public ushort CalibrationWord => _handle.Use(NativeMethods.pamoja_ina226_calibration_word);

    /// <summary>The die id read at initialization, or null before it.</summary>
    public PamojaIna226DieId? Identity =>
        _handle.Use(sensor => NativeMethods.pamoja_ina226_identity(sensor, out PamojaIna226DieId die)
            ? die
            : (PamojaIna226DieId?)null);

    /// <summary>Resets the part, checks it is an INA226, and programs the configuration and calibration.</summary>
    /// <exception cref="PamojaException">
    /// Nothing answered, the id registers are not an INA226's, or the calibration did not read back.
    /// </exception>
    public void Init() => Status.ThrowIfError(_handle.Use(NativeMethods.pamoja_ina226_init));

    /// <summary>Triggers one shunt and bus conversion and reads every result, initializing the part first if needed.</summary>
    /// <returns>The registers as read, and what they mean.</returns>
    /// <exception cref="PamojaException">As <see cref="Init"/>, and when the conversion-ready flag never sets.</exception>
    public Ina226Reading Measure()
    {
        PamojaIna226Reading reading = default;
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_ina226_measure(sensor, out reading)));
        return Ina226Reading.From(reading);
    }

    /// <summary>Programs the alert pin: which limit it watches, and the limit.</summary>
    /// <param name="mask">The mask/enable settings, one alert function at a time.</param>
    /// <param name="limit">The alert-limit register, in the units of the register the function watches.</param>
    /// <exception cref="PamojaException">The transfer failed.</exception>
    public void SetAlert(PamojaIna226MaskEnable mask, ushort limit) =>
        Status.ThrowIfError(_handle.Use(sensor => NativeMethods.pamoja_ina226_set_alert(sensor, mask, limit)));

    /// <summary>Releases the driver and its share of the bus.</summary>
    public void Dispose() => _handle.Dispose();

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

    /// <summary>An INA226 that is not there, for a bus with nothing plugged in.</summary>
    /// <remarks>
    /// A monitor's current and power registers count in steps the calibration sets, so
    /// <see cref="Reporting"/> takes the same shunt and largest current a driver is given. The
    /// part carries TI's manufacturer id and the INA226 die id, and its conversion-ready flag
    /// is set, so every conversion reads as finished.
    /// </remarks>
    public static class Sim
    {
        /// <summary>The shunt <see cref="Part"/> sits across, in milliohms.</summary>
        public const uint ShuntMilliohms = 100;

        /// <summary>The largest current <see cref="Part"/> is sized for, in microamps.</summary>
        public const uint MaxMicroamps = 3_200_000;

        /// <summary>The bus voltage <see cref="Part"/> reports, in microvolts.</summary>
        public const uint BusMicrovolts = 12_000_000;

        /// <summary>The current <see cref="Part"/> reports, in microamps.</summary>
        public const int Microamps = 500_000;

        /// <summary>Makes a part carrying <see cref="Microamps"/> at <see cref="BusMicrovolts"/> through the shunt a driver starts with.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Part(byte address) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_ina226_sim_part(address),
                NativeMethods.pamoja_i2c_part_free,
                "simulated INA226"));

        /// <summary>Makes a part that reads what it is asked to.</summary>
        /// <param name="address">The address it answers to.</param>
        /// <param name="shuntMilliohms">The shunt, as the driver is given it.</param>
        /// <param name="maxMicroamps">The largest current, as the driver is given it.</param>
        /// <param name="busMicrovolts">The bus voltage it reports.</param>
        /// <param name="microamps">The current it reports; negative flows the other way through the shunt.</param>
        /// <returns>The part, to put on a simulated bus.</returns>
        public static WordPart Reporting(
            byte address,
            uint shuntMilliohms,
            uint maxMicroamps,
            uint busMicrovolts,
            int microamps) =>
            new(NativeHandle.Create(
                NativeMethods.pamoja_ina226_sim_reporting(address, shuntMilliohms, maxMicroamps, busMicrovolts, microamps),
                NativeMethods.pamoja_i2c_part_free,
                "simulated INA226"));
    }
}
