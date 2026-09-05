using Pamoja.Actuators;

using static Guides.Guide;

namespace Guides;

/// <summary>The actuator-driver guide example; see docs/guides/actuators.md.</summary>
public static class ActuatorsGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
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
            $"coils     {Convert.ToString(motor.Coils, 2).PadLeft(4, '0')} back at the start "
            + "of the cycle");
        Console.WriteLine($"a quarter turn is {Stepper.StepsForDegrees(90.0f, 200)} steps");
        // ANCHOR_END: example

        Expect(prescale == 0x79, "50 Hz is the prescale the datasheet gives");
        Expect(Pca9685.ChannelRegister(3) == 0x12, "channel 3 starts four registers along");
        Expect(Pwm.Counts(centred).Off == 307, "a centred servo goes low at count 307");
        Expect(flagged, "full off is its own encoding, not a zero duty");
        Expect(motor.Coils == 0b1000, "the cycle wraps back to where it started");
        Expect(Stepper.StepsForDegrees(90.0f, 200) == 50, "a quarter turn is fifty steps");
    }
}
