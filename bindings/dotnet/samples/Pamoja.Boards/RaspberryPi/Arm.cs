// ANCHOR: example
using Pamoja.Actuators;
using Pamoja.Hal;
using Pamoja.Kit;

namespace Boards.RaspberryPi;

/// <summary>
/// The inspection rover's arm: a shoulder servo and an elbow servo on a PCA9685 board on the
/// header's I2C bus, reaching for the controls on an inverter cabinet's panel. Wire the PCA9685
/// board's SDA to GPIO2, SCL to GPIO3, VCC to 3V3, and GND to ground, with the shoulder servo on
/// channel 0, the elbow servo on channel 1, and V+ from a 5 V supply whose ground is joined to
/// the Pi's.
/// </summary>
public static class Arm
{
    // The PCA9685 channels the shoulder and elbow servos are plugged into.
    private const byte ShoulderChannel = 0;
    private const byte ElbowChannel = 1;

    // The panel's controls, each in meters out from the shoulder and up from it.
    private static readonly (string Name, float X, float Y)[] Panel =
    [
        ("reset button", 0.35f, 0.20f),
        ("breaker", 0.45f, 0.10f),
        ("door latch", 0.20f, 0.35f),
        ("fan switch", 0.70f, 0.00f),
    ];

    /// <summary>Reaches for each control on the panel, then parks the arm.</summary>
    public static void Run()
    {
        // The PCA9685 on the header's I2C bus, at 50 Hz for the servos.
        using I2cBus bus = I2cBus.Open("/dev/i2c-1");
        using var controller = new Pca9685(bus, Pca9685.DefaultAddress, frequencyHz: 50);

        // Links of 0.30 m and 0.25 m. The servos were fitted with both joints at 0, so a joint
        // angle of 0 is each servo's center, 90 degrees.
        var arm = new TwoLinkArm(0.30f, 0.25f);
        ServoMap servo = ServoMap.Standard;
        ushort Pulse(float joint) => servo.Pulse((float)(90.0 + joint * 180.0 / Math.PI));

        // Each control the arm can reach, it holds for two seconds; one it cannot, it skips
        // rather than drive a servo into its end stop.
        foreach ((string name, float x, float y) in Panel)
        {
            (float Shoulder, float Elbow)? joints = arm.JointsFor(x, y, Elbow.Up);
            if (joints is null)
            {
                Console.WriteLine($"{name,-12}  out of reach, skipped");
                continue;
            }

            (float shoulder, float elbow) = joints.Value;
            controller.SetChannel(ShoulderChannel, Pwm.Servo(Pulse(shoulder), 50));
            controller.SetChannel(ElbowChannel, Pwm.Servo(Pulse(elbow), 50));
            Console.WriteLine($"{name,-12}  shoulder {Pulse(shoulder)} us, elbow {Pulse(elbow)} us");
            Thread.Sleep(TimeSpan.FromSeconds(2));
        }

        // Back to the pose the arm was fitted in. The PCA9685 goes on sending both pulses after
        // the program exits, so the servos hold the arm there.
        controller.SetChannel(ShoulderChannel, Pwm.Servo(Pulse(0.0f), 50));
        controller.SetChannel(ElbowChannel, Pwm.Servo(Pulse(0.0f), 50));
        Console.WriteLine($"parked        both servos at {Pulse(0.0f)} us");
    }
}
// ANCHOR_END: example
