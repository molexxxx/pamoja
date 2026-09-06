# Pamoja.Kit

Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/kit.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Kit
```

```csharp
using Pamoja.Kit;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs):

```csharp
// A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA
// is full, so the span is 16 mA and mid-scale is 12 mA, not 10.
Calibration level = Calibration.TwoPoint(4.0f, 0.0f, 20.0f, 100.0f);
Console.WriteLine($"12 mA is {level.Apply(12.0f)}% full, 4 mA is {level.Apply(4.0f)}%");

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of
// the scale rather than an empty tank.
float broken = level.Apply(0.0f);
Console.WriteLine($"a dead loop reads {broken}%, which is not a level at all");

// A median window drops that sample outright, where an average would blend a
// quarter of the range into every reading after it.
using var filtered = new Median();
float percent = 0.0f;
foreach (float milliamps in new[] { 12.0f, 12.0f, 0.0f, 12.0f, 12.0f })
{
    percent = level.Apply(filtered.Update(milliamps));
}

Console.WriteLine($"through the dropout, the level held at {percent}%");

// A refill pump runs when the level falls below the deadband, which is the
// direction heating names; nothing about it is specific to temperature. The
// deadband stops a level sitting on the threshold from chattering the contactor.
using var pump = Thermostat.Heating(50.0f, 10.0f);
foreach (float reading in new[] { percent, 38.0f, 45.0f, 62.0f })
{
    Console.WriteLine($"at {reading}% the pump is {(pump.Update(reading) ? "on" : "off")}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-kit`](https://crates.io/crates/pamoja-kit) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html), [docs.rs](https://docs.rs/pamoja-kit), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-kit) |
| TypeScript | [`@pamoja/kit`](https://www.npmjs.com/package/@pamoja/kit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-kit) |
| Python | [`pamoja-kit`](https://pypi.org/project/pamoja-kit/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-kit) |
| C# | [`Pamoja.Kit`](https://www.nuget.org/packages/Pamoja.Kit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-kit) |

## Documentation

- [`Pamoja.Kit` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html), every type in this namespace.
- [The Helpers guide](https://pamoja.molex.cloud/docs/guides/kit.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
