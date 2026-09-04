# Pamoja.Profile

Named, ready-to-run device profiles from plain data or a JSON manifest. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/profile.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Profile
```

```csharp
using Pamoja.Profile;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Power`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs):

```csharp
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-profile`](https://crates.io/crates/pamoja-profile) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [docs.rs](https://docs.rs/pamoja-profile) |
| TypeScript | [`@pamoja/profile`](https://www.npmjs.com/package/@pamoja/profile) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html) |
| Python | [`pamoja-profile`](https://pypi.org/project/pamoja-profile/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html) |
| C# | [`Pamoja.Profile`](https://www.nuget.org/packages/Pamoja.Profile) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html) |

## Documentation

- [`Pamoja.Profile` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), every type in this namespace.
- [The Device profiles guide](https://pamoja.molex.cloud/docs/guides/profile.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
