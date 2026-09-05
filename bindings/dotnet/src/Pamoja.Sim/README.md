# Pamoja.Sim

Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sim.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
// The clear distance ahead, in metres, taken from an earlier survey run. A replay
// hands it back one reading at a time, so the loop below sees the same input on
// every run: the same rover code, driven by a recording rather than a range finder.
float[] capture = [4.0f, 3.0f, 1.5f, 0.5f];
using var ahead = new Replay(capture);
using var throttle = new RecordingActuator();
using var rover = new SimulatedRobot(0.5f); // each command advances half a second

List<float> seen = [];
for (int step = 0; step < capture.Length; step++)
{
    float reading = await ahead.ReadAsync();
    seen.Add(reading);

    // Drive on while there is room ahead, otherwise stop and turn on the spot.
    bool clear = reading > 1.0f;
    float vx = clear ? 1.0f : 0.0f;
    float omega = clear ? 0.0f : 1.0f;
    await throttle.ApplyAsync(vx);
    await rover.ApplyAsync(new Twist(vx, Omega: omega));
    Console.WriteLine($"{reading} m ahead, so drive at {vx} and turn at {omega}");
}

// The recording actuator kept every command, which is how a test says what the
// control loop decided rather than only what it ended up doing.
Console.WriteLine($"commands  {string.Join(", ", throttle.Commands)}");

// Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on
// the spot at 1 rad/s for half a second, which moves the rover nowhere.
Pose pose = rover.Pose;
Console.WriteLine(
    $"pose      x {pose.X:F1} m, y {pose.Y:F1} m, heading {pose.Theta:F1} rad");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sim`](https://crates.io/crates/pamoja-sim) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html), [docs.rs](https://docs.rs/pamoja-sim) |
| TypeScript | [`@pamoja/sim`](https://www.npmjs.com/package/@pamoja/sim) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html) |
| Python | [`pamoja-sim`](https://pypi.org/project/pamoja-sim/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html) |
| C# | [`Pamoja.Sim`](https://www.nuget.org/packages/Pamoja.Sim) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html) |

## Documentation

- [`Pamoja.Sim` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html), every type in this namespace.
- [The Simulators guide](https://pamoja.molex.cloud/docs/guides/sim.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
