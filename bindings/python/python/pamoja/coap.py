"""Idiomatic CoAP facade.

CoAP is the transport for links where MQTT is more than the budget allows: it
runs over UDP, its headers are a handful of bytes, and a node can fire a reading
and forget it rather than holding a session open.
"""

from __future__ import annotations

import enum

from ._core import CoapClient, Message

__all__ = [
    "CoapClient",
    "Message",
    "Reliability",
]


class Reliability(str, enum.Enum):
    """Whether a request is acknowledged and retried."""

    #: Fire and forget: the request is sent once and not acknowledged.
    NON_CONFIRMABLE = "NonConfirmable"
    #: The request is acknowledged, and retransmitted until an ACK arrives.
    CONFIRMABLE = "Confirmable"
