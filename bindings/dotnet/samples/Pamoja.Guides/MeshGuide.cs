using System.Text;

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
        string to = reading.Dst == Mesh.Broadcast ? "every node in range" : "one node";
        Console.WriteLine($"sent      {reading.Bytes.Length} bytes to {to}, hop limit {reading.HopLimit}");

        // A neighbor hears it. Every node in range rebroadcasts, so the same packet
        // arrives several times over; the source and sequence id decide which copy is
        // the first.
        MeshFrame received = Mesh.Parse(reading.Bytes);
        Console.WriteLine($"payload   {Encoding.UTF8.GetString(received.Payload)}");
        using SeenPackets seen = new(64);
        bool first = seen.Record(received.Src, received.Id);
        bool again = seen.Record(received.Src, received.Id);
        if (first && !again)
        {
            Console.WriteLine("dedup     the first copy is relayed, and the second is dropped");
        }

        // Relaying spends one hop. The checksum skips the hop-limit byte, so a relay
        // forwards the frame without recomputing it and the check stays end to end.
        MeshFrame forwarded = Mesh.Relayed(received.Bytes)!;
        MeshFrame onward = Mesh.Parse(forwarded.Bytes);
        Console.WriteLine(
            $"relayed   hop limit {forwarded.HopLimit}, and the checksum still holds: " +
            Encoding.UTF8.GetString(onward.Payload));

        // A frame that has run out of hops is not relayed again, which is what ends the
        // flood.
        if (Mesh.Relayed(Mesh.BroadcastFrame(RiverGauge, 1, "level=high"u8, hopLimit: 0).Bytes) is null)
        {
            Console.WriteLine("spent     at hop limit 0 the frame goes no further");
        }

        // A payload byte the air mangled fails the checksum rather than reaching the
        // application as a plausible reading. The header is a fixed width, so the first
        // byte past it is the first byte of the reading itself.
        byte[] mangled = reading.Bytes.ToArray();
        mangled[Mesh.HeaderLen] ^= 0xFF;
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

        Expect(Encoding.UTF8.GetString(received.Payload) == "level=high", "the payload survives");
        Expect(forwarded.HopLimit == received.HopLimit - 1, "a relay spends one hop");
        Expect(onward.Payload.SequenceEqual(received.Payload), "and carries the same payload");

        // ANCHOR: flood
        // Six nodes down a river valley, each in range of its neighbors only. The gauge at
        // the top, node 0, floods a reading, and every node that hears a packet for the
        // first time takes it and relays it while hops remain. Copies that come back up the
        // valley are echoes, which the memory of each node drops.
        const int Nodes = 6;
        SeenPackets[] memory = Enumerable.Range(0, Nodes).Select(_ => new SeenPackets(64)).ToArray();
        MeshFrame flooded = Mesh.BroadcastFrame(0, 1, "level=high"u8);
        memory[0].Record(flooded.Src, flooded.Id);
        var onTheAir = new Queue<(int From, byte[] Bytes)>();
        onTheAir.Enqueue((0, flooded.Bytes));
        int delivered = 0;
        int relays = 0;
        int echoes = 0;
        int farthest = 0;
        while (onTheAir.TryDequeue(out var sent))
        {
            foreach (int node in new[] { sent.From - 1, sent.From + 1 })
            {
                if (node < 0 || node >= Nodes)
                {
                    continue;
                }

                MeshFrame heard = Mesh.Parse(sent.Bytes);
                if (!memory[node].Record(heard.Src, heard.Id))
                {
                    echoes++;
                    continue;
                }

                delivered++;
                farthest = Math.Max(farthest, node);
                if (Mesh.Relayed(heard.Bytes) is { } onwardFrame)
                {
                    relays++;
                    onTheAir.Enqueue((node, onwardFrame.Bytes));
                }
            }
        }

        foreach (SeenPackets cache in memory)
        {
            cache.Dispose();
        }

        Console.WriteLine($"flood     {delivered} nodes took the reading, {relays} relayed it, {echoes} echoes were dropped");
        Console.WriteLine(
            $"reach     node {farthest} was the farthest, {farthest} hops out, and node {Nodes - 1} never heard it");

        // A node whose memory holds two packets hears the reading, then two packets from
        // other nodes, then a late copy of the reading by a longer path. It has forgotten
        // the reading by then and relays it again; on a busy mesh that repeats without end.
        using SeenPackets small = new(2);
        MeshFrame rain = Mesh.BroadcastFrame(7, 1, "rain=4mm"u8);
        MeshFrame wind = Mesh.BroadcastFrame(8, 1, "wind=12"u8);
        small.Record(flooded.Src, flooded.Id);
        small.Record(rain.Src, rain.Id);
        small.Record(wind.Src, wind.Id);
        if (small.Record(flooded.Src, flooded.Id))
        {
            Console.WriteLine("forgot    a memory of two packets relays the late copy again");
        }

        // A payload one byte past what a frame carries is refused before it is built. A
        // frame is sized to the 250 bytes ESP-NOW carries, less the header and the checksum.
        try
        {
            Mesh.BroadcastFrame(0, 2, new byte[Mesh.MaxPayload + 1]);
            Console.WriteLine("an oversized payload was framed, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"too long  {Mesh.MaxPayload + 1} bytes refused: {error.Message}");
        }
        // ANCHOR_END: flood

        Expect(farthest == Mesh.DefaultHopLimit + 1, "a flood reaches one hop past its hop limit");
    }
}
