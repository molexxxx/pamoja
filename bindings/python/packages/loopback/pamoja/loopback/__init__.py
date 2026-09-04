"""Idiomatic loopback-broker facade.

An in-process broker: publish on one link, receive on another, with no broker
process, no network, and no hardware. It is what makes a message flow testable
from a unit test rather than only from a deployment.
"""

from __future__ import annotations

from pamoja._native import LoopbackBroker, LoopbackTransport, Message

__all__ = [
    "LoopbackBroker",
    "LoopbackTransport",
    "Message",
]
