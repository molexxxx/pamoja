using Pamoja.Serial;

using static Guides.Guide;

namespace Guides;

/// <summary>The serial framing guide example; see docs/guides/serial.md.</summary>
public static class SerialGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A UART carries bytes, not packets, so a framing has to mark where one packet
        // ends. SLIP reserves two byte values for that, and the package names both: the
        // end byte closes a frame, the escape byte carries a value that would otherwise
        // look like one.
        byte[] payload = [.. "lvl="u8, Serial.SlipEnd, Serial.SlipEsc];
        byte[] framed = Serial.SlipEncode(payload);
        Console.WriteLine($"slip      {payload.Length} payload bytes framed as {framed.Length}");

        // Decoding gives the payload back unchanged, reserved bytes and all.
        byte[] restored = Serial.SlipDecode(framed);
        Console.WriteLine($"slip      decoded back to {restored.Length} bytes");

        // COBS trades that escaping for one code byte per run of up to 254 non-zero bytes,
        // each run led by its own length, so a frame never grows by more than a byte per
        // 254. Zero is the delimiter, and never appears inside a frame.
        byte[] packet = [.. "lvl="u8, Serial.CobsDelimiter, .. "7"u8];
        byte[] cobsFramed = Serial.CobsEncode(packet);
        Console.WriteLine($"cobs      {packet.Length} payload bytes framed as {cobsFramed.Length}");

        // A read from a port returns whatever arrived, which is rarely one whole frame.
        // This chunk holds two good frames with a truncated one between them; the decoder
        // hands over the good ones and discards only the bad frame.
        using SlipDecoder decoder = new();
        byte[] chunk =
        [
            .. "ok"u8,
            Serial.SlipEnd,
            Serial.SlipEsc, // a frame that ends before its escape pair completes
            Serial.SlipEnd,
            .. "go"u8,
            Serial.SlipEnd,
        ];
        IReadOnlyList<byte[]> frames = decoder.Feed(chunk);
        foreach (byte[] frame in frames)
        {
            Console.WriteLine($"received  {System.Text.Encoding.UTF8.GetString(frame)}");
        }

        Console.WriteLine($"discarded {decoder.Discarded} frame the stream mangled");
        // ANCHOR_END: example

        // The frames each specification fixes are pinned once, in the crate tests and the
        // generated conformance vectors, so a guide asserts behaviour instead.
        Expect(framed.Length > payload.Length, "stuffing costs bytes");
        Expect(cobsFramed.Length > packet.Length, "and so does the COBS code byte");
        Expect(restored.SequenceEqual(payload), "and it decodes back to the payload");
        Expect(Serial.CobsDecode(cobsFramed).SequenceEqual(packet), "COBS round-trips too");
        Expect(frames.Count == 2, "two whole frames came out of the chunk");
        Expect(decoder.Discarded == 1, "and only the truncated one was dropped");
    }
}
