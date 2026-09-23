using System.Globalization;
using Pamoja.Kit;

using static Guides.Guide;
using static System.FormattableString;

namespace Guides;

/// <summary>The robot motion guide example; see docs/guides/motion.md.</summary>
public static class MotionGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        static double Degrees(float radians) => radians * 180.0 / Math.PI;

        // The same turn, 0.5 m/s forward while turning left at 0.4 rad/s, on four chassis.
        (float left, float right) = new DiffDrive(0.5f).WheelSpeeds(0.5f, 0.4f);
        Console.WriteLine(Invariant($"chassis   two wheels 0.5 m apart: left {left:F2} m/s, right {right:F2} m/s"));
        (left, right) = new SkidSteer(0.5f, 1.3f).WheelSpeeds(0.5f, 0.4f);
        Console.WriteLine(Invariant($"chassis   tracks that slip 1.3 times: left {left:F2} m/s, right {right:F2} m/s"));
        var car = new Ackermann(0.8f);
        float steering = car.SteeringAngle(0.5f, 0.4f);
        float radius = car.TurnRadius(steering);
        Console.WriteLine(Invariant(
            $"chassis   car-like, 0.8 m wheelbase: steer {Degrees(steering):F1} degrees, a {radius:F2} m turn radius"));
        var mecanum = new Mecanum(0.4f, 0.3f);
        static string Four(WheelSpeeds w) =>
            string.Join(", ", new[] { w.FrontLeft, w.FrontRight, w.RearLeft, w.RearRight }
                .Select(speed => speed.ToString("F2", CultureInfo.InvariantCulture)));
        WheelSpeeds turn = mecanum.WheelSpeeds(new Twist(0.5f, 0.0f, 0.4f));
        Console.WriteLine($"chassis   mecanum, front and rear, left and right: {Four(turn)} m/s");
        WheelSpeeds strafe = mecanum.WheelSpeeds(new Twist(0.0f, 0.3f, 0.0f));
        Console.WriteLine($"chassis   mecanum strafing left, which no other chassis can: {Four(strafe)} m/s");

        // Each wheel's encoder gives two channels a quarter step apart; the order they
        // change in tells the direction. These levels step forward four times, then back
        // once.
        using var encoder = new Quadrature();
        int[] steps = new[] { (false, true), (true, true), (true, false), (false, false), (true, false) }
            .Select(level => encoder.Update(level.Item1, level.Item2))
            .ToArray();
        Console.WriteLine($"encoder   each change counts {string.Join(", ", steps)}: {encoder.Count} steps forward in all");

        // Between GPS fixes the rover knows where it is from how far each wheel rolled. Each
        // encoder makes 360 steps a turn of a 0.1 m wheel; every half second the right wheel
        // counts a few more steps than the left, so the rover curves left.
        var wheel = new QuadratureScale(360.0f, 0.1f);
        var drive = new DiffDrive(0.5f);
        using var odometry = new Odometry();
        foreach ((long leftSteps, long rightSteps) in new[] { (430L, 430L), (425L, 455L), (425L, 455L), (430L, 430L) })
        {
            odometry.IntegrateWheels(wheel.Distance(leftSteps), wheel.Distance(rightSteps), drive);
        }

        Pose pose = odometry.Pose;
        Console.WriteLine(Invariant(
            $"odometry  after two seconds: {pose.X:F2} m ahead, {pose.Y:F2} m left, heading {Degrees(pose.Theta):F1} degrees"));

        // A positive yaw rate turns left and a negative one right, as ROS has it.
        static string Turning(float omega) =>
            Invariant($"turning {(omega < 0.0f ? "right" : "left")} at {Math.Abs(omega):F2} rad/s");

        // The inverter cabinet is about 41 m east. Facing north, the rover pivots toward it before
        // it drives; nearly facing it, it drives and trims its heading; close enough, it
        // stops.
        var follower = new WaypointFollower(0.8f, 2.0, 1.5f, 0.8f);
        var here = new Coordinate(-23.5610, 133.8700);
        var cabinet = new Coordinate(-23.5610, 133.8704);
        foreach (float heading in new[] { 0.0f, 80.0f })
        {
            Guidance guidance = follower.Guide(here, heading, cabinet);
            Console.WriteLine(Invariant(
                $"waypoint  heading {heading:F0}, {guidance.DistanceM:F0} m to go: forward {guidance.Twist.Vx:F2} m/s, {Turning(guidance.Twist.Omega)}"));
        }

        Guidance atCabinet = follower.Guide(new Coordinate(-23.5610, 133.87039), 90.0f, cabinet);
        string arrived = atCabinet.Arrived ? "arrived" : "still driving";
        Console.WriteLine(Invariant($"waypoint  {atCabinet.DistanceM:F1} m from the cabinet: {arrived}"));

        // Every command passes through the safety gate: held to 1 m/s, eased on at 0.5
        // m/s^2, and stopped if no fresh command arrives for 0.3 s.
        using var limits = new Limits(1.0f, 1.0f, 0.5f, 2.0f);
        using var gate = new SafetyGate(limits, 0.3f);
        var ahead = new Twist(0.8f);
        var eased = new List<string>();
        for (int i = 0; i < 3; i++)
        {
            gate.Feed();
            eased.Add(Invariant($"{gate.Command(ahead, 0.1f).Vx:F2}"));
        }

        Console.WriteLine($"safety    asked for 0.80 m/s from rest, allowed {string.Join(", ", eased)} m/s");
        string[] silence = Enumerable.Range(0, 3).Select(_ => Invariant($"{gate.Command(ahead, 0.1f).Vx:F2}")).ToArray();
        Console.WriteLine($"safety    then no fresh command: {string.Join(", ", silence)} m/s, stopped once more than 0.3 s pass");
        var veering = new Twist(0.8f, 0.0f, 0.2f);
        Twist near = Kit.ObstacleStop(veering, 0.35f, 0.5f);
        Console.WriteLine(Invariant($"safety    something 0.35 m ahead: forward {near.Vx:F2} m/s, still {Turning(near.Omega)}"));
        Twist blind = Kit.ObstacleStop(veering, float.NaN, 0.5f);
        Console.WriteLine(Invariant($"safety    no range reading at all: forward {blind.Vx:F2} m/s, still {Turning(blind.Omega)}"));
        gate.Feed();
        gate.EngageEstop();
        Twist stopped = gate.Command(ahead, 0.1f);
        Console.WriteLine(Invariant($"safety    e-stop engaged: {stopped.Vx:F2} m/s until a person resets it"));

        // At the cabinet, a two-link arm presses the reset button 0.35 m out and 0.20 m up.
        var arm = new TwoLinkArm(0.30f, 0.25f);
        (float reachMin, float reachMax) = arm.Reach;
        Console.WriteLine(Invariant($"arm       links 0.30 m and 0.25 m reach from {reachMin:F2} m to {reachMax:F2} m"));
        (float shoulder, float elbow) = arm.JointsFor(0.35f, 0.20f, Elbow.Up)
            ?? throw new InvalidOperationException("the button is in reach");
        Console.WriteLine(Invariant(
            $"arm       elbow up: shoulder {Degrees(shoulder):F1} degrees, elbow {Degrees(elbow):F1} degrees"));
        Transform tool = Kit.ForwardKinematics(new DhParameters(A: 0.30f, Theta: shoulder), new DhParameters(A: 0.25f, Theta: elbow));
        (float x, float y, _) = tool.Position;
        Console.WriteLine(Invariant($"arm       forward kinematics puts the tip at {x:F2} m, {y:F2} m"));
        string tooFar = arm.JointsFor(0.70f, 0.0f, Elbow.Up) is null ? "out of reach" : "in reach";
        Console.WriteLine($"arm       a button 0.70 m out: {tooFar}");

        // Hobby servos turn the joints. A joint angle of 0 is the servo's center, 90 degrees.
        ServoMap servo = ServoMap.Standard;
        ushort shoulderPulse = servo.Pulse((float)(90.0 + Degrees(shoulder)));
        ushort elbowPulse = servo.Pulse((float)(90.0 + Degrees(elbow)));
        Console.WriteLine($"servos    shoulder {shoulderPulse} us, elbow {elbowPulse} us");
        Esc esc = Esc.Bidirectional;
        Console.WriteLine($"motors    a quarter throttle forward is {esc.Pulse(0.25f)} us, a quarter back {esc.Pulse(-0.25f)} us");
        // ANCHOR_END: example

        Expect(steps.SequenceEqual([1, 1, 1, 1, -1]) && encoder.Count == 3, "four steps forward and one back");
        Expect(atCabinet.Arrived, "the rover arrived at the cabinet");
        Expect(silence.SequenceEqual(["0.20", "0.25", "0.00"]), "the watchdog stopped the rover");
        Expect(stopped.Vx == 0.0f && near.Vx == 0.0f && blind.Vx == 0.0f, "every stop is a stop");
        Expect(Math.Abs(x - 0.35f) < 1e-4f && Math.Abs(y - 0.20f) < 1e-4f, "the tip is on the button");
    }
}
