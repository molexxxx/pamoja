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
        // SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a
        // payload carrying either goes out as the two-byte pair RFC 1055 fixes for it.
        byte[] payload = [0x01, 0xC0, 0xDB, 0x02];
        byte[] frame = Serial.SlipEncode(payload);
        Expect(
            frame.SequenceEqual(new byte[] { 0x01, 0xDB, 0xDC, 0xDB, 0xDD, 0x02, 0xC0 }),
            "the frame is the escaping RFC 1055 fixes");
        Expect(Serial.SlipDecode(frame).SequenceEqual(payload), "the payload comes back");

        // COBS trades that escaping for one code byte per run of up to 254 non-zero
        // bytes, each run led by its own length. This is the COBS paper's worked example.
        byte[] packet = [0x11, 0x22, 0x00, 0x33];
        byte[] framed = Serial.CobsEncode(packet);
        Expect(
            framed.SequenceEqual(new byte[] { 0x03, 0x11, 0x22, 0x02, 0x33, 0x00 }),
            "the frame is the one the COBS paper works through");
        Expect(Serial.CobsDecode(framed).SequenceEqual(packet), "the packet comes back");

        // A serial read returns an arbitrary chunk rather than a packet. This one holds
        // two frames with a truncated one between them, and only the bad frame is dropped.
        using SlipDecoder decoder = new();
        byte[][] frames = decoder.Feed([0x6F, 0x6B, 0xC0, 0xDB, 0xC0, 0x67, 0x6F, 0xC0]);
        Expect(frames.Length == 2, "the frames either side of the bad one survive");
        Expect(frames[0].SequenceEqual("ok"u8.ToArray()), "the first frame reassembles");
        Expect(frames[1].SequenceEqual("go"u8.ToArray()), "the second frame reassembles");
        Expect(decoder.Discarded == 1, "the truncated frame is counted, not raised");
        // ANCHOR_END: example
    }
}
