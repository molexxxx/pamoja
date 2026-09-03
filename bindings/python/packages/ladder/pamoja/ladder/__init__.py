"""Idiomatic transport-ladder facade.

A ladder is the answer to a node with more than one way to reach the network and
no single one that always works: rungs are tried in the order they were added,
cheapest first, and a message no rung accepts goes into a buffer rather than
being lost.
"""

from __future__ import annotations

import enum

from pamoja._native import Ladder

__all__ = ["Delivery", "Ladder"]


class Delivery(str, enum.Enum):
    """What became of a message handed to a ladder."""

    #: A rung took the message and it is on its way.
    SENT = "Sent"
    #: No rung would take it, so it is in the buffer awaiting a flush.
    BUFFERED = "Buffered"
