# Pamoja.Sim

Noisy and replay sensors, a recording actuator, a simulated robot that dead-reckons its pose, and a link that loses sends on a pattern. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sim.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html)

## Install

```sh
dotnet add package Pamoja.Sim
```

```csharp
using Pamoja.Sim;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Codec`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs):

```csharp
// The clear distance ahead, in meters, replayed from an earlier survey of the
// row, so every run sees the same row.
using var ahead = new Replay([4.0f, 3.0f, 1.5f, 0.5f]);
// The drive keeps the commands it is given instead of turning a motor.
using var drive = new RecordingActuator();
// The rover's pose comes from integrating each command over half a second.
const float Dt = 0.5f;
using var rover = new SimulatedRobot(Dt);
// The soil probe reads around 31 percent, drying half a point a reading, with a
// wobble drawn from a seed, so the same seed gives the same readings every run.
using var soil = new SimulatedSensor(31.0f, -0.5f, 0.3f, 7);
// The radio loses every third report on its way to the base.
using var station = new LoopbackBroker();
using Transport radio = Transport.Degraded(station.Rung(), dropEvery: 3);
await radio.ConnectAsync();

List<float> moisture = [];
int delivered = 0;
while (true)
{
    // A replay that has handed back every reading reports that it is closed.
    float clear;
    try
    {
        clear = await ahead.ReadAsync();
    }
    catch (PamojaException error)
    {
        Console.WriteLine($"ahead     ran out after {moisture.Count} readings: {error.Message}");
        break;
    }

    (float speed, float turn) = clear > 1.0f ? (1.0f, 0.0f) : (0.0f, 1.0f);
    await drive.ApplyAsync(speed);
    await rover.ApplyAsync(new Twist(speed, Omega: turn));
    float wet = await soil.ReadAsync();
    float elapsed = moisture.Count * Dt;
    moisture.Add(wet);
    string report = Tenths(wet);
    try
    {
        await radio.SendAsync("vineyard/row-4/soil", report);
        delivered++;
    }
    catch (PamojaException)
    {
    }

    Console.WriteLine(
        $"{Tenths(elapsed)} s     {Tenths(clear)} m clear:"
        + $" drive {Tenths(speed)}, turn {Tenths(turn)}, soil {report}");
}

// The drive kept every command, which is how a test says what the loop decided
// rather than only what it ended up doing.
Console.WriteLine($"drive     recorded {string.Join(", ", drive.Commands.Select(Tenths))}");

// Three half-second commands at 1 m/s reach 1.5 m along x. The last turns on
// the spot at 1 rad/s for half a second, which moves the rover nowhere.
Pose pose = rover.Pose;
Console.WriteLine(
    $"rover     ended at x {Tenths(pose.X)} m, y {Tenths(pose.Y)} m, heading {Tenths(pose.Theta)} rad");

// A second probe with the same seed reads exactly the same values.
using var twin = new SimulatedSensor(31.0f, -0.5f, 0.3f, 7);
List<float> again = [];
foreach (float _ in moisture)
{
    again.Add(await twin.ReadAsync());
}

string verdict = again.SequenceEqual(moisture) ? "the same" : "different";
Console.WriteLine($"soil      a probe with the same seed read {verdict} {again.Count} values");
Console.WriteLine($"radio     delivered {delivered} of {moisture.Count} soil reports and lost the third");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sim`](https://crates.io/crates/pamoja-sim) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html), [docs.rs](https://docs.rs/pamoja-sim), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sim) |
| TypeScript | [`@pamoja/sim`](https://www.npmjs.com/package/@pamoja/sim) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sim) |
| Python | [`pamoja-sim`](https://pypi.org/project/pamoja-sim/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sim) |
| C# | [`Pamoja.Sim`](https://www.nuget.org/packages/Pamoja.Sim) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sim) |

## Documentation

- [`Pamoja.Sim` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html), every type in this namespace.
- [The Simulators guide](https://pamoja.molex.cloud/docs/guides/sim.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
