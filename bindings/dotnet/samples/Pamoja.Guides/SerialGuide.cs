using System.Buffers.Binary;
using System.Text;

using Pamoja.Hal;
using Pamoja.Serial;

using static Guides.Guide;
using static System.FormattableString;

namespace Guides;

/// <summary>
/// The serial framing guide example: a weather mast whose node sends COBS frames up a UART to a
/// gateway; see docs/guides/serial.md.
/// </summary>
public static class SerialGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A reading is a two-byte sequence number, most significant byte first, then its text.
        static byte[] Reading(ushort sequence, string text)
        {
            byte[] payload = new byte[2 + Encoding.UTF8.GetByteCount(text)];
            BinaryPrimitives.WriteUInt16BigEndian(payload, sequence);
            Encoding.UTF8.GetBytes(text, payload.AsSpan(2));
            return payload;
        }

        // The line: 115200 baud, eight data bits, no parity, one stop bit. Ten bits a character.
        var settings = new SerialSettings(115_200);
        Console.WriteLine(Invariant(
            $"line         {settings}, {settings.BitsPerCharacter} bits a character, {settings.CharacterNanos / 1_000.0:F2} us each"));

        // The two ends of the cable with nothing plugged in. On a Raspberry Pi the gateway's end
        // is SerialPort.Open("/dev/serial0", settings) and nothing after this statement changes.
        var (gateway, node) = SerialPort.Pair(settings);
        using (gateway)
        using (node)
        {
            // A UART carries bytes, and nothing marks where a message ends, so the node frames
            // each reading with COBS: zero becomes the one byte that ends a frame and never
            // appears inside one, which matters here, since the sequence number is full of zeros.
            string[] texts = ["wind=12.4", "wind=13.1", "wind=11.8"];
            int sent = 0;
            for (int index = 0; index < texts.Length; index++)
            {
                byte[] frame = Serial.CobsEncode(Reading((ushort)(index + 1), texts[index]));
                node.Write(frame);
                sent += frame.Length;
            }

            Console.WriteLine(
                $"node         {texts.Length} readings of {Reading(1, texts[0]).Length} bytes, framed as {sent} bytes");

            // A read returns whatever has arrived, which is rarely one frame: here it is all
            // three. The decoder splits the stream back into payloads at each delimiter.
            byte[] arrived = gateway.Read(256, TimeSpan.FromMilliseconds(100));
            Console.WriteLine($"gateway      {arrived.Length} bytes in one read");
            using var decoder = new CobsDecoder();
            byte[][] payloads = decoder.Feed(arrived);
            foreach (byte[] payload in payloads)
            {
                ushort sequence = BinaryPrimitives.ReadUInt16BigEndian(payload);
                Console.WriteLine($"reading {sequence}    {Encoding.UTF8.GetString(payload, 2, payload.Length - 2)}");
            }

            // What one frame costs on the wire at this speed, start and stop bits included.
            int frameLength = sent / texts.Length;
            Console.WriteLine(Invariant(
                $"on the wire  {settings.TransferMicros(frameLength) / 1_000.0:F2} ms for a {frameLength}-byte frame at {settings}"));

            // The node restarts partway through a frame. As it comes back up it sends a lone
            // delimiter, which closes off the half frame, so the gateway drops it rather than
            // gluing it to the next one, and then it sends the reading again.
            byte[] again = Serial.CobsEncode(Reading(4, "wind=12.9"));
            node.Write(again.AsSpan(0, again.Length / 2));
            node.Write([Serial.CobsDelimiter]);
            node.Write(again);
            ulong droppedBefore = decoder.Discarded;
            byte[][] resent = decoder.Feed(gateway.Read(256, TimeSpan.FromMilliseconds(100)));
            ulong dropped = decoder.Discarded - droppedBefore;
            Console.WriteLine(
                $"restart      {dropped} frame cut short and dropped, then {Encoding.UTF8.GetString(resent[0], 2, resent[0].Length - 2)}");

            // SLIP, the older framing, ends a frame with one reserved byte and escapes that byte
            // and its own escape byte inside one. With no reserved bytes in a reading it costs a
            // byte less than COBS; a payload full of them costs up to twice its length under
            // SLIP, and never more than one byte in 254 over under COBS.
            byte[] first = Reading(1, texts[0]);
            int slipLength = Serial.SlipEncode(first).Length;
            int cobsLength = Serial.CobsEncode(first).Length;
            Console.WriteLine(
                $"framing      {first.Length} payload bytes: {slipLength} under SLIP, {cobsLength} under COBS");

            // The node goes quiet. A read waits for the first byte up to its timeout; on a port
            // with nothing plugged in it returns at once and counts the wait instead of sleeping
            // through it, so a test of a silent node takes no time.
            ulong waitedBefore = gateway.WaitedMicros;
            byte[] quiet = gateway.Read(256, TimeSpan.FromMilliseconds(500));
            ulong waited = (gateway.WaitedMicros - waitedBefore) / 1_000;
            Console.WriteLine($"silence      {quiet.Length} bytes in {waited} ms, counted and not slept");
            // ANCHOR_END: example

            Expect(settings.CharacterNanos == 86_806, "10 bits at 115200");
            Expect(payloads.Length == 3 && payloads[2].SequenceEqual(Reading(3, "wind=11.8")), "three readings");
            Expect(sent == 39, "each 11-byte payload gains one code byte and a delimiter");
            Expect(settings.TransferMicros(13) == 1_129, "a frame's time on the wire");
            Expect(dropped == 1, "the half frame is dropped");
            Expect(resent.Length == 1 && resent[0].SequenceEqual(Reading(4, "wind=12.9")), "the reading sent again");
            Expect(slipLength == 12 && cobsLength == 13, "the framing costs");
            Expect(gateway.WaitedMicros == 500_000, "the silence was counted");
            Expect(node.Written == 39 + (again.Length / 2) + 1 + again.Length, "every byte the node wrote");
        }
    }
}
