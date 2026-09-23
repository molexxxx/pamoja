# Pamoja.Actuators

A PCA9685 driver for servos, LEDs, and valves, and stepper drivers for four coil lines or a step and direction chip, in every language. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/actuators.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html)

## Install

```sh
dotnet add package Pamoja.Actuators
```

```csharp
using Pamoja.Actuators;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Gpio` and `Pamoja.Hal`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs):

```csharp
// Where the rig's parts connect: the PCA9685 answers at 0x40 with its six address pins
// low, the tilt servo is on its channel 0, and the status LED on channel 15.
const byte Address = Pca9685.DefaultAddress;
const byte Tilt = 0;
const byte Status = 15;
byte[] glow = Pwm.Duty(Pca9685.Counts / 16);

// The rig with nothing plugged in: a PCA9685 that powers up and keeps its datasheet's
// rules the way the part does. The bus keeps a copy of the part, so the program lets go
// of its own. On a Raspberry Pi the bus is I2cBus.Open("/dev/i2c-1") and nothing after
// this statement changes.
I2cBus bus;
using (I2cPart part = Pca9685.Sim.Part(Address))
{
    bus = I2cBus.Simulated(part);
}

using (bus)
{
    // A hobby servo wants a pulse every 20 ms, 50 Hz. The driver works the prescale out
    // from that and writes it with the oscillator asleep, since only then does the part
    // take it, then wakes the oscillator and waits the 500 us it needs to settle.
    using var controller = new Pca9685(bus, Address, frequencyHz: 50);
    controller.Init();
    Console.WriteLine(Invariant(
        $"controller   prescale {controller.Prescale} for {controller.Frequency:F1} Hz, awake after {bus.WaitedMicros} us"));

    // A servo turns to the width of the pulse it is sent, and this one points the camera
    // a little below level at 1300 us. The LED glows at a sixteenth of full brightness
    // while the rig waits. Reading the channels back shows what the part now holds.
    controller.SetChannel(Tilt, Pwm.Servo(1_300, 50));
    controller.SetChannel(Status, glow);
    var tilt = Pwm.Counts(controller.Channel(Tilt));
    var status = Pwm.Counts(controller.Channel(Status));
    Console.WriteLine($"tilt         1300 us pulse, low at count {tilt.Off} of {Pca9685.Counts}");
    Console.WriteLine($"status       glowing, high for {status.Off} of {Pca9685.Counts} counts");

    // The slider: a 1.8-degree motor, 200 steps a turn, behind an A4988 with MS1 to MS3
    // high, which splits each step into sixteen. A 20-tooth GT2 pulley pulls 40 mm of
    // belt a turn, so 5 mm between frames is an eighth of a turn.
    const uint SliderStepsPerTurn = 200 * 16;
    const float BeltMmPerTurn = 40.0f;
    int slide = Stepper.StepsForDegrees(360.0f * 5.0f / BeltMmPerTurn, SliderStepsPerTurn);
    var sliderDelay = new DelayLog();
    var slider = new StepDir<PinScript>(new PinScript(), new PinScript(), stepMicros: 500, delay: sliderDelay);

    // The pan head: a 28BYJ-48 through a ULN2003, half-stepped, 4096 half-steps a turn
    // through its gearbox. With a camera on it, it steps every 4 ms, half the 500 Hz its
    // pack is rated to start at with no load.
    const uint PanStepsPerTurn = 4096;
    int panStep = Stepper.StepsForDegrees(2.0f, PanStepsPerTurn);
    var coils = (new PinScript(), new PinScript(), new PinScript(), new PinScript());
    var panDelay = new DelayLog();
    using var pan = new FourWire<PinScript>(coils, StepDrive.HalfStep, stepMicros: 4_000, delay: panDelay);

    // Four frames. The LED lights for each exposure, and between frames the rig slides
    // and pans while it glows. The stepper lines record every level, and the delays count
    // every wait without sleeping through it.
    for (int frame = 1; frame <= 4; frame++)
    {
        controller.SetChannel(Status, Pwm.FullOn());
        float mm = slider.Position * BeltMmPerTurn / SliderStepsPerTurn;
        float degrees = pan.Position * 360.0f / PanStepsPerTurn;
        Console.WriteLine(Invariant($"frame {frame}      slider {mm:F1} mm, pan {degrees:F2} degrees"));
        controller.SetChannel(Status, glow);
        if (frame < 4)
        {
            slider.Steps(slide);
            pan.Steps(panStep);
        }
    }

    // A four-wire motor draws current for as long as its coils hold, so the pan head
    // drops them once the shoot is over.
    pan.Idle();
    var (stepLine, directionLine) = slider.Release();
    var (a, b, c, d) = pan.Release();
    int pulses = stepLine.Driven.Count(level => level == PinLevel.High);
    Console.WriteLine($"slider       {pulses} pulses on STEP, DIR {directionLine.Level}");
    Console.WriteLine($"pan head     {pan.Position} half-steps, coils {a.Level} {b.Level} {c.Level} {d.Level}");
    Console.WriteLine($"moving       slider {sliderDelay.TotalMillis} ms, pan head {panDelay.TotalMillis} ms");

    // The part takes a new prescale only while its oscillator sleeps. Written while it
    // runs, as a driver that skipped the sleep would write it, the value is dropped and
    // the servos stay at 50 Hz.
    byte fast = Pca9685.PrescaleForFrequency(1_000);
    bus.Write(Address, [Pca9685.Register.PreScale, fast]);
    byte stillHeld;
    using (I2cPart held = bus.Part<I2cPart>(Address)!)
    {
        stillHeld = held.Register(Pca9685.Register.PreScale);
    }

    Console.WriteLine($"prescale     written while awake, still {stillHeld}");

    // A channel the part does not have is refused before anything reaches the bus.
    try
    {
        controller.SetChannel(16, Pwm.FullOn());
        Console.WriteLine("channel 16   accepted, which should never happen");
    }
    catch (PamojaException error)
    {
        Console.WriteLine($"channel 16   {error.Message}");
    }

    // The shoot is over: every channel off in one transfer through the ALL_LED
    // registers, then the oscillator asleep. The part keeps its registers while it
    // sleeps.
    controller.SetAll(Pwm.FullOff());
    controller.Sleep();
    byte parkedMode;
    using (I2cPart parked = bus.Part<I2cPart>(Address)!)
    {
        parkedMode = parked.Register(Pca9685.Register.Mode1);
    }

    bool allOff = controller.Channel(Tilt).SequenceEqual(Pwm.FullOff())
        && controller.Channel(Status).SequenceEqual(Pwm.FullOff());
    bool asleep = (parkedMode & Pca9685.Mode1.Sleep) != 0;
    Console.WriteLine(
        $"parked       every channel {(allOff ? "off" : "still on")}, oscillator {(asleep ? "asleep" : "running")}");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-actuators`](https://crates.io/crates/pamoja-actuators) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html), [docs.rs](https://docs.rs/pamoja-actuators), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-actuators) |
| TypeScript | [`@pamoja/actuators`](https://www.npmjs.com/package/@pamoja/actuators) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-actuators) |
| Python | [`pamoja-actuators`](https://pypi.org/project/pamoja-actuators/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-actuators) |
| C# | [`Pamoja.Actuators`](https://www.nuget.org/packages/Pamoja.Actuators) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-actuators) |

## Documentation

- [`Pamoja.Actuators` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html), every type in this namespace.
- [The Actuator drivers guide](https://pamoja.molex.cloud/docs/guides/actuators.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
