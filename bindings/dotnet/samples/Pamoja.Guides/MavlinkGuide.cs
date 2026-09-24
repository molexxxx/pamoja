using Pamoja;
using Pamoja.Mavlink;

using static System.FormattableString;
using static Guides.Guide;

namespace Guides;

/// <summary>The MAVLink guide example; see docs/guides/mavlink.md.</summary>
public static class MavlinkGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        const byte Vehicle = 1;
        const byte Autopilot = 1;
        const byte Station = 255;
        const byte Planner = 190;

        // An enum field travels as a number, and the dialect names each number. Printing
        // the name keeps a reader from looking up what 2 or 81 means.
        static string Name(string enumeration, double value) =>
            MavlinkEnum.Entry(enumeration, (ulong)value) ?? Invariant($"{value}");

        // Every node broadcasts a heartbeat to say what it is and that it is alive. The
        // frame wraps the payload in a header and a checksum seeded with the message's own
        // value.
        using MavlinkSchema heartbeatShape = MavlinkSchema.ForName("HEARTBEAT");
        using MavlinkMessage announce = heartbeatShape.CreateMessage();
        announce.Set("type", MavlinkEnum.Value("MAV_TYPE_GCS"));
        announce.Set("autopilot", MavlinkEnum.Value("MAV_AUTOPILOT_INVALID"));
        announce.Set("system_status", MavlinkEnum.Value("MAV_STATE_ACTIVE"));
        announce.Set("mavlink_version", 3);
        using MavlinkFrame sent = announce.ToFrame(new MavlinkHeader(Station, Planner, 0));
        Console.WriteLine($"sent      {heartbeatShape.Name} in {sent.Bytes.Length} bytes");

        // The vehicle answers with its own heartbeat, which reaches the station behind some
        // noise and a copy with its last byte flipped in flight.
        using MavlinkMessage vehicle = heartbeatShape.CreateMessage();
        vehicle.Set("type", MavlinkEnum.Value("MAV_TYPE_QUADROTOR"));
        vehicle.Set("autopilot", MavlinkEnum.Value("MAV_AUTOPILOT_ARDUPILOTMEGA"));
        vehicle.Set(
            "base_mode",
            MavlinkEnum.Value("MAV_MODE_FLAG_CUSTOM_MODE_ENABLED")
                | MavlinkEnum.Value("MAV_MODE_FLAG_STABILIZE_ENABLED")
                | MavlinkEnum.Value("MAV_MODE_FLAG_MANUAL_INPUT_ENABLED"));
        vehicle.Set("system_status", MavlinkEnum.Value("MAV_STATE_STANDBY"));
        vehicle.Set("mavlink_version", 3);
        using MavlinkFrame good = vehicle.ToFrame(new MavlinkHeader(Vehicle, Autopilot, 0));
        byte[] garbled = [.. good.Bytes];
        garbled[^1] ^= 0xFF;
        byte[] delivered = [.. "???"u8, .. garbled, .. good.Bytes];

        // The parser skips whatever does not start a frame and drops a frame whose checksum
        // fails, so only the good copy comes out.
        using MavlinkParser parser = new();
        IReadOnlyList<MavlinkFrame> frames = parser.Push(delivered);
        Console.WriteLine(
            $"parsed    {frames.Count} frame out of {delivered.Length} bytes,"
            + " past the noise and the garbled copy");
        using MavlinkMessage heard = heartbeatShape.Decode(frames[0].Payload);
        Console.WriteLine(
            $"heard     {Name("MAV_TYPE", heard.Get("type"))} on"
            + $" {Name("MAV_AUTOPILOT", heard.Get("autopilot"))},"
            + $" in {Name("MAV_STATE", heard.Get("system_status"))}");

        // The base mode is a bitmask, so it names a set of flags rather than one value.
        ulong baseMode = (ulong)heard.Get("base_mode");
        Console.WriteLine(
            $"flags     {string.Join(" | ", MavlinkEnum.Names("MAV_MODE_FLAG", baseMode))}");
        if ((baseMode & MavlinkEnum.Value("MAV_MODE_FLAG_SAFETY_ARMED")) == 0)
        {
            Console.WriteLine("disarmed  MAV_MODE_FLAG_SAFETY_ARMED is not among them");
        }
        // ANCHOR_END: example

        Expect(heard.Payload.SequenceEqual(vehicle.Payload), "the good copy decodes");
        Expect(frames.Count == 1, "and only it");
        Expect(frames[0].MessageId == heartbeatShape.MessageId, "as a heartbeat");
        foreach (MavlinkFrame frame in frames)
        {
            frame.Dispose();
        }

        // ANCHOR: command
        // The vehicle's answers, each naming the command it answers.
        using MavlinkSchema ackShape = MavlinkSchema.ForName("COMMAND_ACK");
        MavlinkFrame Answer(ulong command, ulong result, byte progress)
        {
            using MavlinkMessage ack = ackShape.CreateMessage();
            ack.Set("command", command);
            ack.Set("result", result);
            ack.Set("progress", progress);
            return ack.ToFrame(new MavlinkHeader(Vehicle, Autopilot, 0));
        }

        MavlinkAckOutcome? Hear(MavlinkCommand tracked, ulong command, ulong result, byte progress)
        {
            using MavlinkFrame frame = Answer(command, result, progress);
            return tracked.OnFrame(frame);
        }

        // A command is not fired and forgotten: the vehicle has to answer, and the sender
        // asks again until it does. Each resend carries the next confirmation number, which
        // is how the vehicle tells a retry from a second, deliberate command.
        using MavlinkCommand arming =
            new((ushort)MavlinkEnum.Value("MAV_CMD_COMPONENT_ARM_DISARM"), 3);
        using MavlinkSchema commandShape = MavlinkSchema.ForName("COMMAND_LONG");
        using MavlinkMessage arm = commandShape.CreateMessage();
        arm.Set("param1", 1.0);
        arm.Set("target_system", Vehicle);
        arm.Set("target_component", Autopilot);
        arm.Set("command", arming.Command);
        arm.Set("confirmation", arming.Confirmation);
        arm.ToFrame(new MavlinkHeader(Station, Planner, 1)).Dispose();
        Console.WriteLine(
            $"sent      {Name("MAV_CMD", arm.Get("command"))},"
            + $" confirmation {arm.Get("confirmation")}");
        byte? resend = arming.OnTimeout();
        if (resend is not null)
        {
            Console.WriteLine($"silence   resent with confirmation {resend}");
        }

        // An answer names the command it answers, so one for another command leaves this
        // one waiting.
        foreach ((ulong command, ulong result) in new[]
        {
            (MavlinkEnum.Value("MAV_CMD_NAV_TAKEOFF"), MavlinkEnum.Value("MAV_RESULT_ACCEPTED")),
            (MavlinkEnum.Value("MAV_CMD_COMPONENT_ARM_DISARM"), MavlinkEnum.Value("MAV_RESULT_ACCEPTED")),
        })
        {
            MavlinkAckOutcome? outcome = Hear(arming, command, result, 0);
            if (outcome?.Kind == MavlinkAckKind.Unrelated)
            {
                Console.WriteLine(
                    $"stray     an answer for {Name("MAV_CMD", command)} leaves it waiting");
            }
            else if (outcome?.Kind == MavlinkAckKind.Final)
            {
                Console.WriteLine($"armed     {Name("MAV_RESULT", outcome.Value.Value!.Value)}");
            }
        }

        // A long command reports progress before its final answer, and a refused one says
        // why.
        using MavlinkCommand calibrating =
            new((ushort)MavlinkEnum.Value("MAV_CMD_PREFLIGHT_CALIBRATION"), 3);
        MavlinkAckOutcome? progress =
            Hear(calibrating, calibrating.Command, MavlinkEnum.Value("MAV_RESULT_IN_PROGRESS"), 40);
        if (progress?.Kind == MavlinkAckKind.InProgress)
        {
            Console.WriteLine(
                $"progress  {Name("MAV_CMD", calibrating.Command)} is {progress.Value.Value}% done");
        }
        using MavlinkCommand changing = new((ushort)MavlinkEnum.Value("MAV_CMD_DO_SET_MODE"), 3);
        MavlinkAckOutcome? refusal =
            Hear(changing, changing.Command, MavlinkEnum.Value("MAV_RESULT_DENIED"), 0);
        if (refusal?.Kind == MavlinkAckKind.Final)
        {
            Console.WriteLine(
                $"refused   {Name("MAV_CMD", changing.Command)}"
                + $" answered {Name("MAV_RESULT", refusal.Value.Value!.Value)}");
        }

        // A command nobody answers runs out of retries, and the caller stops asking.
        using MavlinkCommand returning =
            new((ushort)MavlinkEnum.Value("MAV_CMD_NAV_RETURN_TO_LAUNCH"), 3);
        int sends = 1;
        while (returning.OnTimeout() is not null)
        {
            sends += 1;
        }
        Console.WriteLine(
            $"gave up   {Name("MAV_CMD", returning.Command)} went unanswered {sends} times");
        // ANCHOR_END: command

        Expect(arming.Confirmation == 1, "a timeout numbers the resend");
        Expect(sends == 4, "the first send and three retries");

        // ANCHOR: signing
        // Both ends share a secret key; replace these filler bytes with your own. The signer
        // stamps each frame with its link id and a timestamp that only moves forward.
        byte[] key = Enumerable.Repeat((byte)7, Mavlink.KeyLength).ToArray();
        using MavlinkSigner signer = new(key, 1, Mavlink.TimestampNow());
        using MavlinkFrame signed = signer.Sign(
            new MavlinkHeader(Station, Planner, 2),
            heartbeatShape.MessageId,
            sent.Payload,
            heartbeatShape.CrcExtra);
        Console.WriteLine(
            $"signed    {signed.Bytes.Length} bytes: the {sent.Bytes.Length} of the frame,"
            + $" then a {signed.Signature!.Length}-byte signature");

        // The vehicle checks each frame against the same key, and remembers the newest
        // timestamp from each sender, so a recording played back later is refused.
        static string? RefusedWith(Action check)
        {
            try
            {
                check();
                return null;
            }
            catch (PamojaException refusal)
            {
                return refusal.Message;
            }
        }

        using MavlinkVerifier verifier = new(key);
        if (RefusedWith(() => verifier.Verify(signed)) is null)
        {
            Console.WriteLine("accepted  the same key, and a timestamp it has not seen");
        }
        string? replayed = RefusedWith(() => verifier.Verify(signed));
        if (replayed is not null)
        {
            Console.WriteLine($"replayed  the same frame again is refused: {replayed}");
        }
        byte[] otherKey = Enumerable.Repeat((byte)9, Mavlink.KeyLength).ToArray();
        using MavlinkSigner stranger = new(otherKey, 1, Mavlink.TimestampNow());
        using MavlinkFrame forged = stranger.Sign(
            new MavlinkHeader(Station, Planner, 3),
            heartbeatShape.MessageId,
            sent.Payload,
            heartbeatShape.CrcExtra);
        string? forgery = RefusedWith(() => verifier.Verify(forged));
        if (forgery is not null)
        {
            Console.WriteLine($"forged    another key's frame is refused: {forgery}");
        }
        string? unsigned = RefusedWith(() => verifier.Verify(sent));
        if (unsigned is not null)
        {
            Console.WriteLine($"unsigned  a frame with no signature is refused: {unsigned}");
        }
        // ANCHOR_END: signing

        Expect(signed.Signed, "the signed frame carries its signature");
        Expect(!sent.Signed, "and the plain one does not");

        // ANCHOR: mission
        // A plan: take off to 20 m, fly to a point 50 m up, and return to launch. Positions
        // travel as degrees times ten million.
        (double latitude, double longitude) = (-33.85678, 151.2153);
        using MavlinkSchema itemShape = MavlinkSchema.ForName("MISSION_ITEM_INT");
        MavlinkMessage Item(string command, double x, double y, double z)
        {
            MavlinkMessage built = itemShape.CreateMessage();
            built.Set("command", MavlinkEnum.Value(command));
            built.Set("frame", MavlinkEnum.Value("MAV_FRAME_GLOBAL_RELATIVE_ALT_INT"));
            built.Set("x", x);
            built.Set("y", y);
            built.Set("z", z);
            built.Set("autocontinue", 1);
            return built;
        }

        // The station offers the plan, and the vehicle drives the transfer: it asks for each
        // item in turn and acknowledges the last one.
        MavlinkHeader station = new(Station, Planner, 0);
        MavlinkHeader aboard = new(Vehicle, Autopilot, 0);
        byte missionType = (byte)MavlinkEnum.Value("MAV_MISSION_TYPE_MISSION");
        using MavlinkMissionSender upload = new(Vehicle, Autopilot, missionType);
        using (MavlinkMessage takeoff = Item("MAV_CMD_NAV_TAKEOFF", 0, 0, 20))
        using (MavlinkMessage waypoint = Item(
            "MAV_CMD_NAV_WAYPOINT",
            Math.Round(latitude * 1e7),
            Math.Round(longitude * 1e7),
            50))
        using (MavlinkMessage home = Item("MAV_CMD_NAV_RETURN_TO_LAUNCH", 0, 0, 0))
        {
            upload.AddItem(takeoff);
            upload.AddItem(waypoint);
            upload.AddItem(home);
        }
        using MavlinkMissionReceiver vehicleSide = new(Station, Planner, missionType);
        MavlinkFrame toVehicle = upload.CountFrame(station);
        Console.WriteLine($"count     the station offers {upload.Count} items");
        byte? finished = null;
        while (finished is null)
        {
            MavlinkReceiverStep step = vehicleSide.OnFrame(toVehicle, aboard)!.Value;
            toVehicle.Dispose();
            if (step.Accepted is MavlinkMessage arrived)
            {
                Console.WriteLine(
                    $"arrived   item {arrived.Get("seq")}, {Name("MAV_CMD", arrived.Get("command"))}");
                arrived.Dispose();
            }
            if (step.Kind == MavlinkReceiverKind.Request)
            {
                Console.WriteLine($"request   the vehicle asks for item {vehicleSide.Expected}");
            }
            MavlinkSenderStep following = upload.OnFrame(step.Reply, station)!.Value;
            step.Reply.Dispose();
            if (following.Kind == MavlinkSenderKind.Finished)
            {
                finished = following.Result;
            }
            else
            {
                toVehicle = following.Reply!;
            }
        }
        Console.WriteLine($"done      the vehicle answered {Name("MAV_MISSION_RESULT", finished.Value)}");

        // A request past the end of the plan is answered with a refusal, not with an item.
        using MavlinkSchema requestShape = MavlinkSchema.ForName("MISSION_REQUEST_INT");
        using MavlinkMessage past = requestShape.CreateMessage();
        past.Set("seq", 7);
        past.Set("target_system", Station);
        past.Set("target_component", Planner);
        past.Set("mission_type", missionType);
        using MavlinkFrame asked = past.ToFrame(aboard);
        MavlinkSenderStep? reply = upload.OnFrame(asked, station);
        if (reply?.Kind == MavlinkSenderKind.Reply)
        {
            using MavlinkFrame answered = reply.Value.Reply!;
            using MavlinkSchema ackShapeForPlans = MavlinkSchema.ForName("MISSION_ACK");
            using MavlinkMessage refused = ackShapeForPlans.Decode(answered.Payload);
            Console.WriteLine(
                $"refused   a request for item {past.Get("seq")} is answered"
                + $" {Name("MAV_MISSION_RESULT", refused.Get("type"))}");
        }
        // ANCHOR_END: mission

        Expect(vehicleSide.Complete, "the vehicle holds the whole plan");
        Expect(
            finished == MavlinkEnum.Value("MAV_MISSION_ACCEPTED"),
            "and accepted it");
    }
}
