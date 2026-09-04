using Pamoja;
using Pamoja.Mesh;

using static Guides.Guide;

namespace Guides;

/// <summary>The mesh framing guide example; see docs/guides/mesh.md.</summary>
public static class MeshGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A river gauge floods a reading to every node in range. The header is fixed and
        // big-endian: version, source, destination, sequence id, hop limit, then the
        // payload and a checksum over everything but the hop limit.
        MeshFrame reading = Mesh.BroadcastFrame(0x1234_5678, 1, "level=high"u8);
        Expect(reading.Dst == Mesh.Broadcast, "a broadcast is addressed to every node");
        Expect(
            reading.Bytes.SequenceEqual(
                Convert.FromHexString("0112345678ffffffff0001036c6576656c3d686967683335")),
            "the frame is the bytes that go on the air");

        // The checksum is CRC-16/CCITT-FALSE, whose published check value fixes the
        // polynomial and the starting value.
        Expect(Mesh.Crc16("123456789"u8) == 0x29B1, "the checksum is CRC-16/CCITT-FALSE");

        // A neighbour hears it. Every node in range rebroadcasts, so the same packet
        // arrives several times over; the source and sequence id decide which copy is
        // the first.
        MeshFrame received = Mesh.Parse(reading.Bytes);
        Expect(received.Payload.SequenceEqual("level=high"u8.ToArray()), "it carries the reading");
        using SeenPackets seen = new(64);
        Expect(seen.Record(received.Src, received.Id), "the first copy is new");
        Expect(!seen.Record(received.Src, received.Id), "a second copy is a duplicate");

        // Relaying spends one hop. The checksum skips the hop-limit byte, so a relay
        // forwards the frame without recomputing it and the check stays end to end.
        MeshFrame forwarded = Mesh.Relayed(received.Bytes)!;
        Expect(forwarded.HopLimit == received.HopLimit - 1, "relaying spends one hop");
        Expect(
            Mesh.Parse(forwarded.Bytes).Payload.SequenceEqual(received.Payload),
            "and leaves the frame valid on the air");
        Expect(
            Mesh.Relayed(Mesh.BroadcastFrame(0x1234_5678, 1, "level=high"u8, 0).Bytes) is null,
            "a packet out of hops is not relayed further");

        // A payload byte the air mangled fails the checksum rather than reaching the
        // application as a plausible reading.
        byte[] mangled = [.. reading.Bytes];
        mangled[12] ^= 0xFF;
        bool rejected = false;
        try
        {
            Mesh.Parse(mangled);
        }
        catch (PamojaException)
        {
            rejected = true;
        }
        Expect(rejected, "a frame mangled on the air is rejected");
        // ANCHOR_END: example
    }
}
