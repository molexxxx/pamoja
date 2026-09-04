using System.Buffers.Binary;

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
        // The engine-speed broadcast a J1939 engine or genset puts on the bus. J1939 keeps
        // its addressing in the identifier: a priority, a parameter group, a source address.
        J1939Message engine = Can.DecodeJ1939(0x0CF00400)!;
        Expect(engine.Priority == 3, "the broadcast carries priority 3");
        Expect(engine.Pgn == 61444, "engine speed is parameter group 61444");
        Expect(engine.Broadcast && engine.Destination is null, "a broadcast has no destination");

        // A PDU format below 0xF0 is addressed rather than broadcast, so those eight bits
        // hold a destination instead of extending the parameter group. 59904 is the
        // request group.
        J1939Message request = Can.DecodeJ1939(0x18EA2101)!;
        Expect(request.Pgn == 59904, "the request group decodes");
        Expect(request.Destination == 0x21 && !request.Broadcast, "addressed to node 0x21");
        Expect(Can.ComposeJ1939(6, 59904, 0x01, 0x21) == 0x18EA2101, "the fields compose back");

        // J1939 never rides an 11-bit identifier.
        Expect(Can.DecodeJ1939(0x123, extended: false) is null, "J1939 needs 29 bits");

        // The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
        // parameter group, little-endian at 0.125 rpm per bit, so 0x1F40 reads as 1000 rpm.
        byte[] payload = [0xF0, 0x7D, 0x7D, 0x40, 0x1F, 0x00, 0xF0, 0xFF];
        CanFrame eec1 = Can.Frame(0x0CF00400, payload, extended: true);
        Expect(eec1.Dlc == 8, "eight bytes is data length code 8");
        double rpm = BinaryPrimitives.ReadUInt16LittleEndian(eec1.Data.AsSpan(3, 2)) * 0.125;
        Expect(rpm == 1000.0, "the payload reads as 1000 rpm");

        // Above eight bytes CAN-FD encodes the length in steps, so 32 bytes is code 13,
        // while a classic frame still refuses a ninth byte.
        CanFrame wide = Can.FdFrame(0x0CF00400, new byte[32], extended: true);
        Expect(wide.Dlc == 13, "32 bytes is data length code 13");
        bool rejected = false;
        try
        {
            Can.Frame(0x0CF00400, new byte[9], extended: true);
        }
        catch (PamojaException)
        {
            rejected = true;
        }
        Expect(rejected, "classic CAN carries at most eight bytes");
        // ANCHOR_END: example
    }
}
