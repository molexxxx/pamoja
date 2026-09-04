"""The MQTT guide example; see docs/guides/mqtt.md."""

# ANCHOR: example
import asyncio

from pamoja.core import PamojaError
from pamoja.mqtt import MqttClient, Qos


async def main() -> None:
    # MQTT numbers its three delivery guarantees 0, 1 and 2 on the wire; the binding
    # names them, in that order.
    assert [level.value for level in Qos] == ["AtMostOnce", "AtLeastOnce", "ExactlyOnce"]

    # Nothing listens on this port, so the broker is unreachable. Constructing the client
    # touches nothing; only connecting does.
    client = MqttClient(
        client_id="guide-node",
        host="127.0.0.1",
        port=47811,
        keep_alive_secs=1,
        qos=Qos.EXACTLY_ONCE,
    )
    assert await client.is_connected() is False

    # A refused connection surfaces as a transport error and leaves the client as it was,
    # so the same object can be retried once the broker is back.
    try:
        await client.connect()
    except PamojaError as error:
        assert str(error).startswith("transport error")
    else:
        raise AssertionError("connecting to a closed port should raise")

    assert await client.is_connected() is False


asyncio.run(main())
# ANCHOR_END: example
