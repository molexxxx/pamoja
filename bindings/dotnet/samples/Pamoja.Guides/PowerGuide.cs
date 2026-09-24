using Pamoja.Power;

using static Guides.Guide;
using static System.FormattableString;

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
            Console.WriteLine(Invariant(
                $"at {charge * 100:F0}% charge: {plan.Mode(charge)}, sampling every {every}s"));
        }

        // A panel that is delivering buys back one mode. The interval for a charge knows
        // nothing of the panel, so the cadence comes from the mode the panel bought.
        PowerMode charging = plan.ModeWhileCharging(0.12f, true);
        ulong chargingEvery = plan.IntervalForUs(charging) / 1_000_000;
        Console.WriteLine(
            $"at 12% charge while charging: {charging}, sampling every {chargingEvery}s");

        // A charge worked out from a fuel gauge that did not answer is not a number. The
        // plan takes it as critical, so a node that cannot tell what it has left does the
        // least until it can.
        float unknown = float.NaN;
        ulong unknownEvery = plan.IntervalUs(unknown) / 1_000_000;
        Console.WriteLine(
            $"with no reading from the gauge: {plan.Mode(unknown)}, sampling every {unknownEvery}s");

        // The thresholds say how long the battery must carry the node without sun. Winter
        // nights are long, so a winter plan starts saving sooner and goes critical sooner.
        PowerPlan winter = plan.WithThresholds(0.70f, 0.30f);
        Console.WriteLine(Invariant(
            $"the winter plan saves below {winter.SaverBelow * 100:F0}% and goes critical below {winter.CriticalBelow * 100:F0}%"));
        PowerMode cold = winter.Mode(0.60f);
        PowerMode mild = plan.Mode(0.60f);
        Console.WriteLine($"at 60% charge: {cold} in winter, {mild} by default");

        // A fuel gauge wanders a point or two between readings, so a charge sitting at a
        // threshold would change the cadence on every cycle. NextMode takes the mode the
        // node is in: it drops as soon as the charge falls below a threshold, and climbs
        // back only once the charge is the plan's hysteresis margin clear of it.
        float[] wandering = { 0.49f, 0.51f, 0.50f, 0.53f, 0.48f, 0.52f };
        string Walk(PowerPlan governor)
        {
            PowerMode mode = PowerMode.Active;
            var modes = new List<string>();
            foreach (float charge in wandering)
            {
                mode = governor.NextMode(mode, charge);
                modes.Add(mode.ToString());
            }
            return string.Join(", ", modes);
        }
        string flapping = Walk(plan.WithHysteresis(0f));
        Console.WriteLine($"a charge wandering around 50% with no margin: {flapping}");
        float margin = plan.Hysteresis * 100;
        string settled = Walk(plan);
        Console.WriteLine(Invariant($"and with the {margin:F0} point margin: {settled}"));
        PowerMode back = plan.NextMode(PowerMode.Saver, 0.56f);
        Console.WriteLine($"at 56% the charge has cleared the margin: {back}");

        // The work is the same two seconds whichever mode the node is in; stretching the
        // cycle is what saves the energy. The duty fraction is the proxy for average draw,
        // so the hourly cadence costs a sixtieth of what the one-minute cadence does.
        const ulong AwakeUs = 2_000_000;
        var healthy = new DutyCycle(AwakeUs, plan.IntervalUs(0.80f) - AwakeUs);
        var flat = new DutyCycle(AwakeUs, plan.IntervalUs(0.12f) - AwakeUs);
        Console.WriteLine(Invariant($"awake {healthy.Fraction * 100:F2}% of the time when healthy"));
        Console.WriteLine(Invariant($"awake {flat.Fraction * 100:F3}% of the time when flat"));

        // A node that lives on its panel can stay awake for the share of the time the
        // harvest pays for. Asleep it draws next to nothing, so that share is the harvest
        // over what it draws awake, and the duty cycle turns it into time.
        const ulong MinuteUs = 60_000_000;
        const float AwakeMw = 120f;
        DutyCycle cloudy = DutyCycle.FromFraction(MinuteUs, 12f / AwakeMw);
        Console.WriteLine($"a 12 mW harvest pays for {cloudy.ActiveUs / 1000}ms awake in each minute");

        // The share is clamped, so a harvest above the draw keeps the node awake
        // throughout, and a harvest the meter could not read keeps it asleep until one
        // can be.
        DutyCycle sunny = DutyCycle.FromFraction(MinuteUs, 150f / AwakeMw);
        DutyCycle unread = DutyCycle.FromFraction(MinuteUs, float.NaN);
        Console.WriteLine($"a 150 mW harvest keeps it awake all {sunny.ActiveUs / 1000}ms");
        Console.WriteLine($"an unread harvest keeps it asleep all {unread.SleepUs / 1000}ms");
        // ANCHOR_END: example

        Expect(plan.Mode(0.80f) == PowerMode.Active, "a healthy charge samples at full rate");
        Expect(plan.IntervalUs(0.80f) == 60_000_000, "which is every minute");
        Expect(plan.Mode(0.35f) == PowerMode.Saver, "half empty enters saver mode");
        Expect(plan.Mode(0.12f) == PowerMode.Critical, "and nearly flat is critical");
        Expect(charging == PowerMode.Saver, "a delivering panel buys back one mode");
        Expect(plan.IntervalForUs(charging) == 600_000_000, "and the cadence that goes with it");
        Expect(plan.Mode(unknown) == PowerMode.Critical, "an unread charge is taken as critical");
        Expect(plan.IntervalUs(unknown) == 3_600_000_000, "and sampled hourly");
        Expect(cold == PowerMode.Saver, "the winter plan saves at 60%");
        Expect(mild == PowerMode.Active, "where the default plan does not");
        Expect(flapping == "Saver, Active, Active, Active, Saver, Active", "no margin flaps");
        Expect(settled == "Saver, Saver, Saver, Saver, Saver, Saver", "the margin holds saver");
        Expect(back == PowerMode.Active, "until the charge clears it");
        Expect(Math.Abs(healthy.Fraction - 2.0f / 60.0f) < 1e-6, "two seconds in a minute");
        Expect(Math.Abs(flat.Fraction - 2.0f / 3600.0f) < 1e-6, "and two in an hour");
        Expect(cloudy.ActiveUs == 6_000_000, "a tenth of a minute is six seconds");
        Expect(cloudy.PeriodUs == MinuteUs, "and the period is still a minute");
        Expect(sunny.ActiveUs == MinuteUs && sunny.SleepUs == 0, "a surplus clamps to awake");
        Expect(unread.ActiveUs == 0 && unread.SleepUs == MinuteUs, "an unread harvest sleeps");
    }
}
