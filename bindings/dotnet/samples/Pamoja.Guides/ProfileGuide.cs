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
        Expect(profile.Name == "brooder-heater", "the manifest names the profile");
        Expect(profile.Topic == "poultry/brooder/temperature", "and the topic it publishes on");
        Expect(profile.Control.Kind == ControlKind.Setpoint, "the control policy is a setpoint");
        Expect(profile.Control.Setpoint == 32.0f, "held at 32 C");
        Expect(profile.Control.Cooling == false, "by heating rather than cooling");
        Expect(profile.Power.ActiveSecs == 120, "and it samples every two minutes at charge");
        Expect(profile.Power.SaverBelow == 0.5f, "with the default saver threshold filled in");

        // The manifest is the whole control loop. At 27.5 C the reading is below the
        // deadband, so the lamp switches on, and it is more than 4 C from target, so the
        // chicks are cold.
        using var controller = profile.Controller();
        var reaction = controller.Evaluate(27.5f);
        Expect(reaction.Actuator == true, "the lamp is switched on");
        Expect(reaction.Alert?.Kind == AlertKind.OutOfRange, "and the drift is reported");
        Expect(reaction.Alert?.Reading == 27.5f, "carrying the reading that raised it");

        // Serializing writes the defaulted fields out in full, so a profile edited on a
        // device and shared back carries no value the next reader has to infer.
        var shared = profile.ToJson();
        Expect(shared.Contains("\"saver_below\"", StringComparison.Ordinal), "defaults are written out");
        using var reloaded = Profile.FromJson(shared);
        Expect(reloaded.Control.Setpoint == 32.0f, "and it reloads to the same profile");
        // ANCHOR_END: example
    }
}
