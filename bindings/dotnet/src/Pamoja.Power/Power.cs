using Pamoja.Native.Interop;

namespace Pamoja.Power;

/// <summary>What a node should be doing at the current state of charge.</summary>
public enum PowerMode
{
    /// <summary>Full duty, because the charge is healthy.</summary>
    Active = 0,

    /// <summary>Reduced duty, to conserve charge.</summary>
    Saver = 1,

    /// <summary>Minimum duty, to stay alive as long as possible.</summary>
    Critical = 2,
}

/// <summary>The split between the time a node works and the time it sleeps.</summary>
/// <param name="ActiveUs">How long the node stays awake each period, in microseconds.</param>
/// <param name="SleepUs">How long it sleeps each period, in microseconds.</param>
public readonly record struct DutyCycle(ulong ActiveUs, ulong SleepUs)
{
    /// <summary>Creates a duty cycle that spends a fraction of a period awake.</summary>
    /// <remarks>
    /// The time awake is rounded down to a whole microsecond and the rest of the period is
    /// spent asleep, so the two always add up to the period.
    /// </remarks>
    /// <param name="periodUs">The whole period, in microseconds.</param>
    /// <param name="fraction">
    /// The share spent awake, clamped to 0 through 1. A fraction that is not a number keeps
    /// the node asleep for the whole period.
    /// </param>
    /// <returns>The duty cycle.</returns>
    public static DutyCycle FromFraction(ulong periodUs, float fraction)
    {
        PamojaDutyCycle duty = NativeMethods.pamoja_duty_cycle_from_fraction(periodUs, fraction);
        return new DutyCycle(duty.ActiveUs, duty.SleepUs);
    }

    /// <summary>Gets the whole period, awake plus asleep, in microseconds.</summary>
    public ulong PeriodUs => NativeMethods.pamoja_duty_cycle_period_us(Native);

    /// <summary>Gets the share of the period spent awake, from 0 through 1.</summary>
    public float Fraction => NativeMethods.pamoja_duty_cycle_fraction(Native);

    /// <summary>Gets the blittable form the C ABI takes.</summary>
    private PamojaDutyCycle Native => new() { ActiveUs = ActiveUs, SleepUs = SleepUs };
}

