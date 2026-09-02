using System.Runtime.InteropServices;

using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>Which control policy a profile applies to each reading.</summary>
public enum ControlKind
{
    /// <summary>Hold a reading near a setpoint by switching an output on and off.</summary>
    Setpoint = 0,

    /// <summary>Watch a falling level and warn before it reaches empty.</summary>
    Level = 1,

    /// <summary>Warn when a reading changes faster than a limit.</summary>
    Surge = 2,

    /// <summary>Report readings only, with no output and no alerts.</summary>
    Monitor = 3,
}

/// <summary>Which threshold a reading crossed.</summary>
public enum AlertKind
{
    /// <summary>A controlled reading drifted outside its safe band.</summary>
    OutOfRange = 1,

    /// <summary>A falling level will reach empty within a few more samples.</summary>
    RunningOut = 2,

    /// <summary>A reading is changing faster than its safe rate.</summary>
    ChangingFast = 3,
}

/// <summary>An alert a reading raised.</summary>
/// <remarks>Only the value belonging to <see cref="Kind"/> is set.</remarks>
/// <param name="Kind">Which threshold the reading crossed.</param>
/// <param name="Reading">The offending reading, for an out-of-range alert.</param>
/// <param name="Samples">The samples until empty, for a running-out alert.</param>
/// <param name="Rate">The change since the previous sample, for a changing-fast alert.</param>
public readonly record struct Alert(
    AlertKind Kind,
    float? Reading,
    uint? Samples,
    float? Rate);

/// <summary>What a controller decided about one reading.</summary>
/// <param name="Actuator">
/// The setting the output should take, or <c>null</c> when the profile observes
/// rather than controls.
/// </param>
/// <param name="Alert">The alert the reading raised, or <c>null</c> if it crossed nothing.</param>
public readonly record struct Reaction(bool? Actuator, Alert? Alert);

/// <summary>A profile's control policy.</summary>
/// <remarks>Only the values belonging to <see cref="Kind"/> are set.</remarks>
/// <param name="Kind">Which policy this describes.</param>
/// <param name="Setpoint">The target reading, for a setpoint policy.</param>
/// <param name="Hysteresis">Half the deadband width, for a setpoint policy.</param>
/// <param name="Cooling">Whether the output cools rather than heats.</param>
/// <param name="SafeBand">How far the reading may stray before an alert.</param>
/// <param name="Empty">The level treated as empty, for a level policy.</param>
/// <param name="WarnWithin">How many samples ahead to warn, for a level policy.</param>
/// <param name="Rising">Whether a rise rather than a fall is watched.</param>
/// <param name="Limit">The largest safe change per sample, for a surge policy.</param>
public readonly record struct ControlPolicy(
    ControlKind Kind,
    float? Setpoint,
    float? Hysteresis,
    bool? Cooling,
    float? SafeBand,
    float? Empty,
    uint? WarnWithin,
    bool? Rising,
    float? Limit);

/// <summary>How often a node samples as its battery drains, in whole seconds.</summary>
/// <param name="ActiveSecs">Seconds between samples at a healthy charge.</param>
/// <param name="SaverSecs">Seconds between samples while conserving.</param>
/// <param name="CriticalSecs">Seconds between samples when critically low.</param>
/// <param name="SaverBelow">Enter the saver cadence below this state of charge.</param>
/// <param name="CriticalBelow">Enter the critical cadence below this state of charge.</param>
public readonly record struct PowerSchedule(
    ulong ActiveSecs,
    ulong SaverSecs,
    ulong CriticalSecs,
    float SaverBelow,
    float CriticalBelow);

/// <summary>A named, ready-to-run node assembled from pamoja capabilities.</summary>
/// <remarks>
/// A profile is a pre-wired bundle: a control policy, a publish topic, and a
/// power schedule. Instantiate one rather than choosing algorithms and tuning
/// constants by hand. The presentation a dashboard reads travels inside the
/// manifest JSON, so <see cref="ToJson"/> carries the whole profile.
/// </remarks>
public sealed class Profile : IDisposable
{
    private readonly NativeHandle _handle;

