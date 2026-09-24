"""The mesh-routing guide example; see docs/guides/routing.md."""

# ANCHOR: example
from pamoja.routing import ForwardAction, Router

# The nodes on this mesh. An address is just a number; naming them is what makes the
# table below read as a map of the site rather than a list of numbers.
GATEWAY = 1
PUMP = 9
TANK = 10
NORTH_RELAY = 5
EAST_RELAY = 7
SOUTH_RELAY = 3
SILO = 32

# A node learns the way to another from traffic it already hears: a packet from the pump
# that arrived through a relay proves that relay is a way back, at the cost the packet
# reports. The table keeps the cheapest way it has heard, and a tie keeps the way in use so
# two equal paths do not flap. Word from the relay already in use is taken even when it is
# worse, which is how a failing link lets a detour win.
router = Router(GATEWAY, 4)
for via, cost in [
    (NORTH_RELAY, 2),
    (EAST_RELAY, 1),
    (SOUTH_RELAY, 4),
    (NORTH_RELAY, 1),
    (EAST_RELAY, 3),
    (NORTH_RELAY, 2),
]:
    changed = router.observe(PUMP, via, cost)
    route = router.route(PUMP)
    outcome = "so the route is" if changed else "and the route stays"
    print(
        f"heard     the pump via {via} at cost {cost}, {outcome} {route.next_hop} "
        f"at cost {route.cost}"
    )

# The table lists what it holds, one route for each node it has heard from.
router.observe(TANK, NORTH_RELAY, 3)
held = [f"to {route.dst} via {route.next_hop} at cost {route.cost}" for route in router.routes()]
print(f"table     {len(router)} routes of {router.capacity}: {', '.join(held)}")

# Every packet gets one of three answers: deliver it here, relay it to the neighbor on the
# way, or flood it because no route is known yet.
for name, address in [("gateway", GATEWAY), ("pump", PUMP), ("silo", SILO)]:
    decision = router.forward(address)
    if decision.action == ForwardAction.DELIVER:
        print(f"{name:<10}deliver here")
    elif decision.action == ForwardAction.RELAY:
        print(f"{name:<10}relay via {decision.next_hop}")
    else:
        print(f"{name:<10}flood, no route known")

# The table keeps no clock, so a route through a relay that has gone quiet stays until the
# caller forgets it, typically when a relayed packet goes unanswered. Forgetting returns the
# node's traffic to flooding, the answer that always works.
router.forget(PUMP)
if router.forward(PUMP).action == ForwardAction.FLOOD:
    print(f"forgot    the pump, so it floods again, and {len(router)} route is left")
# ANCHOR_END: example

assert len(router) == 1
assert router.next_hop(TANK) == NORTH_RELAY

# ANCHOR: limits
# A table has a fixed number of slots. Once they are full, a cheaper route takes the slot of
# the costliest one held and a costlier route is refused, so a small table keeps the nodes
# nearest to it and floods to the rest.
WELL = 11
GATE = 12
small = Router(GATEWAY, 2)
small.observe(TANK, NORTH_RELAY, 3)
small.observe(SILO, SOUTH_RELAY, 5)
kept = [f"to {route.dst} via {route.next_hop} at cost {route.cost}" for route in small.routes()]
print(f"full      {len(small)} routes of {small.capacity}: {', '.join(kept)}")
if small.observe(WELL, EAST_RELAY, 2) and small.route(SILO) is None:
    print("evicted   the well at cost 2 took the slot of the silo, the costliest held")
if not small.observe(GATE, EAST_RELAY, 6) and small.forward(GATE).action == ForwardAction.FLOOD:
    print("refused   the gate at cost 6 costs more than every route held, so it floods")

# A flood echoes, and the gateway hears its own packets come back through the relays. A
# route to the node itself is never learned, whatever it costs.
if not router.observe(GATEWAY, EAST_RELAY, 2):
    print("echo      the gateway's own packet coming back teaches it nothing")

