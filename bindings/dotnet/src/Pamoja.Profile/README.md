# Pamoja.Profile

Named, ready-to-run device profiles from plain data or a JSON manifest. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/profile.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html)

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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-profile`](https://crates.io/crates/pamoja-profile) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [docs.rs](https://docs.rs/pamoja-profile), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-profile) |
| TypeScript | [`@pamoja/profile`](https://www.npmjs.com/package/@pamoja/profile) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-profile) |
| Python | [`pamoja-profile`](https://pypi.org/project/pamoja-profile/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-profile) |
| C# | [`Pamoja.Profile`](https://www.nuget.org/packages/Pamoja.Profile) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-profile) |

## Documentation

- [`Pamoja.Profile` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), every type in this namespace.
- [The Device profiles guide](https://pamoja.molex.cloud/docs/guides/profile.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
