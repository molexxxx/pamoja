using Pamoja.Core.Interop;

namespace Pamoja.Core;

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
    /// <param name="periodUs">The whole period, in microseconds.</param>
    /// <param name="fraction">The share spent awake, clamped to 0 through 1.</param>
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
public readonly record struct PowerPlan(
    ulong ActiveUs,
    ulong SaverUs,
    ulong CriticalUs,
    float SaverBelow,
    float CriticalBelow)
{
    /// <summary>Creates a plan from its three work intervals, with the defaults.</summary>
    /// <remarks>
    /// The defaults enter <see cref="PowerMode.Saver"/> below 50% charge and
    /// <see cref="PowerMode.Critical"/> below 20%.
    /// </remarks>
    /// <param name="activeUs">The interval at a healthy charge, in microseconds.</param>
    /// <param name="saverUs">The longer interval used to conserve, in microseconds.</param>
    /// <param name="criticalUs">The longest interval, in microseconds.</param>
    /// <returns>The power plan.</returns>
    public static PowerPlan Create(ulong activeUs, ulong saverUs, ulong criticalUs)
    {
        PamojaPowerPlan plan =
            NativeMethods.pamoja_power_plan_new(activeUs, saverUs, criticalUs);
        return new PowerPlan(
            plan.ActiveUs,
            plan.SaverUs,
            plan.CriticalUs,
            plan.SaverBelow,
            plan.CriticalBelow);
    }

    /// <summary>Returns a plan with the state-of-charge thresholds moved.</summary>
    /// <param name="saverBelow">Enter saver mode below this charge.</param>
    /// <param name="criticalBelow">Enter critical mode below this charge.</param>
    /// <returns>The adjusted plan.</returns>
    public PowerPlan WithThresholds(float saverBelow, float criticalBelow)
    {
        PamojaPowerPlan plan = NativeMethods.pamoja_power_plan_with_thresholds(
            Native, saverBelow, criticalBelow);
        return new PowerPlan(
            plan.ActiveUs,
            plan.SaverUs,
            plan.CriticalUs,
            plan.SaverBelow,
            plan.CriticalBelow);
    }

    /// <summary>Returns the mode this plan calls for at a state of charge.</summary>
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
    public ulong IntervalForUs(PowerMode mode) =>
        NativeMethods.pamoja_power_plan_interval_for_us(Native, (PamojaPowerMode)mode);

    /// <summary>Returns the work interval at a state of charge, in microseconds.</summary>
    /// <param name="soc">The battery state of charge, from 0 through 1.</param>
    /// <returns>The interval in microseconds.</returns>
    public ulong IntervalUs(float soc) =>
        NativeMethods.pamoja_power_plan_interval_us(Native, soc);

    /// <summary>Gets the blittable form the C ABI takes.</summary>
    private PamojaPowerPlan Native => new()
    {
        ActiveUs = ActiveUs,
        SaverUs = SaverUs,
        CriticalUs = CriticalUs,
        SaverBelow = SaverBelow,
        CriticalBelow = CriticalBelow,
    };
}
