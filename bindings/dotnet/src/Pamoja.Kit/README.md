# Pamoja.Kit

Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
using var level = Calibration.TwoPoint(4.0f, 0.0f, 20.0f, 100.0f);
Expect(level.Apply(12.0f) == 50.0f, "mid-scale current is half of the span");
Expect(level.Apply(4.0f) == 0.0f, "the live zero reads empty");

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of
// the scale rather than an empty tank. A median window drops that sample outright,
// where an average would blend a quarter of the range into every reading after it.
Expect(level.Apply(0.0f) == -25.0f, "a dead loop reads below the scale");
using var filtered = new Median();
float[] loop = [12.0f, 12.0f, 0.0f, 12.0f, 12.0f];
float percent = 0.0f;
foreach (float milliamps in loop)
{
    percent = level.Apply(filtered.Update(milliamps));
    Expect(percent == 50.0f, "the dropout never reaches the pump");
}

// A refill pump runs when the level falls below the deadband, which is the
// direction Heating names; nothing about it is specific to temperature. The
// deadband stops a level sitting on the threshold from chattering the contactor.
using var pump = Thermostat.Heating(50.0f, 10.0f);
Expect(!pump.Update(percent), "a half-full tank leaves the pump off");
Expect(pump.Update(38.0f), "below the deadband the pump runs");
Expect(pump.Update(45.0f), "inside the deadband it holds its state");
Expect(!pump.Update(62.0f), "above the deadband it stops");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-kit`](https://crates.io/crates/pamoja-kit) | [docs.rs](https://docs.rs/pamoja-kit), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html) |
| TypeScript | [`@pamoja/kit`](https://www.npmjs.com/package/@pamoja/kit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html) |
| Python | [`pamoja-kit`](https://pypi.org/project/pamoja-kit/) | [`pamoja.kit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html) |
| C# | [`Pamoja.Kit`](https://www.nuget.org/packages/Pamoja.Kit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.Kit.html) |

## Documentation

- [The Helpers guide](https://pamoja.molex.cloud/docs/guides/kit.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
