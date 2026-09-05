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
        const byte Gateway = 1;
        const byte Pump = 9;
        const byte Tank = 10;
        const byte NorthRelay = 5;
        const byte EastRelay = 7;
        const byte SouthRelay = 3;
        const byte Silo = 32;

        // A node learns the way to another from traffic it already hears: a packet from
        // the pump that arrived through the north relay proves that relay is a way back,
        // at the cost the packet reports.
        using Router router = new(Gateway, 4);
        router.Observe(Pump, NorthRelay, 2);

        // The table keeps only the cheapest way it knows to each node, so a cost-1 report
        // through the east relay takes over and the later cost-4 report changes nothing.
        router.Observe(Pump, EastRelay, 1);
        router.Observe(Pump, SouthRelay, 4);
        router.Observe(Tank, NorthRelay, 3);

        Route? route = router.RouteTo(Pump);
        Console.WriteLine($"to the pump   via {route?.NextHop} at cost {route?.Cost}");
        Console.WriteLine($"routes held   {router.Count}");

        // Every packet gets one of three answers: deliver it here, relay it to the
        // neighbour on the way, or flood it because no route is known yet.
        foreach ((string name, byte address) in
            new[] { ("gateway", Gateway), ("pump", Pump), ("silo", Silo) })
        {
            ForwardDecision decision = router.Forward(address);
            Console.WriteLine(decision.Action switch
            {
                ForwardAction.Deliver => $"for the {name,-8} deliver here",
                ForwardAction.Relay => $"for the {name,-8} relay via {decision.NextHop}",
                _ => $"for the {name,-8} flood, no route known",
            });
        }

        // Forgetting a node that has gone quiet returns its traffic to flooding, so
        // routing is an optimisation over flooding rather than a second thing that can
        // fail.
        router.Forget(Pump);
        ForwardDecision after = router.Forward(Pump);
        Console.WriteLine(
            $"pump forgotten, so it floods again: {after.Action == ForwardAction.Flood}");
        // ANCHOR_END: example

        Expect(route?.NextHop == EastRelay, "the pump is reached through the cheaper relay");
        Expect(route?.Cost == 1, "at the cost that relay reported");
        Expect(router.Count == 1, "forgetting drops exactly one route");
        Expect(after.Action == ForwardAction.Flood, "the way to the pump is gone");

        using Router fresh = new(Gateway, 4);
        Expect(fresh.Observe(Pump, NorthRelay, 2), "the first packet heard teaches a route");
        Expect(fresh.Observe(Pump, EastRelay, 1), "a cheaper neighbour redirects it");
        Expect(!fresh.Observe(Pump, SouthRelay, 4), "a costlier way is not worth taking");
        Expect(fresh.Observe(Tank, NorthRelay, 3), "a second node is learned");
        Expect(fresh.Count == 2, "two nodes are known, not four observations");
        Expect(fresh.Forward(Gateway).Action == ForwardAction.Deliver, "a packet for this node");
        Expect(fresh.Forward(Pump).NextHop == EastRelay, "goes to the relay on the way");
        Expect(fresh.Forward(Silo).Action == ForwardAction.Flood, "an unknown node still floods");
    }
}