/// <summary>The work intervals a node uses in each mode, and where they change.</summary>
/// <remarks>
/// A node on a battery and a panel has to decide how often to do anything at all.
/// A plan stretches the interval between work as the charge falls, so a node that
/// would otherwise go dark in a cloudy week keeps reporting, less often.
/// </remarks>
/// <param name="ActiveUs">The interval at a healthy charge, in microseconds.</param>
/// <param name="SaverUs">The longer interval used to conserve, in microseconds.</param>
/// <param name="CriticalUs">The longest interval, in microseconds.</param>
/// <param name="SaverBelow">Enter <see cref="PowerMode.Saver"/> below this charge.</param>
/// <param name="CriticalBelow">Enter <see cref="PowerMode.Critical"/> below this charge.</param>
/// <param name="Hysteresis">How far above a threshold the charge must climb before the
/// plan leaves the lower mode.</param>
public readonly record struct PowerPlan(
    ulong ActiveUs,
    ulong SaverUs,
    ulong CriticalUs,
    float SaverBelow,
    float CriticalBelow,
    float Hysteresis = PowerPlan.DefaultHysteresis)
{
    /// <summary>The margin a charge must climb past a threshold before a plan leaves the lower mode.</summary>
    public const float DefaultHysteresis = 0.05f;

    /// <summary>Creates a plan from its three work intervals, with the defaults.</summary>
    /// <remarks>
    /// The defaults enter <see cref="PowerMode.Saver"/> below 50% charge and
    /// <see cref="PowerMode.Critical"/> below 20%, and leave each lower mode once the charge
    /// is <see cref="DefaultHysteresis"/> above the threshold that brought it on.
    /// </remarks>
    /// <param name="activeUs">The interval at a healthy charge, in microseconds.</param>
    /// <param name="saverUs">The longer interval used to conserve, in microseconds.</param>
    /// <param name="criticalUs">The longest interval, in microseconds.</param>
    /// <returns>The power plan.</returns>
    public static PowerPlan Create(ulong activeUs, ulong saverUs, ulong criticalUs) =>
        FromNative(NativeMethods.pamoja_power_plan_new(activeUs, saverUs, criticalUs));

    /// <summary>Returns a plan with the state-of-charge thresholds moved.</summary>
    /// <param name="saverBelow">Enter saver mode below this charge.</param>
    /// <param name="criticalBelow">Enter critical mode below this charge.</param>
    /// <returns>The adjusted plan.</returns>
    public PowerPlan WithThresholds(float saverBelow, float criticalBelow) =>
        FromNative(NativeMethods.pamoja_power_plan_with_thresholds(
            Native, saverBelow, criticalBelow));

    /// <summary>Returns a plan with the hysteresis margin moved.</summary>
    /// <param name="margin">How far above a threshold the charge must climb before the plan
    /// leaves the lower mode; 0 switches at the thresholds themselves, and a margin below
    /// zero or not a number is taken as 0.</param>
    /// <returns>The adjusted plan.</returns>
    public PowerPlan WithHysteresis(float margin) =>
        FromNative(NativeMethods.pamoja_power_plan_with_hysteresis(Native, margin));

    /// <summary>Returns the mode a node in <paramref name="current"/> moves to at a new charge.</summary>
    /// <remarks>
    /// The node drops to a lower mode as soon as the charge falls below that mode's
    /// threshold, and climbs back only once the charge reaches the threshold plus
    /// <see cref="Hysteresis"/>, so a charge wandering around a threshold keeps the node
    /// where it is.
    /// </remarks>
    /// <param name="current">The mode the node is running in.</param>
    /// <param name="soc">The battery state of charge, from 0 through 1.</param>
    /// <returns>The mode the node should run in next.</returns>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="current"/> is not one of the <see cref="PowerMode"/> values.</exception>
    public PowerMode NextMode(PowerMode current, float soc) =>
        (PowerMode)NativeMethods.pamoja_power_plan_next_mode(
            Native, (PamojaPowerMode)NamedValue.Require(current, nameof(current)), soc);

    /// <summary>Returns the mode a node in <paramref name="current"/> moves to, eased one step toward full duty while charging.</summary>
    /// <param name="current">The mode the node is running in, as this returned it last time.</param>
    /// <param name="soc">The battery state of charge, from 0 through 1.</param>
    /// <param name="charging">Whether the node is taking charge.</param>
    /// <returns>The mode the node should run in next.</returns>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="current"/> is not one of the <see cref="PowerMode"/> values.</exception>
    public PowerMode NextModeWhileCharging(PowerMode current, float soc, bool charging) =>
        (PowerMode)NativeMethods.pamoja_power_plan_next_mode_while_charging(
            Native,
            (PamojaPowerMode)NamedValue.Require(current, nameof(current)),
            soc,
            charging ? (byte)1 : (byte)0);

    /// <summary>Returns the mode this plan calls for at a state of charge.</summary>
    /// <remarks>
    /// A charge that is not a number, such as a fuel gauge that failed to answer, is taken
    /// as critical.
    /// </remarks>
    /// <param name="soc">The battery state of charge, from 0 through 1.</param>
    /// <returns>The mode the node should run in.</returns>
    public PowerMode Mode(float soc) =>
        (PowerMode)NativeMethods.pamoja_power_plan_mode(Native, soc);

    /// <summary>Returns the mode, eased one step toward full duty while charging.</summary>
    /// <param name="soc">The battery state of charge, from 0 through 1.</param>
    /// <param name="charging">Whether the node is taking charge.</param>
    /// <returns>The mode the node should run in.</returns>
    public PowerMode ModeWhileCharging(float soc, bool charging) =>
        (PowerMode)NativeMethods.pamoja_power_plan_mode_while_charging(
            Native, soc, charging ? (byte)1 : (byte)0);

    /// <summary>Returns the work interval for a mode, in microseconds.</summary>
    /// <param name="mode">The mode to look up.</param>
    /// <returns>The interval in microseconds.</returns>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="mode"/> is not one of the <see cref="PowerMode"/> values.</exception>
    public ulong IntervalForUs(PowerMode mode) =>
        NativeMethods.pamoja_power_plan_interval_for_us(
            Native, (PamojaPowerMode)NamedValue.Require(mode, nameof(mode)));

    /// <summary>Returns the work interval at a state of charge, in microseconds.</summary>
    /// <param name="soc">The battery state of charge, from 0 through 1.</param>
    /// <returns>The interval in microseconds.</returns>
    public ulong IntervalUs(float soc) =>
        NativeMethods.pamoja_power_plan_interval_us(Native, soc);

    /// <summary>Rebuilds a plan from the blittable form the C ABI returns.</summary>
    /// <param name="plan">The plan as it crossed the boundary.</param>
    /// <returns>The plan.</returns>
    internal static PowerPlan FromNative(PamojaPowerPlan plan) => new(
        plan.ActiveUs,
        plan.SaverUs,
        plan.CriticalUs,
        plan.SaverBelow,
        plan.CriticalBelow,
        plan.Hysteresis);

    /// <summary>Gets the blittable form the C ABI takes.</summary>
    private PamojaPowerPlan Native => new()
    {
        ActiveUs = ActiveUs,
        SaverUs = SaverUs,
        CriticalUs = CriticalUs,
        SaverBelow = SaverBelow,
        CriticalBelow = CriticalBelow,
        Hysteresis = Hysteresis,
    };
}
