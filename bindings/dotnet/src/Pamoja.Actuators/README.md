# Pamoja.Actuators

PCA9685 PWM and servo pulses, and stepper coil sequencing. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/actuators.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Actuators
```

```csharp
using Pamoja.Actuators;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs):

```csharp
// A servo bank wants 50 Hz. The prescale register that produces it is derived
// from the part's 25 MHz internal oscillator, so a caller names the rate it wants
// rather than working the divider out.
byte prescale = Pca9685.PrescaleForFrequency(50);
Console.WriteLine(
    $"prescale  {prescale} gives {Pca9685.FrequencyForPrescale(prescale):F1} Hz");

// Each channel owns four consecutive registers, so a whole channel is written in
// one bus transaction rather than four.
Console.WriteLine($"channel 3 starts at register 0x{Pca9685.ChannelRegister(3):X2}");

// A centred hobby servo holds its output high for 1500 us of the 20 ms period.
// The part counts in 4096 steps per period, so that is where the pulse ends.
byte[] centred = Pwm.Servo(1500, 50);
Console.WriteLine($"centred servo goes low at count {Pwm.Counts(centred).Off} of 4096");

// Fully off carries its own flag rather than a zero duty, which would still hold
// the output high for the first count of every period.
bool flagged = Pwm.Counts(Pwm.FullOff()).Off != Pwm.Counts(Pwm.Duty(0)).Off;
Console.WriteLine($"full off flag set: {flagged}");

// A stepper is driven by walking a pattern of coil states. Half-step drive
// interleaves the one-coil and two-coil patterns, so it has twice as many.
using var motor = new Stepper(StepDrive.HalfStep);
Console.WriteLine($"coils     {Convert.ToString(motor.Coils, 2).PadLeft(4, '0')} at rest");
for (int step = 0; step < 2; step++)
{
    byte coils = motor.Step(StepDirection.Forward);
    Console.WriteLine(
        $"coils     {Convert.ToString(coils, 2).PadLeft(4, '0')} after a step");
}

// The patterns wrap, so the motor runs indefinitely either way, and an angle
// converts to whole steps: a quarter turn of a 1.8-degree motor is fifty of them.
for (int step = 2; step < Stepper.StepCount(StepDrive.HalfStep); step++)
{
    motor.Step(StepDirection.Forward);
}

Console.WriteLine(
    $"coils     {Convert.ToString(motor.Coils, 2).PadLeft(4, '0')} back at the start");
Console.WriteLine($"a quarter turn is {Stepper.StepsForDegrees(90.0f, 200)} steps");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-actuators`](https://crates.io/crates/pamoja-actuators) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html), [docs.rs](https://docs.rs/pamoja-actuators) |
| TypeScript | [`@pamoja/actuators`](https://www.npmjs.com/package/@pamoja/actuators) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html) |
| Python | [`pamoja-actuators`](https://pypi.org/project/pamoja-actuators/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html) |
| C# | [`Pamoja.Actuators`](https://www.nuget.org/packages/Pamoja.Actuators) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html) |

## Documentation

- [`Pamoja.Actuators` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html), every type in this namespace.
- [The Actuator drivers guide](https://pamoja.molex.cloud/docs/guides/actuators.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