    private Profile(IntPtr handle, string what) =>
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_profile_free, what);

    /// <summary>A cold-chain fridge monitor, which holds 5 C and flags an excursion.</summary>
    /// <returns>The profile.</returns>
    public static Profile VaccineFridgeMonitor() =>
        new(NativeMethods.pamoja_profile_vaccine_fridge_monitor(), "profile");

    /// <summary>An irrigation node, which opens a valve as soil moisture falls.</summary>
    /// <returns>The profile.</returns>
    public static Profile IrrigationNode() =>
        new(NativeMethods.pamoja_profile_irrigation_node(), "profile");

    /// <summary>A well-level monitor, which warns before a tank runs dry.</summary>
    /// <returns>The profile.</returns>
    public static Profile WellLevel() =>
        new(NativeMethods.pamoja_profile_well_level(), "profile");

    /// <summary>A flood sensor, which warns when a level rises too fast.</summary>
    /// <returns>The profile.</returns>
    public static Profile FloodSensor() =>
        new(NativeMethods.pamoja_profile_flood_sensor(), "profile");

    /// <summary>Loads a profile from its JSON manifest.</summary>
    /// <param name="manifest">The manifest.</param>
    /// <returns>The profile.</returns>
    /// <exception cref="PamojaException">The manifest is malformed.</exception>
    public static Profile FromJson(string manifest) =>
        new(NativeMethods.pamoja_profile_from_json(manifest), "profile");

    /// <summary>Gets the profile's stable, human-readable name.</summary>
    public string Name => _handle.Use(p => OwnedString.Read(NativeMethods.pamoja_profile_name(p)));

    /// <summary>Gets the topic each reading is published to.</summary>
    public string Topic => _handle.Use(p => OwnedString.Read(NativeMethods.pamoja_profile_topic(p)));

    /// <summary>Gets the control policy applied to each reading.</summary>
    public ControlPolicy Control => _handle.Use(p =>
    {
        PamojaStatus status = NativeMethods.pamoja_profile_control(p, out PamojaControlSpec spec);
        PamojaCore.ThrowIfError(status);
        return Policy(spec);
    });

    /// <summary>Gets the sampling schedule kept as the battery drains.</summary>
    public PowerSchedule Power => _handle.Use(p =>
    {
        PamojaStatus status = NativeMethods.pamoja_profile_power(p, out PamojaPowerSchedule s);
        PamojaCore.ThrowIfError(status);
        return new PowerSchedule(
            s.ActiveSecs,
            s.SaverSecs,
            s.CriticalSecs,
            s.SaverBelow,
            s.CriticalBelow);
    });

    /// <summary>Gets the schedule assembled into a power governor.</summary>
    public PowerPlan PowerPlan => _handle.Use(p =>
    {
        PamojaStatus status = NativeMethods.pamoja_profile_power_plan(p, out PamojaPowerPlan plan);
        PamojaCore.ThrowIfError(status);
        return new PowerPlan(
            plan.ActiveUs,
            plan.SaverUs,
            plan.CriticalUs,
            plan.SaverBelow,
            plan.CriticalBelow);
    });

    /// <summary>Serializes this profile to its JSON manifest.</summary>
    /// <returns>The manifest.</returns>
    /// <exception cref="PamojaException">The profile cannot be serialized.</exception>
    public string ToJson() =>
        _handle.Use(p => OwnedString.Read(NativeMethods.pamoja_profile_to_json(p)));

    /// <summary>Builds the decision logic this profile describes.</summary>
    /// <returns>The controller, which the caller disposes.</returns>
    public Controller Controller() =>
        _handle.Use(p => Core.Controller.FromNative(NativeMethods.pamoja_profile_controller(p)));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Flattens a native policy into the record a caller sees.</summary>
    /// <param name="spec">The native policy.</param>
    /// <returns>The policy.</returns>
    private static ControlPolicy Policy(PamojaControlSpec spec) => spec.Kind switch
    {
        PamojaControlKind.Setpoint => new ControlPolicy(
            ControlKind.Setpoint,
            spec.Setpoint,
            spec.Hysteresis,
            spec.Cooling != 0,
            spec.SafeBand,
            null,
            null,
            null,
            null),
        PamojaControlKind.Level => new ControlPolicy(
            ControlKind.Level,
            null,
            null,
            null,
            null,
            spec.Empty,
            spec.WarnWithin,
            null,
            null),
        PamojaControlKind.Surge => new ControlPolicy(
            ControlKind.Surge,
            null,
            null,
            null,
            null,
            null,
            null,
            spec.Rising != 0,
            spec.Limit),
        _ => new ControlPolicy(ControlKind.Monitor, null, null, null, null, null, null, null, null),
    };
}

