using Pamoja.Power;

using static Guides.Guide;

namespace Guides;

/// <summary>The power-scheduling guide example; see docs/guides/power.md.</summary>
public static class PowerGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A solar node samples every minute while the charge is healthy, stretches to
        // ten minutes to conserve, and to an hour once the battery is nearly flat.
        // Durations cross the binding as microseconds.
        PowerPlan plan = PowerPlan.Create(60_000_000, 600_000_000, 3_600_000_000);

        // The default thresholds enter saver mode below 50% charge and critical below 20%.
        Expect(plan.Mode(0.80f) == PowerMode.Active, "a healthy charge runs at full cadence");
        Expect(plan.IntervalUs(0.80f) == 60_000_000, "which is a reading every minute");
        Expect(plan.Mode(0.35f) == PowerMode.Saver, "a third of a charge conserves");
        Expect(plan.IntervalUs(0.35f) == 600_000_000, "at ten minutes between readings");
        Expect(plan.Mode(0.12f) == PowerMode.Critical, "and a nearly flat one survives");
        Expect(plan.IntervalUs(0.12f) == 3_600_000_000, "at one reading an hour");

        // A panel that is delivering buys back one mode, so the same flat battery keeps
        // reporting on the ten-minute saver cadence while the sun is on it.
        Expect(
            plan.ModeWhileCharging(0.12f, true) == PowerMode.Saver,
            "incoming charge eases the governor off by one mode");

        // The work is the same two seconds whichever mode the node is in; stretching the
        // cycle is what saves the energy. The duty fraction is the proxy for average
        // draw, so the hourly cadence costs a sixtieth of what the one-minute one does.
        const ulong awakeUs = 2_000_000;
        DutyCycle healthy = new(awakeUs, plan.IntervalUs(0.80f) - awakeUs);
        DutyCycle flat = new(awakeUs, plan.IntervalUs(0.12f) - awakeUs);
        Expect(Math.Abs(healthy.Fraction - (2.0f / 60.0f)) < 1e-6f, "one part in thirty awake");
        Expect(Math.Abs(flat.Fraction - (2.0f / 3600.0f)) < 1e-6f, "one part in 1800 awake");

        // Stating the budget as a fraction instead gives the awake time directly.
        DutyCycle quarter = DutyCycle.FromFraction(1_000_000, 0.25f);
        Expect(quarter.ActiveUs == 250_000, "a quarter of a second of work");
        Expect(quarter.SleepUs == 750_000, "and three quarters asleep");
        // ANCHOR_END: example
    }
}
