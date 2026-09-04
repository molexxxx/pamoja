"""Idiomatic async MQTT client facade.

Wraps the native :class:`pamoja._native.MqttClient` with a Python-native surface:
awaitable methods, ``async for`` iteration over messages, ``async with``
lifecycle management, a string enum for quality of service, and keyword
construction. It adds ergonomics only; every operation delegates to the Rust
core.
"""

from __future__ import annotations

import enum
from typing import AsyncIterator, Optional, Union

from pamoja._native import MqttClient as _NativeMqttClient
from pamoja._native import MqttMessage

__all__ = ["MqttClient", "MqttMessage", "Qos"]


class Qos(str, enum.Enum):
    """MQTT delivery guarantee, mirroring the protocol's quality-of-service levels."""

    #: Fire and forget; the broker does not acknowledge delivery.
    AT_MOST_ONCE = "AtMostOnce"
    #: Delivered at least once and acknowledged.
    AT_LEAST_ONCE = "AtLeastOnce"
    #: Delivered exactly once via a four-step handshake.
    EXACTLY_ONCE = "ExactlyOnce"


class MqttClient:
    """An MQTT client transport.

    Construct it with broker settings, :meth:`connect`, then :meth:`publish`,
    :meth:`subscribe`, and read inbound messages with :meth:`recv` or by
    iterating the client with ``async for``. The client also works as an async
    context manager, connecting on entry and disconnecting on exit.

    Example::

        async with MqttClient(client_id="sensor-1", host="localhost", port=1883) as client:
            await client.subscribe("sensors/+/temperature")
            await client.publish("sensors/1/temperature", "21.5")
            async for message in client:
                print(message.topic, message.payload.decode())
    """

    def __init__(
        self,
        *,
        client_id: str,
        host: str,
        port: int,
        keep_alive_secs: Optional[int] = None,
        capacity: Optional[int] = None,
        qos: Optional[Qos] = None,
    ) -> None:
        """Create a disconnected client from the given broker settings.

        :param client_id: The MQTT client identifier presented to the broker.
        :param host: The broker hostname or IP address.
        :param port: The broker TCP port, conventionally 1883 for plaintext MQTT.
        :param keep_alive_secs: Keep-alive interval in seconds. Defaults to 30.
        :param capacity: Bound on outstanding client requests. Defaults to 64.
        :param qos: Default quality of service. Defaults to ``Qos.AT_LEAST_ONCE``.
        """
        qos_value = qos.value if isinstance(qos, Qos) else qos
        self._native = _NativeMqttClient(
            client_id=client_id,
            host=host,
            port=port,
            keep_alive_secs=keep_alive_secs,
            capacity=capacity,
            qos=qos_value,
        )

    async def connect(self) -> None:
        """Connect to the broker and start the background event loop.

        :raises PamojaError: If the connection cannot be established.
        """
        await self._native.connect()

    async def publish(self, topic: str, payload: Union[str, bytes]) -> None:
        """Publish a payload to a topic.

        :param topic: The destination topic.
        :param payload: The message body; ``str`` payloads are encoded as UTF-8.
        """
        data = payload.encode("utf-8") if isinstance(payload, str) else bytes(payload)
        await self._native.publish(topic, data)

    async def subscribe(self, topic: str) -> None:
        """Subscribe to a topic filter.

        :param topic: The topic or wildcard filter to subscribe to.
        """
        await self._native.subscribe(topic)

    async def recv(self) -> Optional[MqttMessage]:
        """Await the next message from any subscribed topic.

        :returns: The next message, or ``None`` once the connection has ended.
        """
        return await self._native.recv()

    async def is_connected(self) -> bool:
        """Report whether the client currently holds an active connection."""
        return await self._native.is_connected()

    async def disconnect(self) -> None:
        """Close the connection and stop the background event loop."""
        await self._native.disconnect()

    async def messages(self) -> AsyncIterator[MqttMessage]:
        """Yield messages from subscribed topics until the connection ends."""
        while True:
            message = await self._native.recv()
            if message is None:
                return
            yield message

    def __aiter__(self) -> AsyncIterator[MqttMessage]:
        """Iterate incoming messages, so a client can be used with ``async for``."""
        return self.messages()

    async def __aenter__(self) -> "MqttClient":
        await self.connect()
        return self

    async def __aexit__(self, *_exc: object) -> None:
        await self.disconnect()
