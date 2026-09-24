# Pamoja.Power

Duty cycling and an energy-aware governor that stretches work as the battery drains and holds its mode against a wandering charge. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/power.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html)

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
