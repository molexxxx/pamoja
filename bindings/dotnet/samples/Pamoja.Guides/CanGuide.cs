using Pamoja;
using Pamoja.Can;

using static Guides.Guide;
using static System.FormattableString;

namespace Guides;

/// <summary>
/// The CAN and J1939 guide example: a standby generator's bus, with the engine controller
/// broadcasting its speed, a monitoring gateway keeping only that, a service laptop hearing
/// everything, and a coolant sensor speaking plain CAN; see docs/guides/can.md.
/// </summary>
public static class CanGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The nodes by the address each answers to, and the two parameter groups in play.
        const byte Engine = 0;
        const byte Gateway = 1;
        const uint EngineController1 = 61_444; // carries engine speed
        const uint Request = 59_904; // asks another node for a parameter group

        // Where engine speed sits inside that group, and the scale the standard fixes for it.
        const int EngineSpeedAt = 3;
        const double RpmPerBit = 0.125;

        // J1939 keeps its addressing inside the 29-bit identifier: a priority, the parameter
        // group, and the sender's address. A broadcast names no destination.
        uint speedId = Can.BroadcastJ1939(J1939Priority.Control, EngineController1, Engine);
        J1939Message speed = Can.DecodeJ1939(speedId)!;
        Console.WriteLine(
            $"engine speed 0x{speedId:X8}: pgn {speed.Pgn} at priority {speed.Priority}, from node {Engine} to every node");

        // A reading starts with every signal marked not available, and the engine writes only
        // its speed.
        CanFrame Reading(double rpm)
        {
            Signals reported = Signals.New();
            reported.SetU16(EngineSpeedAt, (ushort)(rpm / RpmPerBit));
            return Can.Frame(speedId, reported.ToArray(), extended: true);
        }

        static double RpmOf(CanFrame received) =>
            (Signals.From(received.Data).U16(EngineSpeedAt) ?? 0) * RpmPerBit;

        CanFrame first = Reading(1500);
        int unreported = first.Data.Count(value => value == Signals.NotAvailable);
        Console.WriteLine(Invariant(
            $"payload      {RpmOf(first):F1} rpm in bytes {EngineSpeedAt + 1} and {EngineSpeedAt + 2}, the other {unreported} not available"));

        // Four nodes on one bus with nothing plugged in. On a Linux board each is
        // CanBus.Open("can0"), and nothing after this statement changes.
        using CanBus engine = CanBus.Simulated();
        using CanBus gateway = engine.Join();
        using CanBus laptop = engine.Join();
        using CanBus sensor = engine.Join();

        // The gateway keeps engine speed and nothing else; the laptop keeps everything.
        gateway.SetFilters(CanFilter.Pgn(EngineController1));

        // Two engine readings, and between them the coolant sensor, which speaks plain CAN: its
        // level in percent on the 11-bit identifier 0x120.
        engine.Send(first);
        sensor.Send(Can.Frame(0x120, [87]));
        engine.Send(Reading(1512.5));

        // Every node hears every frame but its own, and keeps what its filters pass.
        while (gateway.Receive(TimeSpan.FromMilliseconds(10)) is { } kept)
        {
            byte from = Can.DecodeJ1939(kept.Id, kept.Extended)?.Source ?? 0;
            Console.WriteLine(Invariant($"gateway      {RpmOf(kept):F1} rpm from node {from}"));
        }

        long onTheBus = engine.Sent + sensor.Sent;
        Console.WriteLine($"gateway      kept {gateway.Received} of the {onTheBus} frames on the bus");
        var heard = new List<CanFrame>();
        while (laptop.Receive(TimeSpan.FromMilliseconds(10)) is { } received)
        {
            heard.Add(received);
        }

        if (heard.Find(received => Can.DecodeJ1939(received.Id, received.Extended) is null) is { } plain)
        {
            Console.WriteLine(
                $"laptop       heard {heard.Count}, among them 0x{plain.Id:X3}, an 11-bit identifier and no J1939 message");
        }

        // A request is addressed rather than broadcast: below the PDU1 limit, eight bits of the
        // identifier name the node it is for.
        J1939Message request = Can.DecodeJ1939(
            Can.ComposeJ1939((byte)J1939Priority.Normal, Request, Gateway, Engine))!;
        Console.WriteLine($"request      pgn {request.Pgn} from node {request.Source} to node {request.Destination}");

        // The engine goes quiet. A receive waits for a frame up to its timeout; on a simulated
        // bus it returns at once and counts the wait instead of sleeping through it.
        ulong before = gateway.WaitedMicros;
        CanFrame? quiet = gateway.Receive(TimeSpan.FromMilliseconds(500));
        Console.WriteLine(
            $"silent       {(quiet is null ? 0 : 1)} frames in {(gateway.WaitedMicros - before) / 1_000} ms, counted and not slept");

        // Above eight bytes CAN FD encodes a length in steps, and a classic frame refuses a
        // ninth byte.
        CanFrame wide = Can.FdFrame(speedId, new byte[32], extended: true);
        Console.WriteLine($"fd           32 bytes travel at data length code {wide.Dlc}");
        try
        {
            Can.Frame(speedId, new byte[9], extended: true);
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"classic      refused nine bytes: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(speedId == 0x0CF0_0400, "the engine speed broadcast");
        Expect(unreported == 6, "six bytes not reported");
        Expect(gateway.Received == 2, "the gateway kept the two readings");
        Expect(heard.Count == 3, "the laptop heard everything");
        Expect(request.Destination == Engine, "addressed to the engine");
        Expect(quiet is null, "nothing more came");
        Expect(wide.Dlc == 13, "32 bytes is length code 13");
    }
}
