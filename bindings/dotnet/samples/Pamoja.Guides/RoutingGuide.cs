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
        // A node learns the way to another from traffic it already hears: a packet from
        // 0x09 that arrived through neighbour 0x05 proves 0x05 is the way back, at the
        // cost the packet reports.
        using Router router = new(0x01, 4);
        Expect(router.Observe(0x09, 0x05, 2), "the first packet heard from 0x09 teaches a route");

        // The table keeps the cheapest way it knows to each node, so the report of cost 1
        // takes over and the later cost-4 report changes nothing.
        Expect(router.Observe(0x09, 0x07, 1), "a cheaper neighbour redirects the route");
        Expect(!router.Observe(0x09, 0x03, 4), "a costlier way is not worth taking");
        Expect(router.Observe(0x0A, 0x05, 3), "a second node is learned");
        Route? route = router.RouteTo(0x09);
        Expect(route?.NextHop == 0x07, "0x09 is reached through the cheaper neighbour");
        Expect(route?.Cost == 1, "at the cost that neighbour reported");
        Expect(router.Count == 2, "two nodes are known, not four observations");

        // A packet gets one of three answers: deliver it here, relay it to the neighbour
        // on the way, or flood it because no route is known yet.
        Expect(router.Forward(0x01).Action == ForwardAction.Deliver, "a packet for this node");
        ForwardDecision relayed = router.Forward(0x09);
        Expect(relayed.Action == ForwardAction.Relay, "a packet for a node we can reach");
        Expect(relayed.NextHop == 0x07, "goes to the neighbour on the way");
        Expect(router.Forward(0x20).Action == ForwardAction.Flood, "an unknown node still floods");

        // Forgetting a node that has gone quiet returns its traffic to flooding, so
        // routing is an optimisation over flooding rather than a second thing that can
        // fail.
        router.Forget(0x09);
        Expect(router.Forward(0x09).Action == ForwardAction.Flood, "the way to 0x09 is gone");
        Expect(router.Count == 1, "forgetting drops exactly one route");
        // ANCHOR_END: example
    }
}