/// <summary>The decision logic a profile assembles.</summary>
/// <remarks>
/// A controller carries state between readings, because a level estimate and a
/// rate of change both need the previous sample, so evaluate readings through one
/// controller in the order they were taken.
/// </remarks>
public sealed class Controller : IDisposable
{
    private readonly NativeHandle _handle;

    private Controller(IntPtr handle) =>
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_controller_free, "controller");

    /// <summary>Holds a reading near a setpoint by switching an output on and off.</summary>
    /// <param name="setpoint">The target reading.</param>
    /// <param name="hysteresis">Half the deadband width, which stops chattering.</param>
    /// <param name="cooling">Whether the output cools rather than heats.</param>
    /// <param name="safeBand">How far the reading may stray before an alert.</param>
    /// <returns>The controller.</returns>
    public static Controller Setpoint(
        float setpoint,
        float hysteresis,
        bool cooling,
        float safeBand) =>
        new(NativeMethods.pamoja_controller_setpoint(setpoint, hysteresis, cooling, safeBand));

    /// <summary>Warns before a falling level reaches empty.</summary>
    /// <param name="empty">The level treated as empty.</param>
    /// <param name="warnWithin">Warn once empty is this many samples away.</param>
    /// <returns>The controller.</returns>
    public static Controller Level(float empty, uint warnWithin) =>
        new(NativeMethods.pamoja_controller_level(empty, warnWithin));

    /// <summary>Warns when a reading changes faster than a limit.</summary>
    /// <param name="rising">Watch a rapid rise rather than a rapid fall.</param>
    /// <param name="limit">The largest safe change per sample.</param>
    /// <returns>The controller.</returns>
    public static Controller Surge(bool rising, float limit) =>
        new(NativeMethods.pamoja_controller_surge(rising, limit));

    /// <summary>Reports readings without judging them.</summary>
    /// <returns>The controller.</returns>
    public static Controller Monitor() => new(NativeMethods.pamoja_controller_monitor());

    /// <summary>Decides what one reading calls for.</summary>
    /// <param name="reading">The reading to evaluate.</param>
    /// <returns>The decision.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Reaction Evaluate(float reading) => _handle.Use(c =>
    {
        PamojaStatus status =
            NativeMethods.pamoja_controller_evaluate(c, reading, out PamojaReaction reaction);
        PamojaCore.ThrowIfError(status);

        Alert? alert = reaction.Alert switch
        {
            PamojaAlertKind.OutOfRange =>
                new Alert(AlertKind.OutOfRange, reaction.Reading, null, null),
            PamojaAlertKind.RunningOut =>
                new Alert(AlertKind.RunningOut, null, reaction.Samples, null),
            PamojaAlertKind.ChangingFast =>
                new Alert(AlertKind.ChangingFast, null, null, reaction.Rate),
            _ => null,
        };
        return new Reaction(reaction.HasActuator != 0 ? reaction.Actuator != 0 : null, alert);
    });

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Wraps a controller handle a native call produced.</summary>
    /// <param name="handle">The native handle.</param>
    /// <returns>The controller.</returns>
    internal static Controller FromNative(IntPtr handle) => new(handle);
}

/// <summary>Reads and releases the owned strings the C ABI produces.</summary>
internal static class OwnedString
{
    /// <summary>Copies an owned string out and releases it.</summary>
    /// <param name="text">The native string handle.</param>
    /// <returns>The string.</returns>
    /// <exception cref="PamojaException">The native call produced no string.</exception>
    public static string Read(IntPtr text)
    {
        string? read = ReadOrNull(text);
        return read ?? throw new PamojaException(
            PamojaCore.LastError() ?? "the call produced no string");
    }

    /// <summary>Copies an owned string out and releases it, allowing none.</summary>
    /// <param name="text">The native string handle, which may be null.</param>
    /// <returns>The string, or <c>null</c> when the call produced none.</returns>
    public static string? ReadOrNull(IntPtr text)
    {
        if (text == IntPtr.Zero)
        {
            return null;
        }

        try
        {
            return Marshal.PtrToStringUTF8(NativeMethods.pamoja_string_data(text));
        }
        finally
        {
            NativeMethods.pamoja_string_free(text);
        }
    }
}
