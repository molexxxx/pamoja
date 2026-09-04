"""The mesh-routing guide example; see docs/guides/routing.md."""

# ANCHOR: example
from pamoja.routing import ForwardAction, Router

# A node learns the way to another from traffic it already hears: a packet from 0x09
# that arrived through neighbour 0x05 proves 0x05 is the way back, at the cost the
# packet reports.
router = Router(0x01, 4)
assert router.observe(0x09, 0x05, 2)

# The table keeps the cheapest way it knows to each node, so the report of cost 1 takes
# over and the later cost-4 report changes nothing.
assert router.observe(0x09, 0x07, 1)
assert not router.observe(0x09, 0x03, 4)
assert router.observe(0x0A, 0x05, 3)
route = router.route(0x09)
assert route.next_hop == 0x07
assert route.cost == 1
assert len(router) == 2

# A packet gets one of three answers: deliver it here, relay it to the neighbour on the
# way, or flood it because no route is known yet.
assert router.forward(0x01).action == ForwardAction.DELIVER
assert router.forward(0x09).action == ForwardAction.RELAY
assert router.forward(0x09).next_hop == 0x07
assert router.forward(0x20).action == ForwardAction.FLOOD

# Forgetting a node that has gone quiet returns its traffic to flooding, so routing is
# an optimisation over flooding rather than a second thing that can fail.
router.forget(0x09)
assert router.forward(0x09).action == ForwardAction.FLOOD
assert len(router) == 1
# ANCHOR_END: example
