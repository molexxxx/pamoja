using Pamoja.Mesh;
using Pamoja.Routing;

using static Guides.Guide;

namespace Guides;

/// <summary>The mesh-routing guide example; see docs/guides/routing.md.</summary>
public static class RoutingGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The nodes on this mesh. An address is just a number; naming them is what makes
        // the table below read as a map of the site rather than a list of numbers.
        const uint Gateway = 1;
        const uint Pump = 9;
        const uint Tank = 10;
        const uint NorthRelay = 5;
        const uint EastRelay = 7;
        const uint SouthRelay = 3;
        const uint Silo = 32;

        // A node learns the way to another from traffic it already hears: a packet from
        // the pump that arrived through a relay proves that relay is a way back, at the
        // cost the packet reports. The table keeps the cheapest way it has heard, and a tie
        // keeps the way in use so two equal paths do not flap. Word from the relay already
        // in use is taken even when it is worse, which is how a failing link lets a detour
        // win.
        using Router router = new(Gateway, 4);
        (uint Via, ushort Cost)[] heardFromThePump =
        [
            (NorthRelay, 2),
            (EastRelay, 1),
            (SouthRelay, 4),
            (NorthRelay, 1),
            (EastRelay, 3),
            (NorthRelay, 2),
        ];
        foreach ((uint via, ushort cost) in heardFromThePump)
        {
            bool changed = router.Observe(Pump, via, cost);
            Route? route = router.RouteTo(Pump);
            string outcome = changed ? "so the route is" : "and the route stays";
            Console.WriteLine(
                $"heard     the pump via {via} at cost {cost}, {outcome} {route?.NextHop} at cost {route?.Cost}");
        }

        // The table lists what it holds, one route for each node it has heard from.
        router.Observe(Tank, NorthRelay, 3);
        IEnumerable<string> held = router.Routes()
            .Select(route => $"to {route.Dst} via {route.NextHop} at cost {route.Cost}");
        Console.WriteLine($"table     {router.Count} routes of {router.Capacity}: {string.Join(", ", held)}");

        // Every packet gets one of three answers: deliver it here, relay it to the
        // neighbor on the way, or flood it because no route is known yet.
        foreach ((string name, uint address) in
            new[] { ("gateway", Gateway), ("pump", Pump), ("silo", Silo) })
        {
            ForwardDecision decision = router.Forward(address);
            Console.WriteLine(decision.Action switch
            {
                ForwardAction.Deliver => $"{name,-10}deliver here",
                ForwardAction.Relay => $"{name,-10}relay via {decision.NextHop}",
                _ => $"{name,-10}flood, no route known",
            });
        }

        // The table keeps no clock, so a route through a relay that has gone quiet stays
        // until the caller forgets it, typically when a relayed packet goes unanswered.
        // Forgetting returns the node's traffic to flooding, the answer that always works.
        router.Forget(Pump);
        if (router.Forward(Pump).Action == ForwardAction.Flood)
        {
            Console.WriteLine($"forgot    the pump, so it floods again, and {router.Count} route is left");
        }
        // ANCHOR_END: example

        Expect(router.Count == 1, "forgetting drops exactly one route");
        Expect(router.NextHop(Tank) == NorthRelay, "and leaves the tank's");

        // ANCHOR: limits
        // A table has a fixed number of slots. Once they are full, a cheaper route takes
        // the slot of the costliest one held and a costlier route is refused, so a small
        // table keeps the nodes nearest to it and floods to the rest.
        const uint Well = 11;
        const uint Gate = 12;
        using Router small = new(Gateway, 2);
        small.Observe(Tank, NorthRelay, 3);
        small.Observe(Silo, SouthRelay, 5);
        IEnumerable<string> kept = small.Routes()
            .Select(route => $"to {route.Dst} via {route.NextHop} at cost {route.Cost}");
        Console.WriteLine($"full      {small.Count} routes of {small.Capacity}: {string.Join(", ", kept)}");
        if (small.Observe(Well, EastRelay, 2) && small.RouteTo(Silo) is null)
        {
            Console.WriteLine("evicted   the well at cost 2 took the slot of the silo, the costliest held");
        }

        if (!small.Observe(Gate, EastRelay, 6) && small.Forward(Gate).Action == ForwardAction.Flood)
        {
            Console.WriteLine("refused   the gate at cost 6 costs more than every route held, so it floods");
        }

        // A flood echoes, and the gateway hears its own packets come back through the
        // relays. A route to the node itself is never learned, whatever it costs.
        if (!router.Observe(Gateway, EastRelay, 2))
        {
            Console.WriteLine("echo      the gateway's own packet coming back teaches it nothing");
        }

        // A table with no slots is flooding with nothing remembered, which a node with no
        // memory to spare can still do. A packet for the node itself is still delivered.
        using Router none = new(Gateway, 0);
        bool learned = none.Observe(Pump, EastRelay, 1);
        if (!learned
            && none.Forward(Pump).Action == ForwardAction.Flood
            && none.Forward(Gateway).Action == ForwardAction.Deliver)
        {
            Console.WriteLine(
                "no room   a table of 0 learns nothing: the pump floods, and the gateway still delivers");
        }
        // ANCHOR_END: limits

        Expect(small.NextHop(Well) == EastRelay, "the well took the silo's slot");
        Expect(none.Count == 0, "a table of 0 holds nothing");

        // ANCHOR: site
        // Who hears whom on the site. The gateway hears the three relays, and each relay
        // hears the one node beyond it; the pump is out of the gateway's range.
        var site = new Dictionary<uint, uint[]>
        {
            [Gateway] = [NorthRelay, EastRelay, SouthRelay],
            [NorthRelay] = [Gateway, Tank],
            [EastRelay] = [Gateway, Pump],
            [SouthRelay] = [Gateway, Silo],
            [Tank] = [NorthRelay],
            [Pump] = [EastRelay],
            [Silo] = [SouthRelay],
        };

        // Sends a frame from one node and plays out what the site does with it. A node that
        // hears a frame for the first time learns the way back to its source, then asks its
        // own table what to do: deliver it, relay it to the one neighbor on the way, or
        // flood it to every neighbor in range, spending a hop each time it goes on. Every
        // frame here starts at the default hop limit, and one heard straight from its
        // source still has all of it, so the hops a frame has come are what it has spent,
        // plus one. Returns how many times a radio sent, and the nodes that took the frame.
        (int Sends, List<uint> Reached) Send(
            Dictionary<uint, Router> tables,
            uint source,
            MeshFrame first)
        {
            Dictionary<uint, SeenPackets> seen = site.Keys.ToDictionary(node => node, _ => new SeenPackets(64));
            seen[source].Record(first.Src, first.Id);
            int sends = 0;
            var reached = new List<uint>();
            var onTheAir = new Queue<(uint Sender, MeshFrame Sent)>();
            onTheAir.Enqueue((source, first));
            while (onTheAir.TryDequeue(out var next))
            {
                ForwardDecision decision = tables[next.Sender].Forward(next.Sent.Dst);
                if (decision.Action == ForwardAction.Deliver)
                {
                    continue;
                }

                uint[] to = decision.NextHop is uint hop ? [hop] : site[next.Sender];
                sends++;
                foreach (uint node in to)
                {
                    MeshFrame heard = Mesh.Parse(next.Sent.Bytes);
                    if (!seen[node].Record(heard.Src, heard.Id))
                    {
                        continue;
                    }

                    reached.Add(node);
                    ushort hops = (ushort)(Mesh.DefaultHopLimit - heard.HopLimit + 1);
                    tables[node].Observe(heard.Src, next.Sender, hops);
                    if (Mesh.Relayed(heard.Bytes) is { } onward)
                    {
                        onTheAir.Enqueue((node, onward));
                    }
                }
            }

            foreach (SeenPackets cache in seen.Values)
            {
                cache.Dispose();
            }

            return (sends, reached);
        }

        // The pump floods a reading, and every node that takes it learns the way back.
        Dictionary<uint, Router> tables = site.Keys.ToDictionary(node => node, node => new Router(node, 8));
        MeshFrame reading = Mesh.BroadcastFrame(Pump, 1, "flow=12"u8);
        (int sends, List<uint> reached) = Send(tables, Pump, reading);
        Console.WriteLine(
            $"flood     the pump's reading took {sends} sends to reach {reached.Count} nodes, and each learned the way back");

        // The gateway answers the pump. Each node on the way relays to the one neighbor its
        // table names, so the answer reaches only the nodes on the path.
        MeshFrame answer = Mesh.Frame(Gateway, Pump, 1, "run=10min"u8);
        (int routed, List<uint> path) = Send(tables, Gateway, answer);
        Console.WriteLine(
            $"routed    the gateway's answer reached only {string.Join(" and ", path)}, in {routed} sends");

        // The same answer on a site that has learned nothing floods, and every node relays
        // it.
        Dictionary<uint, Router> blank = site.Keys.ToDictionary(node => node, node => new Router(node, 8));
        (int flooded, List<uint> everyone) = Send(blank, Gateway, answer);
        Console.WriteLine(
            $"flooded   with nothing learned, the same answer reached all {everyone.Count} other nodes in {flooded} sends");
        // ANCHOR_END: site

        Expect(reached.Count == site.Count - 1, "the reading reached every other node");
        Expect(path.SequenceEqual([EastRelay, Pump]), "the answer took the learned path");
        Expect(routed < flooded, "and fewer sends than a flood");
        Expect(tables[Gateway].NextHop(Pump) == EastRelay, "the gateway learned the pump's relay");
        Expect(tables[Tank].Cost(Pump) == 4, "the tank learned the pump four hops out");

        foreach (Router table in tables.Values.Concat(blank.Values))
        {
            table.Dispose();
        }
    }
}
