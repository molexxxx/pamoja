"""The MQTT guide example; see docs/guides/mqtt.md."""

# ANCHOR: example
import asyncio

from pamoja.core import PamojaError
from pamoja.mqtt import MqttClient, MqttWill, Qos

# The broker on the site. The guide's CI runs one on localhost; point these at yours and
# nothing else changes.
BROKER = "127.0.0.1"
PORT = 1883


def connection(connected: bool) -> str:
    return "still connected" if connected else "not connected"


async def main() -> None:
    # The gateway takes every temperature on the site. A `+` stands for exactly one level,
    # so this matches every node's temperature and nothing deeper.
    gateway = MqttClient(
        client_id="site-gateway", host=BROKER, port=PORT, qos=Qos.AT_LEAST_ONCE
    )
    await gateway.connect()
    await gateway.subscribe("sensors/+/temperature")
    print("gateway   subscribed to sensors/+/temperature")

    # A node publishes under that pattern. At least once has the broker acknowledge each
    # message, where at most once would send it and forget it. It also leaves a will: should
    # it drop off the network without saying goodbye, the broker publishes offline on its
    # status topic for it.
    will = MqttWill("sites/node-1/status", "offline", qos=Qos.AT_LEAST_ONCE, retain=True)
    node = MqttClient(
        client_id="node-1", host=BROKER, port=PORT, qos=Qos.AT_LEAST_ONCE, will=will
    )
    await node.connect()
    await node.publish("sensors/1/temperature", "21.5")
    print("node      published 21.5 to sensors/1/temperature")

    # The gateway receives it with the topic attached, which is how it knows which node
    # sent the reading without the payload having to repeat it.
    received = await gateway.recv()
    print(f"gateway   got {received.text} on {received.topic}")

    # The node says it is up, retained, so a dashboard that opens later sees it at once.
    # `publish_confirmed` returns once the broker acknowledges the message.
    await node.publish_confirmed("sites/node-1/status", "online", retain=True)
    print("node      the broker holds online on sites/node-1/status")

    # A dashboard that subscribes afterwards still gets it, because the broker keeps the
    # last retained message on each topic for whoever subscribes next.
    dashboard = MqttClient(client_id="site-dashboard", host=BROKER, port=PORT)
    await dashboard.connect()
    await dashboard.subscribe("sites/node-1/status")
    status = await dashboard.recv()
    print(f"dashboard {status.topic} is {status.text}")

    # Two days of readings saved at one a minute, sent as one message, make a packet over
    # the connection's 10 KiB limit. The send is refused before anything leaves and the
    # connection stays up; a node that must send it raises the limit on every client that
    # shares the topic, or splits it.
    backlog = ",".join(["21.5"] * (2 * 24 * 60))
    try:
        await node.publish("sensors/1/backlog", backlog)
        print("node      sent an oversized backlog, which should never happen")
    except PamojaError as error:
        print(f"node      backlog refused: {error}")
    after_refusal = await node.is_connected()
    print(f"node      {connection(after_refusal)}")

    # Before it leaves, the node says so itself. A clean disconnect discards the will, which
    # is only for a node that drops off without this goodbye.
    await node.publish_confirmed("sites/node-1/status", "offline", retain=True)
    goodbye = await dashboard.recv()
    print(f"dashboard {goodbye.topic} is {goodbye.text}")

    # Disconnecting leaves the client reusable, so a node that loses its link can
    # reconnect the same object when the broker comes back.
    await node.disconnect()
    after_disconnect = await node.is_connected()
    print(f"node      {connection(after_disconnect)} after disconnecting")
    await gateway.disconnect()
    await dashboard.disconnect()

    # A broker that is not there is reported rather than leaving a client that looks
    # connected, so a retry loop has something to test.
    nowhere = MqttClient(client_id="node-2", host=BROKER, port=1, keep_alive_secs=1)
    try:
        await nowhere.connect()
        print("an unreachable broker accepted a connection, which should never happen")
    except PamojaError as error:
        print(f"unreachable broker refused: {error}")

    return received, status, goodbye, after_refusal, after_disconnect


received, status, goodbye, after_refusal, after_disconnect = asyncio.run(main())
# ANCHOR_END: example

assert received.topic == "sensors/1/temperature"
assert received.payload == b"21.5"
assert status.text == "online"
assert goodbye.text == "offline"
assert after_refusal, "a refused send leaves the connection up"
assert not after_disconnect, "a disconnected client says so"
