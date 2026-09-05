using Pamoja;
using Pamoja.Can;

using static Guides.Guide;

namespace Guides;

/// <summary>The CAN and J1939 guide example; see docs/guides/can.md.</summary>
public static class CanGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The nodes on this bus, by the address each answers to, and the two parameter
        // groups in play. J1939 publishes both, so naming them makes the traffic readable.
        const byte Engine = 0;
        const byte Gateway = 1;
        const byte Gearbox = 33;
        const uint EngineController1 = 61_444; // carries engine speed
        const uint Request = 59_904; // asks another node for a parameter group

        // Where engine speed sits inside that group, and the scale the standard fixes for
        // it. Naming both is what stops a sender and a receiver disagreeing about either.
        const int EngineSpeedAt = 3;
        const double RpmPerBit = 0.125;

        // J1939 keeps its addressing inside the CAN identifier: a priority, the parameter
        // group, and the address of whatever sent it. A broadcast has no destination, so
        // it is its own constructor rather than a magic address a caller has to know.
        uint speedId = Can.BroadcastJ1939(J1939Priority.Control, EngineController1, Engine);
        J1939Message speed = Can.DecodeJ1939(speedId)!;
        Console.WriteLine($"broadcast pgn {speed.Pgn} at priority {speed.Priority}");

        // A parameter group below the PDU1 limit is addressed rather than broadcast, so
        // those eight identifier bits carry a destination instead of extending the group.
        uint requestId = Can.ComposeJ1939((byte)J1939Priority.Normal, Request, Gateway, Gearbox);
        Console.WriteLine($"request   pgn {Request} addressed to node {Gearbox}");

        // Reading one back off the bus is the same thing in reverse, so a receiver never
        // unpacks 29 bits by hand.
        J1939Message heard = Can.DecodeJ1939(requestId)!;
        Console.WriteLine($"heard     from node {heard.Source} for node {heard.Destination}");

        // The payload. Every signal starts marked not available, and this controller
        // reports only engine speed, so that is the only one it writes.
        Signals reported = Signals.New();
        reported.SetU16(EngineSpeedAt, (ushort)(1000 / RpmPerBit));
        CanFrame eec1 = Can.Frame(speedId, reported.ToArray(), extended: true);

        // The receiving node reads the same offset back, so neither end slices the payload.
        double rpm = Signals.From(eec1.Data).U16(EngineSpeedAt)!.Value * RpmPerBit;
        Console.WriteLine($"engine    {rpm} rpm, carried in {eec1.Dlc} bytes");

        // Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
        // classic frame still refuses a ninth byte.
        CanFrame wide = Can.FdFrame(speedId, new byte[32], extended: true);
        Console.WriteLine($"32 bytes carries length code {wide.Dlc}");
        try
        {
            Can.Frame(speedId, new byte[9], extended: true);
            Console.WriteLine("a classic frame took nine bytes, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"classic   refused nine bytes: {error.Message}");
        }

        // J1939 never rides an 11-bit identifier, so a standard frame is not one of its
        // messages however its bits happen to line up.
        Console.WriteLine($"an 11-bit identifier is J1939: {Can.DecodeJ1939(291, false) is not null}");
        // ANCHOR_END: example

        Expect(speed.Priority == (byte)J1939Priority.Control, "a control priority");
        Expect(speed.Pgn == EngineController1, "the engine controller group");
        Expect(speed.Broadcast && speed.Destination is null, "a broadcast has no destination");
        Expect(heard.Pgn == Request, "the request group");
        Expect(heard.Destination == Gearbox, "addressed to the gearbox");
        Expect(heard.Source == Gateway, "sent by the gateway");
        Expect(rpm == 1000.0, "a thousand rpm");
        Expect(eec1.Dlc == 8, "eight bytes");
        Expect(wide.Dlc == 13, "32 bytes is length code 13");
        Expect(Can.DecodeJ1939(291, false) is null, "J1939 does not ride an 11-bit identifier");
        Expect(reported.U8(0) == Signals.NotAvailable, "every signal it does not report");
    }
}
