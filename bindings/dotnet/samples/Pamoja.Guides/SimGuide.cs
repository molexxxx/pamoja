using System.Globalization;

using Pamoja;
using Pamoja.Core;
using Pamoja.Kit;
using Pamoja.Loopback;
using Pamoja.Sim;

using static Guides.Guide;

namespace Guides;

/// <summary>The simulators guide example; see docs/guides/sim.md.</summary>
public static class SimGuide
{
    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes once the rover has run the row.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
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
        // ANCHOR_END: example

        Expect(drive.Commands.SequenceEqual([1.0f, 1.0f, 1.0f, 0.0f]), "the drive kept every command");
        Expect(Math.Abs(pose.X - 1.5f) < 1e-6f && Math.Abs(pose.Y) < 1e-6f, "1.5 m along x");
        Expect(Math.Abs(pose.Theta - 0.5f) < 1e-6f, "a half-radian turn on the spot");
        Expect(again.SequenceEqual(moisture), "the same seed gave the same readings");
        Expect(delivered == 3, "the radio lost the third report");
    }

    /// <summary>Writes a value to one decimal place, whatever the machine's culture.</summary>
    private static string Tenths(float value) => value.ToString("F1", CultureInfo.InvariantCulture);
}
