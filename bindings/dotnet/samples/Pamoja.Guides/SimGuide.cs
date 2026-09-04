using Pamoja.Sim;

using static Guides.Guide;

namespace Guides;

/// <summary>The simulators guide example; see docs/guides/sim.md.</summary>
public static class SimGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the example has run.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        // The clear distance ahead, in metres, taken from an earlier survey run. A replay
        // hands it back one reading at a time, so the loop below sees the same input on
        // every run.
        float[] capture = [4.0f, 3.0f, 1.5f, 0.5f];
        using var ahead = new Replay(capture);
        using var throttle = new RecordingActuator();
        using var rover = new SimulatedRobot(0.5f); // each command advances half a second

        var seen = new List<float>();
        foreach (var _ in capture)
        {
            var reading = await ahead.ReadAsync();
            seen.Add(reading);

            // Drive on while there is room ahead, otherwise stop and turn on the spot.
            var clear = reading > 1.0f;
            var vx = clear ? 1.0f : 0.0f;
            var omega = clear ? 0.0f : 1.0f;
            await throttle.ApplyAsync(vx);
            await rover.ApplyAsync(new Twist(vx, Omega: omega));
        }

        Expect(seen.SequenceEqual(capture), "the replay hands back the captured series");
        Expect(
            throttle.Commands.SequenceEqual([1.0f, 1.0f, 1.0f, 0.0f]),
            "and the actuator recorded what it was told to do");

        // Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on
        // the spot at 1 rad/s for half a second, which moves the rover nowhere.
        var pose = rover.Pose;
        Expect(Math.Abs(pose.X - 1.5f) < 1e-6f, "1.5 m travelled along x");
        Expect(Math.Abs(pose.Y) < 1e-6f, "with no sideways drift");
        Expect(Math.Abs(pose.Theta - 0.5f) < 1e-6f, "and half a radian turned in place");
        // ANCHOR_END: example
    }
}
