"""Idiomatic mesh-routing facade.

Flooding always works but costs every node airtime and power on every packet.
Once a mesh has settled, most traffic goes to a few known places, and a node that
remembers the way can forward to one neighbour instead of shouting at the whole
network. Routing is that optimisation, and it falls back to flooding rather than
failing whenever it does not know the way.
"""

from __future__ import annotations

import enum

from ._core import ForwardDecision, Route, Router
from ._core import routing_table_capacity as _table_capacity

__all__ = [
    "TABLE_CAPACITY",
    "ForwardAction",
    "ForwardDecision",
    "Route",
    "Router",
    "router",
]

#: The number of routes a routing table holds.
TABLE_CAPACITY = _table_capacity()


class ForwardAction(str, enum.Enum):
    """What to do with a packet bound for a given node."""

    #: The packet is for this node; hand it to the application.
    DELIVER = "Deliver"
    #: A route is known; unicast the packet to the next hop reported alongside.
    RELAY = "Relay"
    #: No route is known; fall back to flooding the packet.
    FLOOD = "Flood"


def router(address: int) -> Router:
    """Create an empty routing table for a node.

    :param address: The address of this node, which is what a routing decision
        recognises as a local delivery.
    :returns: The routing table, ready to learn from the traffic the node hears.
    """
    return Router(address)
