using Pamoja.Kit;

using static Guides.Guide;
using static System.FormattableString;

namespace Guides;

/// <summary>The helpers guide example; see docs/guides/kit.md.</summary>
public static class KitGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The tower's level transmitter reports on a 4-20 mA loop: 4 mA is empty and 20 mA
        // is full, so mid-scale is 12 mA, and a dead loop reads below empty rather than as
        // empty.
        Calibration level = Calibration.TwoPoint(4.0f, 0.0f, 20.0f, 100.0f);
        (float mid, float empty, float dead) = (level.Apply(12.0f), level.Apply(4.0f), level.Apply(0.0f));
        Console.WriteLine(Invariant($"level     12 mA reads {mid:F1}%, 4 mA reads {empty:F1}%, a dead loop {dead:F1}%"));

        // One dropout in five readings: the median of the five ignores it, the mean does
        // not.
        using var median = new Median(5);
        using var recent = new Window(5);
        float held = 0.0f;
        foreach (float milliamps in new[] { 12.0f, 12.0f, 0.0f, 12.0f, 12.0f })
        {
            held = median.Update(milliamps);
            recent.Push(milliamps);
        }

        float heldPercent = level.Apply(held);
        float meanPercent = level.Apply(recent.Mean()!.Value);
        Console.WriteLine(Invariant(
            $"level     through a dropout the median holds {heldPercent:F1}%, the mean falls to {meanPercent:F1}%"));

        // Water sloshing in the tower swings the reading. A smoother moves a quarter of
        // the way from its last value toward each new reading, so the swing mostly cancels
        // out.
        using var smoother = new Smoother(0.25f);
        using var swing = new Window(6);
        foreach (float percent in new[] { 50.0f, 53.0f, 48.0f, 52.0f, 49.0f, 51.0f })
        {
            smoother.Update(percent);
            swing.Push(percent);
        }

        Console.WriteLine(Invariant(
            $"level     sloshing readings from {swing.Min():F1}% to {swing.Max():F1}% smooth to {smoother.Value:F1}%"));

        // A Kalman filter is told how noisy the sensor is and how fast the level can really
        // move. When the pump starts and the level climbs from 50% to 60%, the one told the
        // level barely moves takes the climb for noise and lags; the one told it moves
        // keeps up.
        using var expectsSteady = new Kalman(0.01f, 2.0f, 50.0f);
        using var expectsMotion = new Kalman(0.5f, 2.0f, 50.0f);
        foreach (float percent in new[] { 50.0f, 50.0f, 50.0f, 60.0f, 60.0f, 60.0f, 60.0f })
        {
            expectsSteady.Update(percent);
            expectsMotion.Update(percent);
        }

        (float slow, float fast) = (expectsSteady.Estimate, expectsMotion.Estimate);
        Console.WriteLine(Invariant(
            $"level     four readings into a rise to 60%, a Kalman filter expecting a steady level reads {slow:F1}%, one expecting motion {fast:F1}%"));

        // An accelerometer on the tank watches the tower's lean. Standing still, only gravity
        // pulls on it, so the direction of the pull, in g, gives the tilt.
        Tilt atRest = Kit.TiltFromAccel(0.0, 0.007, 1.0);
        Console.WriteLine(Invariant($"tower     at rest the accelerometer reads a lean of {atRest.Roll:F2} degrees"));

        // In wind the tower sways, and the sway's own acceleration swings the accelerometer's
        // tilt. A gyro's rate of turn does not swing, but it drifts. A complementary filter
        // trusts the gyro from one tenth of a second to the next and the accelerometer over
        // time.
        using var lean = new Complementary(0.98f, (float)atRest.Roll);
        using var gusts = new Window(5);
        foreach ((float rate, float tilt) in new[] { (0.4f, 2.1f), (-0.6f, -1.3f), (0.5f, 1.8f), (-0.3f, -0.9f), (0.1f, 1.2f) })
        {
            lean.Update(rate, tilt, 0.1f);
            gusts.Push(tilt);
        }

        float steadyLean = lean.Estimate;
        Console.WriteLine(Invariant(
            $"tower     in wind the accelerometer swings from {gusts.Min():F1} to {gusts.Max():F1} degrees; fused with the gyro the lean reads {steadyLean:F1}"));

        // The refill pump starts at 40% and stops at 60%: on/off control with a band either
        // side of 50. Starting when the level falls is the direction Heating names.
        using var pump = Thermostat.Heating(50.0f, 10.0f);
        var states = new List<string>();
        foreach (float percent in new[] { 50.0f, 39.0f, 45.0f, 61.0f })
        {
            string running = pump.Update(percent) ? "on" : "off";
            states.Add(Invariant($"{percent:F0}% {running}"));
        }

        Console.WriteLine($"pump      {string.Join(", ", states)}");

        // The high-level float switch bounces as the water sloshes at the top. It has to
        // read full three times running before the pump controller believes it.
        using var floatSwitch = new Debounce(3, false);
        (int rawChanges, int settledChanges, bool lastRaw) = (0, 0, false);
        foreach (bool raw in new[] { true, false, true, true, true, false, true })
        {
            rawChanges += raw != lastRaw ? 1 : 0;
            lastRaw = raw;
            bool before = floatSwitch.State;
            settledChanges += floatSwitch.Update(raw) != before ? 1 : 0;
        }

        string full = floatSwitch.State ? "full" : "not full";
        Console.WriteLine($"float     {rawChanges} raw changes settled into {settledChanges}: the tower reads {full}");

        // The low-water alarm is sent once when the level drops under 20% and not again
        // until it has come back above 25%, however long it hovers near the line.
        using var lowWater = Trigger.Below(20.0f, 5.0f);
        foreach (float percent in new[] { 24.0f, 19.0f, 18.0f, 21.0f, 19.0f, 26.0f })
        {
            switch (lowWater.Update(percent))
            {
                case Edge.Set:
                    Console.WriteLine(Invariant($"alarm     low water at {percent:F0}%: alarm sent"));
                    break;
                case Edge.Cleared:
                    Console.WriteLine(Invariant($"alarm     back to {percent:F0}%: all clear sent"));
                    break;
            }
        }

        // A booster pump holds the mains at 3.0 bar. The PID asks for a speed, the ramp lets
        // the pump change by at most 25% a second so the pipes never take a water hammer,
        // and within 0.05 bar of 3.0 the reading counts as on target.
        using var pressureHold = Pid.WithLimits(40.0f, 8.0f, 0.0f, 0.0f, 100.0f);
        using var softStart = new Ramp(0.0f, 25.0f);
        foreach (float bar in new[] { 1.0f, 1.8f, 2.5f, 2.9f, 3.02f })
        {
            float steady = Kit.Deadband(bar, 3.0f, 0.05f);
            float asked = pressureHold.Update(3.0f, steady, 1.0f);
            float given = softStart.Update(asked);
            Console.WriteLine(Invariant(
                $"booster   at {bar:F2} bar the PID asks for {asked:F0}%, the pump is given {given:F0}%"));
        }

        // The booster's controller hangs in the pump house above the mains. The pump house
        // thermometer reads Fahrenheit, and a pipe colder than the air's dew point sweats.
        float air = Units.FahrenheitToCelsius(84.0f);
        double dew = Kit.DewPoint(air, 78.0);
        string sweats = 18.0 < dew ? "sweat" : "stay dry";
        Console.WriteLine(Invariant(
            $"pumphouse 84 F is {air:F1} C, and at 78% humidity it dews at {dew:F1} C, so the 18 C mains {sweats}"));

        // A power cut stops the borehole pump. From the hourly level, the countdown says how
        // long until the tower reaches its 20% reserve.
        using var reserve = new Depletion(20.0f);
        uint? hoursLeft = null;
        foreach (float percent in new[] { 80.0f, 76.0f, 72.0f })
        {
            hoursLeft = reserve.Update(percent);
        }

        Console.WriteLine($"outage    at the rate it is falling, the tower reaches 20% in {hoursLeft} hours");

        // With the outlet shut overnight the level should hold. A steady fall is a leak.
        using var overnight = new Trend(6);
        foreach (float percent in new[] { 78.0f, 77.6f, 77.1f, 76.7f, 76.2f, 75.8f })
        {
            overnight.Push(percent);
        }

        float slope = overnight.Slope!.Value;
        Console.WriteLine(Invariant($"leak      with the outlet shut the level falls {-slope:F2}% an hour"));

        // A burst main shows as pressure falling faster than any demand could pull it.
        using var burst = Surge.Falling(0.5f);
        foreach (float bar in new[] { 3.0f, 2.9f, 1.7f })
        {
            if (burst.Update(bar) is float fall)
            {
                Console.WriteLine(Invariant($"burst     the pressure fell {fall:F1} bar in one reading"));
            }
        }

        // The flow meter's readings set their own baseline. A hydrant opened stands out,
        // and so does a reading the meter could not make.
        using var flow = new Anomaly(3.0f, 8);
        using var normal = new Window(8);
        int flagged = 0;
        foreach (float cubicMeters in new[] { 12.1f, 11.8f, 12.4f, 12.0f, 11.9f, 12.2f, 12.0f, 12.3f })
        {
            flagged += flow.Check(cubicMeters) ? 1 : 0;
            normal.Push(cubicMeters);
        }

        Console.WriteLine(Invariant(
            $"meter     {normal.Count} readings from {normal.Min():F1} to {normal.Max():F1} m3/h, {flagged} flagged"));
        static string Verdict(bool standsOut) => standsOut ? "stands out" : "passes";
        bool hydrant = flow.Check(30.5f);
        bool failed = flow.Check(float.NaN);
        Console.WriteLine($"meter     a reading of 30.5 m3/h {Verdict(hydrant)}; a failed reading {Verdict(failed)}");

        // The tanker truck delivers inside a 20 km district around its depot.
        var depot = new Coordinate(-1.5177, 37.2634);
        var village = new Coordinate(-1.4480, 37.3390);
        double km = Kit.DistanceBetween(depot, village) / 1000.0;
        double bearing = Kit.BearingBetween(depot, village);
        Console.WriteLine(Invariant($"truck     the village is {km:F1} km from the depot, bearing {bearing:F0} degrees"));
        using var district = new Geofence(depot, 20_000.0);
        var roadOut = new Coordinate(-1.3000, 37.4500);
        var further = new Coordinate(-1.2500, 37.5000);
        string[] crossings = new[] { depot, village, roadOut, further, village }
            .Select(fix => district.Update(fix).ToString().ToLowerInvariant())
            .ToArray();
        Console.WriteLine($"truck     {string.Join(", ", crossings)}");
        // ANCHOR_END: example

        Expect((mid, empty, dead) == (50.0f, 0.0f, -25.0f), "the loop maps its live zero and span");
        Expect(heldPercent == 50.0f, "the median rode over the dropout");
        Expect(Math.Abs(meanPercent - 35.0f) < 1e-3f, "and the mean did not");
        Expect(states.SequenceEqual(["50% off", "39% on", "45% on", "61% off"]), "the pump keeps its band");
        Expect((rawChanges, settledChanges) == (5, 1) && floatSwitch.State, "the float settled once");
        Expect(hoursLeft == 13, "the reserve is 13 hours away");
        Expect(Math.Abs(slope + 0.4457f) < 1e-3f, "the leak shows as a steady fall");
        Expect(slow < 56.0f && fast > 58.0f, "the tuned filter keeps up");
        Expect(Math.Abs(atRest.Roll - 0.401) < 1e-3 && Math.Abs(steadyLean - 0.426f) < 1e-3f, "the lean holds through the gusts");
        Expect(dew > 24.0 && dew < 25.0, "the pump house air dews above the mains");
        Expect(hydrant && failed && flagged == 0, "only the odd readings stand out");
        Expect(crossings.SequenceEqual(["inside", "inside", "exited", "outside", "entered"]), "the truck crossed out and back");
    }
}
