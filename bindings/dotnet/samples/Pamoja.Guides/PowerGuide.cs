using Pamoja.Power;

using static Guides.Guide;

namespace Guides;

/// <summary>The power-budget guide example; see docs/guides/power.md.</summary>
public static class PowerGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A solar node samples every minute while the charge is healthy, stretches to ten
        // minutes to conserve, and to an hour once the battery is nearly flat. Durations
        // cross the binding as microseconds.
        PowerPlan plan = PowerPlan.Create(60_000_000, 600_000_000, 3_600_000_000);

        // The default thresholds enter saver mode below 50% charge and critical below 20%.
        foreach (float charge in new[] { 0.80f, 0.35f, 0.12f })
        {
            ulong every = plan.IntervalUs(charge) / 1_000_000;
            Console.WriteLine(
                $"at {charge * 100:F0}% charge: {plan.Mode(charge)}, sampling every {every}s");
        }

        // A panel that is delivering buys back one mode, so the same flat battery keeps
        // reporting on the ten-minute saver cadence while the sun is on it.
        PowerMode charging = plan.ModeWhileCharging(0.12f, true);
        Console.WriteLine($"the same flat battery, while charging: {charging}");

        // The work is the same two seconds whichever mode the node is in; stretching the
        // cycle is what saves the energy. The duty fraction is the proxy for average draw,
        // so the hourly cadence costs a sixtieth of what the one-minute cadence does.
        const ulong AwakeUs = 2_000_000;
        var healthy = new DutyCycle(AwakeUs, plan.IntervalUs(0.80f) - AwakeUs);
        var flat = new DutyCycle(AwakeUs, plan.IntervalUs(0.12f) - AwakeUs);
        Console.WriteLine($"awake {healthy.Fraction * 100:F2}% of the time when healthy");
        Console.WriteLine($"awake {flat.Fraction * 100:F3}% of the time when flat");

        // Stating the budget as a fraction instead gives the awake time directly.
        DutyCycle quarter = DutyCycle.FromFraction(1_000_000, 0.25f);
        Console.WriteLine($"a quarter-duty second is {quarter.ActiveUs / 1000}ms awake");
        // ANCHOR_END: example

        Expect(plan.Mode(0.80f) == PowerMode.Active, "a healthy charge samples at full rate");
        Expect(plan.IntervalUs(0.80f) == 60_000_000, "which is every minute");
        Expect(plan.Mode(0.35f) == PowerMode.Saver, "half empty enters saver mode");
        Expect(plan.Mode(0.12f) == PowerMode.Critical, "and nearly flat is critical");
        Expect(charging == PowerMode.Saver, "a delivering panel buys back one mode");
        Expect(Math.Abs(healthy.Fraction - 2.0f / 60.0f) < 1e-6, "two seconds in a minute");
        Expect(Math.Abs(flat.Fraction - 2.0f / 3600.0f) < 1e-6, "and two in an hour");
        Expect(quarter.ActiveUs == 250_000, "a quarter of a second is awake");
        Expect(quarter.SleepUs == 750_000, "and the rest is asleep");
    }
}