# A table with no slots is flooding with nothing remembered, which a node with no memory to
# spare can still do. A packet for the node itself is still delivered.
none = Router(GATEWAY, 0)
learned = none.observe(PUMP, EAST_RELAY, 1)
if (
    not learned
    and none.forward(PUMP).action == ForwardAction.FLOOD
    and none.forward(GATEWAY).action == ForwardAction.DELIVER
):
    print("no room   a table of 0 learns nothing: the pump floods, and the gateway still delivers")
# ANCHOR_END: limits

assert small.next_hop(WELL) == EAST_RELAY
assert len(none) == 0

# ANCHOR: site
from collections import deque

from pamoja.mesh import DEFAULT_HOP_LIMIT, SeenPackets, broadcast, frame, parse, relayed

# Who hears whom on the site. The gateway hears the three relays, and each relay hears the
# one node beyond it; the pump is out of the gateway's range.
site = {
    GATEWAY: [NORTH_RELAY, EAST_RELAY, SOUTH_RELAY],
    NORTH_RELAY: [GATEWAY, TANK],
    EAST_RELAY: [GATEWAY, PUMP],
    SOUTH_RELAY: [GATEWAY, SILO],
    TANK: [NORTH_RELAY],
    PUMP: [EAST_RELAY],
    SILO: [SOUTH_RELAY],
}


def send(tables, source, first):
    """Send a frame from one node and play out what the site does with it.

    A node that hears a frame for the first time learns the way back to its source, then
    asks its own table what to do: deliver it, relay it to the one neighbor on the way, or
    flood it to every neighbor in range, spending a hop each time it goes on. Every frame
    here starts at the default hop limit, and one heard straight from its source still has
    all of it, so the hops a frame has come are what it has spent, plus one.

    :returns: How many times a radio sent, and the nodes that took the frame.
    """
    seen = {node: SeenPackets(64) for node in site}
    seen[source].record(first.src, first.id)
    sends, reached = 0, []
    on_the_air = deque([(source, first)])
    while on_the_air:
        sender, sent = on_the_air.popleft()
        decision = tables[sender].forward(sent.dst)
        if decision.action == ForwardAction.DELIVER:
            continue
        to = site[sender] if decision.next_hop is None else [decision.next_hop]
        sends += 1
        for node in to:
            heard = parse(sent.bytes)
            if not seen[node].record(heard.src, heard.id):
                continue
            reached.append(node)
            tables[node].observe(heard.src, sender, DEFAULT_HOP_LIMIT - heard.hop_limit + 1)
            onward = relayed(heard.bytes)
            if onward is not None:
                on_the_air.append((node, onward))
    return sends, reached


# The pump floods a reading, and every node that takes it learns the way back.
tables = {node: Router(node, 8) for node in site}
reading = broadcast(PUMP, 1, b"flow=12")
sends, reached = send(tables, PUMP, reading)
print(
    f"flood     the pump's reading took {sends} sends to reach {len(reached)} nodes, "
    "and each learned the way back"
)

# The gateway answers the pump. Each node on the way relays to the one neighbor its table
# names, so the answer reaches only the nodes on the path.
answer = frame(GATEWAY, PUMP, 1, b"run=10min")
routed, path = send(tables, GATEWAY, answer)
print(
    f"routed    the gateway's answer reached only {' and '.join(map(str, path))}, "
    f"in {routed} sends"
)

# The same answer on a site that has learned nothing floods, and every node relays it.
blank = {node: Router(node, 8) for node in site}
flooded, everyone = send(blank, GATEWAY, answer)
print(
    f"flooded   with nothing learned, the same answer reached all {len(everyone)} other nodes "
    f"in {flooded} sends"
)
# ANCHOR_END: site

assert len(reached) == len(site) - 1
assert path == [EAST_RELAY, PUMP]
assert routed < flooded
assert tables[GATEWAY].next_hop(PUMP) == EAST_RELAY
assert tables[TANK].cost(PUMP) == 4
