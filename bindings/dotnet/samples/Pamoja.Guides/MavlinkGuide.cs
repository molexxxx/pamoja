using Pamoja.Mavlink;

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

        // The values the MAVLink common dialect gives these fields.
        const byte MavTypeGcs = 6;
        const byte MavTypeQuadrotor = 2;
        const byte MavAutopilotInvalid = 8;
        const byte MavAutopilotArdupilotmega = 3;
        const byte MavStateActive = 4;
        const byte MavStateStandby = 3;
        const ushort MavCmdComponentArmDisarm = 400;
        const ushort MavCmdNavTakeoff = 22;
        const byte MavResultAccepted = 0;

        // Every MAVLink node broadcasts a heartbeat to say what it is and that it is
        // alive. The fields are set by name rather than by writing the payload out byte
        // by byte.
        using MavlinkSchema heartbeatShape = MavlinkSchema.ForName("HEARTBEAT");
        using MavlinkMessage announce = heartbeatShape.CreateMessage();
        announce.Set("type", MavTypeGcs);
        announce.Set("autopilot", MavAutopilotInvalid);
        announce.Set("system_status", MavStateActive);
        announce.Set("mavlink_version", 3);
        using MavlinkFrame sent = announce.ToFrame(new MavlinkHeader(Station, 190, 0));
        Console.WriteLine($"sent      HEARTBEAT in {sent.Bytes.Length} bytes");

        // The vehicle answers with its own heartbeat. This copy arrives after some bytes
        // that were already on the wire, and after a copy with one bit flipped in flight.
        using MavlinkMessage vehicle = heartbeatShape.CreateMessage();
        vehicle.Set("type", MavTypeQuadrotor);
        vehicle.Set("autopilot", MavAutopilotArdupilotmega);
        vehicle.Set("system_status", MavStateStandby);
        vehicle.Set("mavlink_version", 3);
        using MavlinkFrame good = vehicle.ToFrame(new MavlinkHeader(Vehicle, Autopilot, 0));
        byte[] garbled = [.. good.Bytes];
        garbled[^1] ^= 0xFF;
        byte[] delivered = [.. "???"u8, .. garbled, .. good.Bytes];

        // The parser skips whatever does not start a frame and drops one whose checksum
        // fails, so the frame it hands back is the good copy rather than the garbled one.
        using MavlinkParser parser = new();
        using MavlinkFrame received = parser.Push(delivered)[0];
        using MavlinkMessage heard = heartbeatShape.Decode(received.Payload);
        Console.WriteLine(
            $"heard     a type-{heard.Get("type")} vehicle in state {heard.Get("system_status")}");

        // Arming it is a command, not a message a sender fires and forgets: the vehicle
        // has to answer, and the sender keeps asking until it does. The protocol numbers
        // each resend, which is how a vehicle tells a retry from a deliberate second one.
        using MavlinkCommand arming = new(MavCmdComponentArmDisarm, 3);
        using MavlinkSchema commandShape = MavlinkSchema.ForName("COMMAND_LONG");
        using MavlinkMessage arm = commandShape.CreateMessage();
        arm.Set("param1", 1.0); // 1 arms, 0 disarms
        arm.Set("target_system", Vehicle);
        arm.Set("target_component", Autopilot);
        arm.Set("command", arming.Command);
        arm.Set("confirmation", arming.Confirmation);
        Console.WriteLine($"sent      arm request, confirmation {arming.Confirmation}");
        arm.ToFrame(new MavlinkHeader(Station, 190, 1)).Dispose();

        // Nothing comes back in time, so it goes again with the next confirmation number.
        byte? resend = arming.OnTimeout();
        Console.WriteLine($"silence, resending with confirmation {resend}");

        // An acknowledgement names the command it answers, so one for a different command
        // is not this exchange finishing.
        using MavlinkSchema ackShape = MavlinkSchema.ForName("COMMAND_ACK");
        MavlinkAckOutcome? stray = Acknowledge(ackShape, arming, MavCmdNavTakeoff);
        Console.WriteLine($"an ack for another command: {stray?.Kind}");

        MavlinkAckOutcome? outcome = Acknowledge(ackShape, arming, MavCmdComponentArmDisarm);
        Console.WriteLine(
            outcome?.Kind == MavlinkAckKind.Final && outcome?.Value == MavResultAccepted
                ? "armed     the vehicle is ready"
                : $"the vehicle answered {outcome?.Kind} {outcome?.Value}");

        static MavlinkAckOutcome? Acknowledge(
            MavlinkSchema shape,
            MavlinkCommand tracked,
            ushort command)
        {
            using MavlinkMessage ack = shape.CreateMessage();
            ack.Set("command", command);
            ack.Set("result", 0);
            using MavlinkFrame frame = ack.ToFrame(new MavlinkHeader(1, 1, 0));
            return tracked.OnFrame(frame);
        }
        // ANCHOR_END: example

        Expect(heard.Get("type") == MavTypeQuadrotor, "the vehicle says what it is");
        Expect(received.MessageId == 0, "and it is a heartbeat");
        Expect(resend == 1, "a timeout numbers the resend");
        Expect(stray?.Kind == MavlinkAckKind.Unrelated, "another command's ack is not this one");
        Expect(outcome?.Kind == MavlinkAckKind.Final, "this one finishes the exchange");
        Expect(outcome?.Value == MavResultAccepted, "with the vehicle accepting");
    }
}
