"""Idiomatic mesh-routing facade.

Flooding always works but costs every node airtime and power on every packet.
Once a mesh has settled, most traffic goes to a few known places, and a node that
remembers the way can forward to one neighbour instead of shouting at the whole
network. Routing is that optimisation, and it falls back to flooding rather than
failing whenever it does not know the way.
"""

from __future__ import annotations

import enum

from pamoja._native import ForwardDecision, Route, Router
from pamoja._native import routing_default_capacity as _default_capacity

__all__ = [
    "DEFAULT_CAPACITY",
    "ForwardAction",
    "ForwardDecision",
    "Route",
    "Router",
    "router",
]

#: A routing table size for a caller with no reason to choose one.
DEFAULT_CAPACITY = _default_capacity()


class ForwardAction(str, enum.Enum):
    """What to do with a packet bound for a given node."""

    #: The packet is for this node; hand it to the application.
    DELIVER = "Deliver"
    #: A route is known; unicast the packet to the next hop reported alongside.
    RELAY = "Relay"
    #: No route is known; fall back to flooding the packet.
    FLOOD = "Flood"


def router(address: int, capacity: int = DEFAULT_CAPACITY) -> Router:
    """Create an empty routing table for a node.

    :param address: The address of this node, which is what a routing decision
        recognises as a local delivery.
    :param capacity: How many routes to make room for. A capacity of zero floods
        every unknown destination, which is the behaviour with no table at all.
    :returns: The routing table, ready to learn from the traffic the node hears.
    """
    return Router(address, capacity)
