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
# that arrived through the north relay proves that relay is a way back, at the cost the
# packet reports.
router = Router(GATEWAY, 4)
router.observe(PUMP, NORTH_RELAY, 2)

# The table keeps only the cheapest way it knows to each node, so a cost-1 report through
# the east relay takes over and the later cost-4 report changes nothing.
router.observe(PUMP, EAST_RELAY, 1)
router.observe(PUMP, SOUTH_RELAY, 4)
router.observe(TANK, NORTH_RELAY, 3)

route = router.route(PUMP)
print(f"to the pump   via {route.next_hop} at cost {route.cost}")
print(f"routes held   {len(router)}")

# Every packet gets one of three answers: deliver it here, relay it to the neighbour on
# the way, or flood it because no route is known yet.
for name, address in [("gateway", GATEWAY), ("pump", PUMP), ("silo", SILO)]:
    decision = router.forward(address)
    if decision.action == ForwardAction.DELIVER:
        print(f"for the {name:<8} deliver here")
    elif decision.action == ForwardAction.RELAY:
        print(f"for the {name:<8} relay via {decision.next_hop}")
    else:
        print(f"for the {name:<8} flood, no route known")

# Forgetting a node that has gone quiet returns its traffic to flooding, so routing is an
# optimisation over flooding rather than a second thing that can fail.
router.forget(PUMP)
after = router.forward(PUMP)
print(f"pump forgotten, so it floods again: {after.action == ForwardAction.FLOOD}")
# ANCHOR_END: example

assert route.next_hop == EAST_RELAY
assert route.cost == 1
assert len(router) == 1
assert after.action == ForwardAction.FLOOD

fresh = Router(GATEWAY, 4)
assert fresh.observe(PUMP, NORTH_RELAY, 2)
assert fresh.observe(PUMP, EAST_RELAY, 1)
assert not fresh.observe(PUMP, SOUTH_RELAY, 4)
assert fresh.observe(TANK, NORTH_RELAY, 3)
assert len(fresh) == 2
assert fresh.forward(GATEWAY).action == ForwardAction.DELIVER
assert fresh.forward(PUMP).action == ForwardAction.RELAY
assert fresh.forward(PUMP).next_hop == EAST_RELAY
assert fresh.forward(SILO).action == ForwardAction.FLOOD
