# Pamoja.Power

Duty cycling and an energy-aware governor that stretches work as the battery drains. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/power.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Power
```

```csharp
using Pamoja.Power;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/PowerGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/PowerGuide.cs):

```csharp
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-power`](https://crates.io/crates/pamoja-power) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html), [docs.rs](https://docs.rs/pamoja-power), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-power) |
| TypeScript | [`@pamoja/power`](https://www.npmjs.com/package/@pamoja/power) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-power) |
| Python | [`pamoja-power`](https://pypi.org/project/pamoja-power/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-power) |
| C# | [`Pamoja.Power`](https://www.nuget.org/packages/Pamoja.Power) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-power) |

## Documentation

- [`Pamoja.Power` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html), every type in this namespace.
- [The Power guide](https://pamoja.molex.cloud/docs/guides/power.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
