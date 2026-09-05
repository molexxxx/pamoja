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
        // J1939 keeps its addressing inside the CAN identifier: a priority, a parameter
        // group that says what the message is, and the address of whatever sent it.
        // Building one from those fields saves a caller packing 29 bits by hand.
        const byte Engine = 0x00;
        const uint Eec1 = 61_444; // electronic engine controller 1, which carries speed
        uint broadcast = Can.ComposeJ1939(3, Eec1, Engine);
        J1939Message engine = Can.DecodeJ1939(broadcast)!;
        Console.WriteLine($"broadcast priority {engine.Priority} pgn {engine.Pgn}");
        Console.WriteLine($"addressed to one node: {!engine.Broadcast}");

        // A parameter group below the PDU1 limit is addressed rather than broadcast, so
        // those eight identifier bits carry a destination instead of extending the group.
        const uint Request = 59_904;
        const byte Gateway = 0x01;
        const byte Transmission = 0x21;
        J1939Message request = Can.DecodeJ1939(
            Can.ComposeJ1939(6, Request, Gateway, Transmission))!;
        Console.WriteLine($"request   pgn {request.Pgn} to node 0x{request.Destination:X2}");
        Console.WriteLine($"heard     from 0x{request.Source:X2}");

        // J1939 never rides an 11-bit identifier, so a standard frame is not one.
        Console.WriteLine(
            $"an 11-bit identifier is J1939: {Can.DecodeJ1939(0x123, extended: false) is not null}");

        // The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of
        // that parameter group at 0.125 rpm per bit, and every signal this controller is
        // not reporting is filled with the not-available byte the standard reserves.
        byte[] payload = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        BitConverter.TryWriteBytes(payload.AsSpan(3), (ushort)(1000 / 0.125));
        CanFrame eec1 = Can.Frame(broadcast, payload, extended: true);
        double speed = BitConverter.ToUInt16(eec1.Data, 3) * 0.125;
        Console.WriteLine($"engine    {speed} rpm in {eec1.Dlc} bytes");

        // Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
        // classic frame still refuses a ninth byte.
        Console.WriteLine(
            $"32 bytes carries length code {Can.FdFrame(broadcast, new byte[32], true).Dlc}");
        try
        {
            Can.Frame(broadcast, new byte[9], extended: true);
            Console.WriteLine("a classic frame took nine bytes, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"classic   refused nine bytes: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(engine.Priority == 3, "the priority survives the round trip");
        Expect(engine.Pgn == Eec1, "and so does the parameter group");
        Expect(engine.Broadcast && engine.Destination is null, "a broadcast has no destination");
        Expect(request.Pgn == Request, "an addressed group keeps its number");
        Expect(request.Destination == Transmission, "and carries the node it is for");
        Expect(request.Source == Gateway, "and the node it came from");
        Expect(Can.DecodeJ1939(0x123, extended: false) is null, "J1939 is never 11-bit");
        Expect(eec1.Dlc == 8, "the broadcast fills eight bytes");
        Expect(speed == 1000.0, "which decode to a thousand rpm");
        Expect(Can.FdFrame(broadcast, new byte[32], true).Dlc == 13, "32 bytes is length code 13");
    }
}
