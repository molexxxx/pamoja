using Pamoja.Kit;

using static Guides.Guide;

namespace Guides;

/// <summary>The helpers guide example; see docs/guides/kit.md.</summary>
public static class KitGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA
        // is full, so the span is 16 mA and mid-scale is 12 mA, not 10.
        Calibration level = Calibration.TwoPoint(4.0f, 0.0f, 20.0f, 100.0f);
        Console.WriteLine($"12 mA is {level.Apply(12.0f)}% full, 4 mA is {level.Apply(4.0f)}%");

        // The live zero is what makes a broken loop detectable: 0 mA is off the bottom of
        // the scale rather than an empty tank.
        float broken = level.Apply(0.0f);
        Console.WriteLine($"a dead loop reads {broken}%, which is not a level at all");

        // A median window drops that sample outright, where an average would blend a
        // quarter of the range into every reading after it.
        using var filtered = new Median();
        float percent = 0.0f;
        foreach (float milliamps in new[] { 12.0f, 12.0f, 0.0f, 12.0f, 12.0f })
        {
            percent = level.Apply(filtered.Update(milliamps));
        }

        Console.WriteLine($"through the dropout, the level held at {percent}%");

        // A refill pump runs when the level falls below the deadband, which is the
        // direction heating names; nothing about it is specific to temperature. The
        // deadband stops a level sitting on the threshold from chattering the contactor.
        using var pump = Thermostat.Heating(50.0f, 10.0f);
        foreach (float reading in new[] { percent, 38.0f, 45.0f, 62.0f })
        {
            Console.WriteLine($"at {reading}% the pump is {(pump.Update(reading) ? "on" : "off")}");
        }
        // ANCHOR_END: example

        Expect(level.Apply(12.0f) == 50.0f, "mid-scale is half full");
        Expect(level.Apply(4.0f) == 0.0f, "and the live zero is empty");
        Expect(broken == -25.0f, "a dead loop reads off the bottom of the scale");
        Expect(percent == 50.0f, "the median window rode over the dropout");

        using var again = Thermostat.Heating(50.0f, 10.0f);
        Expect(!again.Update(50.0f), "at the setpoint the pump is off");
        Expect(again.Update(38.0f), "below the deadband it runs");
        Expect(again.Update(45.0f), "and keeps running inside it");
        Expect(!again.Update(62.0f), "until the level is back above");
    }
}
