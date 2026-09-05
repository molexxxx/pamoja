using Pamoja;
using Pamoja.Modbus;

using static Guides.Guide;

namespace Guides;

/// <summary>The Modbus RTU guide example; see docs/guides/modbus.md.</summary>
public static class ModbusGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The device this gateway polls: a power meter at unit 17, whose manual says the
        // three registers holding voltage, current and a fault word start at address 107.
        const byte Meter = 17;
        const ushort FirstRegister = 107;

        // Ask it for those three registers. The frame is complete, checksum included,
        // exactly as it goes out on the wire.
        byte[] request = Modbus.ReadHoldingRegisters(Meter, FirstRegister, 3);
        Console.WriteLine($"polling unit {Meter}, {request.Length} bytes out");

        // A stand-in for the meter. On a running gateway this frame arrives over RS485;
        // here the library builds what a meter reporting those values would send back.
        byte[] fromTheMeter = Modbus.ReadHoldingRegistersReply(Meter, [2301, 418, 0]);

        // Everything below is the gateway's own code. A reply carries its own checksum,
        // so the frame is validated before any value is read out of it.
        ModbusFrame reply = Modbus.ParseFrame(fromTheMeter);
        ushort[] registers = reply.Registers();
        Console.WriteLine($"voltage   {registers[0] / 10.0:F1} V");
        Console.WriteLine($"current   {registers[1] / 100.0:F2} A");
        Console.WriteLine($"faults    {registers[2]}");

        // One flipped bit anywhere in the frame fails the checksum, which is the whole
        // point of carrying one over a long RS485 run.
        byte[] mangled = [.. fromTheMeter];
        mangled[2] ^= 0xFF;
        try
        {
            Modbus.ParseFrame(mangled);
            Console.WriteLine("mangled frame accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"mangled frame rejected: {error.Message}");
        }
        // ANCHOR_END: example

        // The request and reply frames the specification fixes are pinned in the crate
        // tests, so a guide asserts behaviour instead.
        Expect(request.Length == 8, "a three-register request is eight bytes on the wire");
        Expect(reply.Address == Meter, "the reply comes from the unit that was asked");
        Expect(reply.Exception is null, "a served request reports no exception");
        Expect(
            registers.SequenceEqual(new ushort[] { 2301, 418, 0 }),
            "the three registers decode to what the meter reported");
    }
}
