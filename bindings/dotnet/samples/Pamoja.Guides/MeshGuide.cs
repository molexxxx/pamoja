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
        // A river gauge floods a level reading to every node in range. The header is fixed
        // and big-endian: version, source, destination, sequence id, hop limit, then the
        // payload and a checksum over everything but the hop limit.
        const uint RiverGauge = 305_419_896;
        MeshFrame reading = Mesh.BroadcastFrame(RiverGauge, 1, "level=high"u8);
        Console.WriteLine($"sent      {reading.Bytes.Length} bytes to every node in range");
        Console.WriteLine($"addressed to broadcast: {reading.Dst == Mesh.Broadcast}");

        // A neighbour hears it. Every node in range rebroadcasts, so the same packet
        // arrives several times over; the source and sequence id decide which copy is
        // the first.
        MeshFrame received = Mesh.Parse(reading.Bytes);
        Console.WriteLine($"payload   {System.Text.Encoding.UTF8.GetString(received.Payload)}");

        using SeenPackets seen = new(64);
        bool first = seen.Record(received.Src, received.Id);
        bool again = seen.Record(received.Src, received.Id);
        Console.WriteLine($"first copy relayed: {first}, second copy relayed: {again}");

        // Relaying spends one hop. The checksum skips the hop-limit byte, so a relay
        // forwards the frame without recomputing it and the check stays end to end.
        MeshFrame forwarded = Mesh.Relayed(received.Bytes)!;
        Console.WriteLine($"relayed   hop limit {forwarded.HopLimit}");
        MeshFrame onward = Mesh.Parse(forwarded.Bytes);
        Console.WriteLine($"onward    {System.Text.Encoding.UTF8.GetString(onward.Payload)}");

        // A frame that has run out of hops is not relayed again, which ends the flood.
        MeshFrame? spent = Mesh.Relayed(Mesh.BroadcastFrame(RiverGauge, 1, "level=high"u8, 0).Bytes);
        Console.WriteLine(spent is null
            ? "spent     hop limit reached, the flood stops here"
            : "a spent frame was relayed, which should never happen");

        // A payload byte the air mangled fails the checksum rather than reaching the
        // application as a plausible reading.
        byte[] mangled = [.. reading.Bytes];
        mangled[12] ^= 0xFF;
        try
        {
            Mesh.Parse(mangled);
            Console.WriteLine("a mangled frame was accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"mangled   rejected: {error.Message}");
        }
        // ANCHOR_END: example

        // The frame layout is pinned once, in the generated conformance vectors, so a
        // guide asserts behaviour instead.
        Expect(received.Payload.SequenceEqual("level=high"u8.ToArray()), "it carries the reading");
        Expect(first, "the first copy is new");
        Expect(!again, "a second copy is a duplicate");
        Expect(forwarded.HopLimit == received.HopLimit - 1, "relaying spends one hop");
        Expect(onward.Payload.SequenceEqual(received.Payload), "and the payload survives it");
        Expect(spent is null, "a frame with no hops left is not relayed");
    }
}
