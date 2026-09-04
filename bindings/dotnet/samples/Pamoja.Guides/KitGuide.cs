using Pamoja.Kit;

using static Guides.Guide;

namespace Guides;

/// <summary>The helper-math guide example; see docs/guides/kit.md.</summary>
public static class KitGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA
        // is full, so the span is 16 mA and mid-scale is 12 mA, not 10.
        using var level = Calibration.TwoPoint(4.0f, 0.0f, 20.0f, 100.0f);
        Expect(level.Apply(12.0f) == 50.0f, "mid-scale current is half of the span");
        Expect(level.Apply(4.0f) == 0.0f, "the live zero reads empty");

        // The live zero is what makes a broken loop detectable: 0 mA is off the bottom of
        // the scale rather than an empty tank. A median window drops that sample outright,
        // where an average would blend a quarter of the range into every reading after it.
        Expect(level.Apply(0.0f) == -25.0f, "a dead loop reads below the scale");
        using var filtered = new Median();
        float[] loop = [12.0f, 12.0f, 0.0f, 12.0f, 12.0f];
        float percent = 0.0f;
        foreach (float milliamps in loop)
        {
            percent = level.Apply(filtered.Update(milliamps));
            Expect(percent == 50.0f, "the dropout never reaches the pump");
        }

        // A refill pump runs when the level falls below the deadband, which is the
        // direction Heating names; nothing about it is specific to temperature. The
        // deadband stops a level sitting on the threshold from chattering the contactor.
        using var pump = Thermostat.Heating(50.0f, 10.0f);
        Expect(!pump.Update(percent), "a half-full tank leaves the pump off");
        Expect(pump.Update(38.0f), "below the deadband the pump runs");
        Expect(pump.Update(45.0f), "inside the deadband it holds its state");
        Expect(!pump.Update(62.0f), "above the deadband it stops");
        // ANCHOR_END: example
    }
}
