"""The pamoja engine's surface: the runtime version, the error every native call
raises, and the transport every link shares.

This is the counterpart of the ``pamoja-core`` crate, and like it, it is small; the
compiled engine is ``pamoja-native``, which this package depends on. Each capability
is its own package (``pamoja-mqtt`` gives ``pamoja.mqtt``, and so on), and
``pamoja`` installs all of them.

A ladder rung, a fault injector, and a degraded link all take some transport.
Python has no way to say "any transport", so one class holds whichever kind was
built and dispatches to it. Composing consumes a transport, because the thing it
is composed into owns it from then on; a consumed transport is emptied rather
than left aliasing what now belongs to a ladder, so using one twice raises.
"""

from __future__ import annotations

from typing import Awaitable, Protocol, Union

from pamoja._native import Message, PamojaError, version
from pamoja._native import PyTransport as Transport

__all__ = [
    "Message",
    "PamojaError",
    "ReceivingTransportHandlers",
    "Transport",
    "TransportHandlers",
    "version",
]


class TransportHandlers(Protocol):
    """The methods a link written in Python supplies to stand as a transport.

    Implement them for a link pamoja does not ship, a vendor SDK, a proprietary
    radio, or a cloud client, and hand the object to ``Transport.from_handlers``.
    Each may be a coroutine function or a plain one. One that raises fails the call
    that reached it, with what it raised.
    """

    def connect(self) -> Union[Awaitable[None], None]:
        """Establish the link. Called again on every reconnect."""

    def send(self, topic: str, payload: bytes) -> Union[Awaitable[None], None]:
        """Publish a payload to a topic."""

    def subscribe(self, topic: str) -> Union[Awaitable[None], None]:
        """Subscribe to a topic filter. A ladder places its filters again on every reconnect."""


class ReceivingTransportHandlers(TransportHandlers, Protocol):
    """A link that also delivers what it subscribed to.

    ``recv`` is called again as soon as it returns, from the moment the transport
    connects, and what it delivers is queued for the receiving side. One that raises
    ends the link until the next connect, and the next receive raises what it raised.
    """

    def recv(
        self,
    ) -> Union[
        Awaitable[Union[Message, tuple[str, Union[str, bytes]], None]],
        Message,
        tuple[str, Union[str, bytes]],
        None,
    ]:
        """Wait for the next message: a ``Message``, a ``(topic, payload)`` pair, or
        ``None`` once the link has ended."""
