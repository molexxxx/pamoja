using Pamoja.Profile;

using static Guides.Guide;

namespace Guides;

/// <summary>The device-profile guide example; see docs/guides/profile.md.</summary>
public static class ProfileGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A profile is plain data, so a fleet ships one as a file rather than as code. The
        // two power thresholds are optional and fall back to the documented defaults.
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
        Console.WriteLine($"{profile.Name} reports on {profile.Topic}");
        Console.WriteLine(
            $"wakes every {profile.Power.ActiveSecs}s while the battery is healthy");
        Console.WriteLine($"saver mode below {profile.Power.SaverBelow * 100:F0}% charge");

        // The manifest is the whole control loop. At 27.5 C the reading is below the
        // deadband, so the lamp switches on, and it is more than 4 C from target, so the
        // chicks are cold.
        Reaction cold = profile.Controller().Evaluate(27.5f);
        Console.WriteLine($"at 27.5 C: lamp {cold.Actuator}, alert {cold.Alert?.Kind}");

        // Back inside the deadband the lamp is left as it was, and nothing is raised.
        Reaction settled = profile.Controller().Evaluate(32.2f);
        Console.WriteLine($"at 32.2 C: lamp {settled.Actuator}, alert {settled.Alert?.Kind}");

        // Serializing writes the defaulted fields out in full, so a profile edited on a
        // device and shared back carries no value the next reader has to infer.
        string shared = profile.ToJson();
        Console.WriteLine($"shared form names its defaults: {shared.Contains("saver_below")}");
        // ANCHOR_END: example

        Expect(profile.Control.Kind == ControlKind.Setpoint, "the control policy is a setpoint");
        Expect(profile.Control.Setpoint == 32.0f, "held at 32 C");
        Expect(profile.Control.Cooling == false, "by heating rather than cooling");
        Expect(profile.Power.ActiveSecs == 120, "it samples every two minutes at charge");
        Expect(profile.Power.SaverBelow == 0.5f, "and the default saver threshold applies");
        Expect(cold.Actuator == true, "below the deadband the lamp comes on");
        Expect(cold.Alert?.Kind == AlertKind.OutOfRange, "and the drift is reported");
        Expect(settled.Alert is null, "inside it nothing is raised");
        Expect(shared.Contains("saver_below"), "the shared form names its defaults");
    }
}
