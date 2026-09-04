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
// The datasheet's worked example: 200 Hz off the 25 MHz internal oscillator is
// prescale 0x1E, the value the part powers up holding. A servo bank wants 50 Hz.
Expect(Pca9685.PrescaleForFrequency(200) == 0x1E, "the datasheet's worked example");
Expect(Pca9685.PrescaleForFrequency(50) == 0x79, "and the usual servo rate");

// Each channel owns four consecutive registers from 0x06, so channel 3 starts at
// 0x12 and its four bytes go out in one bus transaction.
Expect(Pca9685.ChannelRegister(3) == 0x12, "the fourth channel's register block");

// A centred hobby servo holds its output high for 1500 us, 7.5 % of the 20 ms
// period, which is 307 of the 4096 counts. The bytes are on-low, on-high,
// off-low, off-high.
Expect(
    Pwm.Servo(1500, 50).SequenceEqual(new byte[] { 0x00, 0x00, 0x33, 0x01 }),
    "a centred pulse is 307 counts of the period");

// Fully off carries its own bit rather than a zero duty, which still holds the
// output high for the first count of every period.
Expect(
    Pwm.FullOff().SequenceEqual(new byte[] { 0x00, 0x00, 0x00, 0x10 }),
    "fully off is its own encoding");
Expect(
    Pwm.Duty(0).SequenceEqual(new byte[] { 0x00, 0x00, 0x00, 0x00 }),
    "which a zero duty is not");

// Half-step drive interleaves the one-coil and two-coil patterns; the most
// significant of the four bits is the first coil.
using var motor = new Stepper(StepDrive.HalfStep);
Expect(motor.Coils == 0b1000, "the cycle opens on one coil");
Expect(motor.Step(StepDirection.Forward) == 0b1100, "then a pair of them");
Expect(motor.Step(StepDirection.Forward) == 0b0100, "then the next coil alone");

// The eight patterns of a cycle wrap, so the motor runs indefinitely either way,
// and an angle converts to whole steps: a quarter turn of a 1.8-degree motor is
// 50 of them.
for (int step = 2; step < Stepper.StepCount(StepDrive.HalfStep); step++)
{
    motor.Step(StepDirection.Forward);
}

Expect(motor.Coils == 0b1000, "a full cycle returns to its first pattern");
Expect(Stepper.StepsForDegrees(90.0f, 200) == 50, "a quarter turn is fifty steps");
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
