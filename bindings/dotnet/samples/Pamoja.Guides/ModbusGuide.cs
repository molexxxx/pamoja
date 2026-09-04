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
        // Ask unit 0x11 for three holding registers starting at 0x006B. The last two bytes
        // are the CRC-16/MODBUS, so this is the frame exactly as it goes out on the wire.
        byte[] request = Modbus.ReadHoldingRegisters(0x11, 0x006B, 3);
        Expect(
            request.SequenceEqual(new byte[] { 0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87 }),
            "the request is the frame the specification fixes");

        // The device answers with three 16-bit registers. A reply carries its own checksum,
        // so the receiver validates the frame before reading any value out of it.
        byte[] body = [0x11, 0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64];
        ushort checksum = Modbus.Crc16(body);
        byte[] wire = [.. body, (byte)(checksum & 0xFF), (byte)(checksum >> 8)];
        using ModbusFrame reply = Modbus.ParseFrame(wire);
        Expect(reply.Address == 0x11, "the reply comes from the unit that was asked");
        Expect(reply.Exception is null, "a served request reports no exception");
        Expect(
            reply.Registers().SequenceEqual(new ushort[] { 0x022B, 0x0000, 0x0064 }),
            "the three registers read back");

        // One flipped bit anywhere in the frame fails the checksum, which is the whole
        // point of carrying one over a long RS485 run.
        byte[] corrupt = [.. wire];
        corrupt[2] ^= 0xFF;
        bool rejected = false;
        try
        {
            using ModbusFrame _ = Modbus.ParseFrame(corrupt);
        }
        catch (PamojaException)
        {
            rejected = true;
        }
        Expect(rejected, "a frame mangled on the wire is rejected");
        // ANCHOR_END: example
    }
}
