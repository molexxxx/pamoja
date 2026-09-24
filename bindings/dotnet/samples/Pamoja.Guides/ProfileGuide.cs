using Pamoja;
using Pamoja.Power;
using Pamoja.Profile;

using static System.FormattableString;
using static Guides.Guide;

namespace Guides;

/// <summary>The device-profile guide example; see docs/guides/profile.md.</summary>
public static class ProfileGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A profile is plain data, so a fleet ships one as a file rather than as code.
        // This manifest names no battery thresholds, so the documented defaults apply.
        const string manifest = """
        {
            "name": "brooder-heater",
            "topic": "poultry/brooder/temperature",
            "control": {
                "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
                "cooling": false, "safe_band": 4.0
            },
            "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
        }
        """;
        using var profile = Profile.FromJson(manifest);
        Console.WriteLine($"profile   {profile.Name} reports on {profile.Topic}");
        Console.WriteLine(Invariant(
            $"defaults  the file names no battery thresholds, so saver starts below {profile.Power.SaverBelow * 100:F0}% and critical below {profile.Power.CriticalBelow * 100:F0}%"));

        // The schedule becomes a power plan, which says what mode a charge puts the node
        // in and how long it waits between samples there, in microseconds.
        PowerPlan plan = profile.PowerPlan;
        foreach (float charge in new[] { 0.8f, 0.3f, 0.1f })
        {
            Console.WriteLine(Invariant(
                $"battery   at {charge * 100:F0}% it runs {plan.Mode(charge)} and samples every {plan.IntervalUs(charge) / 1_000_000} s"));
        }

        // One controller runs for the life of the node, because it remembers whether the
        // lamp is on. The lamp switches on at 31.5 C or below and off at 32.5 C or above,
        // the setpoint less and plus the hysteresis, and in between it stays as it was. A
        // reading more than 4 C from the setpoint raises an alert as well.
        using Controller controller = profile.Controller();
        bool lamp = false;
        foreach (float reading in new[] { 27.5f, 31.8f, 32.6f, 32.1f, 31.4f })
        {
            Reaction reaction = controller.Evaluate(reading);
            bool on = reaction.Actuator == true;
            string change = on
                ? lamp ? "lamp stays on" : "lamp on"
                : lamp ? "lamp off" : "lamp stays off";
            string alert = reaction.Alert is { } raised ? $", alert {raised.Kind}" : "";
            string at = Invariant($"{reading} C");
            Console.WriteLine($"{at,-10}{change}{alert}");
            lamp = on;
        }

        // Written back out, the manifest names the thresholds the file left to their
        // defaults, so the next reader has nothing to infer, and it loads as the same
        // profile.
        string shared = profile.ToJson();
        using (var reloaded = Profile.FromJson(shared))
        {
            if (shared.Contains("saver_below") && reloaded.ToJson() == shared)
            {
                Console.WriteLine("shared    written back out, it names saver_below and loads as the same profile");
            }
        }

        // The manifest also carries how a dashboard draws the node: one element here, the
        // brooder's temperature on a thermometer with the band the chicks are safe in.
        using var drawn = profile.WithPresentation(new Presentation(
        [
            new ElementSpec("brooder_temperature", "celsius", "Brooder temperature", Viz.Thermometer)
            {
                Band = [28f, 36f],
            },
        ]));
        ElementSpec element = drawn.Presentation!.Elements[0];
        string graphic = element.Viz.ToString().ToLowerInvariant();
        Console.WriteLine(Invariant(
            $"draws     {element.Key} in {element.Unit} on a {graphic}, safe from {element.Band![0]} to {element.Band[1]}"));
        // ANCHOR_END: example

        Expect(lamp, "the morning ends with the lamp on");
        Expect(profile.Power.SaverBelow == 0.5f, "the saver threshold defaults to half");

        // ANCHOR: kinds
        // A level warns before a tank or a well runs dry. The shipped well profile counts
        // 0.5 m as dry and warns once the last fall puts dry six samples away or nearer.
        using (var wellLevel = Profile.WellLevel())
        using (Controller well = wellLevel.Controller())
        {
            foreach (float depth in new[] { 5.0f, 4.4f, 3.8f })
            {
                Console.WriteLine(well.Evaluate(depth).Alert is { Kind: AlertKind.RunningOut } alert
                    ? Invariant($"well      {depth} m: dry in {alert.Samples} samples at this rate, RunningOut")
                    : Invariant($"well      {depth} m: no warning yet"));
            }
        }

        // A surge warns when a reading moves too far in one sample. The shipped flood
        // sensor warns when a river rises more than 0.3 m between two readings.
        using (var floodSensor = Profile.FloodSensor())
        using (Controller river = floodSensor.Controller())
        {
            foreach (float gauge in new[] { 1.2f, 1.35f, 1.9f })
            {
                Console.WriteLine(river.Evaluate(gauge).Alert is { Kind: AlertKind.ChangingFast } alert
                    ? Invariant($"river     {gauge} m: up {alert.Rate:F2} m in one sample, ChangingFast")
                    : Invariant($"river     {gauge} m: no warning"));
            }
        }
        // ANCHOR_END: kinds

        // ANCHOR: wrong
        // A probe that fails reports a reading that is not a number. The controller raises
        // it rather than going quiet, and the lamp holds its state; what off means for the
        // chicks is the node's call.
        Reaction failed = controller.Evaluate(float.NaN);
        if (failed.Alert is { } invalid)
        {
            string holds = failed.Actuator == true ? "on" : "off";
            Console.WriteLine($"probe     a reading of NaN raises {invalid.Kind}, and the lamp holds {holds}");
        }

        // A controller built again for each reading forgets the lamp was on, so inside the
        // deadband it switches the lamp off.
        bool? first;
        bool? then;
        using (Controller once = profile.Controller())
        {
            first = once.Evaluate(27.5f).Actuator;
        }

        using (Controller again = profile.Controller())
        {
            then = again.Evaluate(31.8f).Actuator;
        }

        if (first == true && then == false)
        {
            Console.WriteLine("fresh     built again for each reading, the controller turns the lamp off at 31.8 C");
        }

        // A manifest no node could run is refused as it loads, with the reason. So is a
        // misspelled field, with the one it was probably meant to be, rather than leaving
        // the default in its place without a word.
        foreach (string edited in new[]
        {
            manifest.Replace("\"hysteresis\": 0.5", "\"hysteresis\": 0.0"),
            manifest.Replace("\"saver_secs\": 600", "\"saver_secs\": 60"),
            manifest.Replace("\"critical_secs\": 1800 }", "\"critical_secs\": 1800, \"saver_bellow\": 0.3 }"),
        })
        {
            try
            {
                using var accepted = Profile.FromJson(edited);
                Console.WriteLine("a manifest no node could run was accepted, which should never happen");
            }
            catch (PamojaException error)
            {
                Console.WriteLine($"refused   {error.Message}");
            }
        }

        // A kind the library does not ship loads with its parameters, but no built-in
        // controller decides it, so asking for one is refused rather than handing back a
        // node that would never switch the lamp.
        using var custom = Profile.FromJson(manifest.Replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""));
        try
        {
            using Controller inert = custom.Controller();
            Console.WriteLine("a custom kind ran without its policy, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"refused   {error.Message}");
        }
        // ANCHOR_END: wrong
    }
}
