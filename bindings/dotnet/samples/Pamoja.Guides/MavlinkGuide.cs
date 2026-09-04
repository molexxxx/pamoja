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
        // 0x6F91 over "123456789" is the catalogue check value for CRC-16/MCRF4XX, and 50
        // is the CRC_EXTRA the common dialect publishes for HEARTBEAT.
        Expect(Mavlink.Crc16("123456789"u8) == 0x6F91, "the checksum is CRC-16/MCRF4XX");
        Expect(Mavlink.KnownCrcExtra(0) == 50, "HEARTBEAT's published seed");

        // A HEARTBEAT announcing an onboard controller in an active state. The v2 frame
        // around it is the 0xFD marker, the payload length, two flag bytes, the sequence,
        // the sending system and component, a 24-bit message id, the payload, then the
        // checksum.
        byte[] heartbeat = [0, 0, 0, 0, 18, 0, 0, 4, 3];
        using MavlinkFrame sent = Mavlink.Frame(new MavlinkHeader(1, 1, 7), 0, heartbeat);
        byte[] wire =
        [
            0xFD, 0x09, 0x00, 0x00, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x04, 0x03, 0x75, 0x3A,
        ];
        Expect(sent.Bytes.SequenceEqual(wire), "the frame is the layout v2 fixes");

        // A link delivers bytes, not frames. The parser skips whatever does not start one
        // and drops a frame whose checksum fails rather than passing it on.
        byte[] mangled = sent.Bytes;
        mangled[14] ^= 0xFF;
        using MavlinkParser parser = new();
        byte[] noisy = [0x11, 0x22, 0x33, .. mangled];
        Expect(parser.Push(noisy).Count == 0, "neither noise nor a failed checksum is reported");

        // The same frame, split across two reads, still arrives whole.
        Expect(parser.Push(sent.Bytes.AsSpan(0, 5)).Count == 0, "half a frame is not a frame");
        IReadOnlyList<MavlinkFrame> found = parser.Push(sent.Bytes.AsSpan(5));
        Expect(found.Count == 1, "the rest of it completes one");
        using MavlinkFrame received = found[0];
        Expect(received.Version == MavlinkVersion.V2, "v2 is the current wire format");
        Expect(received.MessageId == 0, "and it is the heartbeat that was sent");
        Expect(received.Payload.SequenceEqual(heartbeat), "with its payload intact");
        // ANCHOR_END: example
    }
}
