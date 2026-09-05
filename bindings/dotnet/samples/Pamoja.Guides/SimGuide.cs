using Pamoja.Kit;
using Pamoja.Sim;

using static Guides.Guide;

namespace Guides;

/// <summary>The simulators guide example; see docs/guides/sim.md.</summary>
public static class SimGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the rover has run the capture.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
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
        // ANCHOR_END: example

        Expect(seen.SequenceEqual(capture), "the replay hands back the captured series");
        Expect(
            throttle.Commands.SequenceEqual([1.0f, 1.0f, 1.0f, 0.0f]),
            "and the actuator kept every command the loop issued");
        Expect(Math.Abs(pose.X - 1.5f) < 1e-6f, "1.5 m travelled along x");
        Expect(Math.Abs(pose.Y) < 1e-6f, "with no sideways drift");
        Expect(Math.Abs(pose.Theta - 0.5f) < 1e-6f, "and a half-radian turn at the end");
    }
}
