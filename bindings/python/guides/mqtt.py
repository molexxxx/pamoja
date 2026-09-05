"""The MQTT guide example; see docs/guides/mqtt.md."""

# ANCHOR: example
import asyncio

from pamoja.core import PamojaError
from pamoja.mqtt import MqttClient, Qos

# The broker on the site. The guide's CI runs one on localhost; point these at yours and
# nothing else changes.
BROKER = "127.0.0.1"
PORT = 1883


async def main() -> None:
    # The gateway takes every temperature on the site. A `+` stands for exactly one level,
    # so this matches every node's temperature and nothing deeper.
    gateway = MqttClient(
        client_id="site-gateway", host=BROKER, port=PORT, qos=Qos.AT_LEAST_ONCE
    )
    await gateway.connect()
    await gateway.subscribe("sensors/+/temperature")
    print("gateway   subscribed to sensors/+/temperature")

    # A node publishes under that pattern. At-least-once means the broker acknowledges the
    # message, so a node knows its reading was taken rather than hoping.
    node = MqttClient(client_id="node-1", host=BROKER, port=PORT, qos=Qos.AT_LEAST_ONCE)
    await node.connect()
    await node.publish("sensors/1/temperature", "21.5")
    print("node      published 21.5 to sensors/1/temperature")

    # The gateway receives it with the topic attached, which is how it knows which node
    # sent the reading without the payload having to repeat it.
    received = await gateway.recv()
    print(f"gateway   got {received.payload.decode()} on {received.topic}")

    # Disconnecting leaves the client reusable, so a node that loses its link can
    # reconnect the same object when the broker comes back.
    await node.disconnect()
    print(f"node      disconnected, still connected: {await node.is_connected()}")
    await gateway.disconnect()

    # A broker that is not there is reported rather than leaving a client that looks
    # connected, so a retry loop has something to test.
    nowhere = MqttClient(client_id="node-2", host=BROKER, port=1, keep_alive_secs=1)
    try:
        await nowhere.connect()
        print("an unreachable broker accepted a connection, which should never happen")
    except PamojaError as error:
        print(f"unreachable broker refused: {error}")

    return received


received = asyncio.run(main())
# ANCHOR_END: example

assert received.topic == "sensors/1/temperature"
assert received.payload == b"21.5"
