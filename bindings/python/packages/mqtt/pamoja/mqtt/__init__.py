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
from pamoja._native import MqttMessage, MqttTls, MqttWill

__all__ = ["MqttClient", "MqttMessage", "MqttTls", "MqttWill", "Qos"]


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
        max_packet_size: Optional[int] = None,
        username: Optional[str] = None,
        password: Optional[str] = None,
        will: Optional[MqttWill] = None,
        tls: Optional[MqttTls] = None,
    ) -> None:
        """Create a disconnected client from the given broker settings.

        :param client_id: The MQTT client identifier presented to the broker.
        :param host: The broker hostname or IP address.
        :param port: The broker TCP port, conventionally 1883 for plaintext MQTT.
        :param keep_alive_secs: Keep-alive interval in seconds. Defaults to 30.
        :param capacity: Bound on outstanding client requests. Defaults to 64.
        :param qos: Default quality of service. Defaults to ``Qos.AT_LEAST_ONCE``.
        :param max_packet_size: The largest packet the connection sends or accepts, in
            bytes. Defaults to 10,240. A publish that would be larger is refused and the
            connection stays up, but a larger packet arriving from the broker ends the
            connection, so every client that shares a topic needs a limit that fits it.
        :param username: The name to sign in to the broker with.
        :param password: The password to sign in with, which needs a username. It travels
            in the clear unless the connection uses ``tls``.
        :param will: A message the broker publishes if the connection ends without a
            goodbye.
        :param tls: TLS settings; a connection with them is secured, conventionally on
            port 8883.
        :raises PamojaError: If a password comes without a username.
        """
        qos_value = qos.value if isinstance(qos, Qos) else qos
        self._native = _NativeMqttClient(
            client_id=client_id,
            host=host,
            port=port,
            keep_alive_secs=keep_alive_secs,
            capacity=capacity,
            qos=qos_value,
            max_packet_size=max_packet_size,
            username=username,
            password=password,
            will=will,
            tls=tls,
        )

    async def connect(self) -> None:
        """Connect to the broker and start the background event loop.

        :raises PamojaError: If the connection cannot be established.
        """
        await self._native.connect()

    async def publish(
        self,
        topic: str,
        payload: Union[str, bytes],
        *,
        qos: Optional[Qos] = None,
        retain: bool = False,
    ) -> None:
        """Publish a payload to a topic, returning once it is queued for the broker.

        This returns before the broker acknowledges the message;
        :meth:`publish_confirmed` waits for that.

        :param topic: The destination topic.
        :param payload: The message body; ``str`` payloads are encoded as UTF-8.
        :param qos: The quality of service for this message. Defaults to the client's.
        :param retain: Whether the broker keeps the message for clients that subscribe
            later. An empty retained message clears the one the broker holds.
        """
        data = payload.encode("utf-8") if isinstance(payload, str) else bytes(payload)
        qos_value = qos.value if isinstance(qos, Qos) else qos
        await self._native.publish(topic, data, qos=qos_value, retain=retain)

    async def send(self, topic: str, payload: Union[str, bytes]) -> None:
        """Publish a payload to a topic at the client's default quality of service.

        This is the ``send`` every link has, so a profile's :class:`~pamoja.profile.Node`
        or a :class:`~pamoja.profile.RuleEngine` publishes over an MQTT client the way it
        publishes over any other link.

        :param topic: The destination topic.
        :param payload: The message body; ``str`` payloads are encoded as UTF-8.
        """
        await self.publish(topic, payload)

    async def publish_confirmed(
        self,
        topic: str,
        payload: Union[str, bytes],
        *,
        qos: Optional[Qos] = None,
        retain: bool = False,
    ) -> None:
        """Publish a payload to a topic and wait for the broker to acknowledge it.

        The broker answers with a ``PUBACK`` at ``Qos.AT_LEAST_ONCE`` and a ``PUBCOMP``
        at ``Qos.EXACTLY_ONCE``; at ``Qos.AT_MOST_ONCE`` MQTT acknowledges nothing, so
        this returns once the connection has taken the message.

        :param topic: The destination topic.
        :param payload: The message body; ``str`` payloads are encoded as UTF-8.
        :param qos: The quality of service for this message. Defaults to the client's.
        :param retain: Whether the broker keeps the message for clients that subscribe
            later.
        :raises PamojaError: If the connection ends before the acknowledgment, when the
            message may or may not have arrived.
        """
        data = payload.encode("utf-8") if isinstance(payload, str) else bytes(payload)
        qos_value = qos.value if isinstance(qos, Qos) else qos
        await self._native.publish_confirmed(topic, data, qos=qos_value, retain=retain)

    async def subscribe(self, topic: str) -> None:
        """Subscribe to a topic filter.

        :param topic: The topic or wildcard filter to subscribe to.
        """
        await self._native.subscribe(topic)

    async def recv(self) -> Optional[MqttMessage]:
        """Await the next message from any subscribed topic.

        Wrap it in :func:`asyncio.wait_for` to stop waiting; the receive it cancels
        leaves its message queued for the next one.

        :returns: The next message, or ``None`` once the connection has ended.
        :raises PamojaError: Once, saying why, when the connection ended on its own,
            because the broker went away or another client connected with the same id.
        """
        return await self._native.recv()

    async def is_connected(self) -> bool:
        """Report whether the client currently holds an active connection."""
        return await self._native.is_connected()

    async def disconnect(self) -> None:
        """Close the connection and stop the background event loop."""
        await self._native.disconnect()

    async def messages(self) -> AsyncIterator[MqttMessage]:
        """Yield messages from subscribed topics until the connection ends.

        A connection that ends on its own raises its reason out of the loop.
        """
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
