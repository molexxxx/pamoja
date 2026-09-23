// ANCHOR: example
using System.Globalization;

using Pamoja.Actuators;
using Pamoja.Gpio;
using Pamoja.Hal;

namespace Boards.RaspberryPi;

/// <summary>
/// A pan and tilt head for a time-lapse: a tilt servo and a status LED on a PCA9685 board on the
/// header's I2C bus, and a 28BYJ-48 pan motor on four GPIO lines through a ULN2003 board. Wire the
/// PCA9685 board's SDA to GPIO2, SCL to GPIO3, VCC to 3V3, and GND to ground, with the servo on
/// channel 0, an LED on channel 15, and V+ from a 5 V supply whose ground is joined to the Pi's.
/// Wire the ULN2003 board's IN1 to IN4 to GPIO5, GPIO6, GPIO13, and GPIO26.
/// </summary>
public static class Rig
{
    // The GPIO chip the header's lines live on, numbered as the BCM numbers.
    private const string Chip = "/dev/gpiochip0";

    // The PCA9685 channels the tilt servo and the LED are plugged into.
    private const byte Tilt = 0;
    private const byte Status = 15;

    // The shoot: twelve frames across a 90-degree pan, one every five seconds.
    private const int Frames = 12;
    private const float SweepDegrees = 90.0f;

    // The lines the ULN2003 board's IN1 to IN4 are wired to, coils A to D.
    private static readonly uint[] PanLines = [5, 6, 13, 26];

    private static readonly TimeSpan Interval = TimeSpan.FromSeconds(5);

    /// <summary>Runs the shoot, then parks the head.</summary>
    public static void Run()
    {
        // The PCA9685 on the header's I2C bus, at 50 Hz for the servo. The first channel
        // written runs the datasheet's start-up: sleep, prescale, wake, 500 us, restart.
        using I2cBus bus = I2cBus.Open("/dev/i2c-1");
        using var controller = new Pca9685(bus, Pca9685.DefaultAddress, frequencyHz: 50);
        byte[] glow = Pwm.Duty(Pca9685.Counts / 16);
        controller.SetChannel(Tilt, Pwm.Servo(1_300, 50));
        controller.SetChannel(Status, glow);

        // Each coil line is taken low, so the motor holds nothing until its first step. The
        // 28BYJ-48 turns 4096 half-steps a turn through its gearbox, and with a camera on it,
        // it steps every 4 ms.
        using GpioLine a = GpioLine.OpenOutput(Chip, PanLines[0], PinLevel.Low);
        using GpioLine b = GpioLine.OpenOutput(Chip, PanLines[1], PinLevel.Low);
        using GpioLine c = GpioLine.OpenOutput(Chip, PanLines[2], PinLevel.Low);
        using GpioLine d = GpioLine.OpenOutput(Chip, PanLines[3], PinLevel.Low);
        using var pan = new FourWire<GpioLine>((a, b, c, d), StepDrive.HalfStep, stepMicros: 4_000);
        int perFrame = Stepper.StepsForDegrees(SweepDegrees / (Frames - 1), 4096);

        // The LED lights while the head holds still for the camera, and glows while it moves.
        for (int frame = 1; frame <= Frames; frame++)
        {
            controller.SetChannel(Status, Pwm.FullOn());
            Console.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"frame {frame,2}  pan {pan.Position * 360.0f / 4096,6:F2} degrees"));
            Thread.Sleep(Interval);
            controller.SetChannel(Status, glow);
            if (frame < Frames)
            {
                pan.Steps(perFrame);
            }
        }

        // Back to the start, then everything off: the coils dropped, every channel off in one
        // transfer, and the oscillator asleep.
        pan.Steps(-pan.Position);
        pan.Idle();
        controller.SetAll(Pwm.FullOff());
        controller.Sleep();
        Console.WriteLine($"parked at {pan.Position} half-steps");
    }
}
// ANCHOR_END: example
