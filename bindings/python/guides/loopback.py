"""The loopback guide example; see docs/guides/loopback.md."""

# ANCHOR: example
import asyncio

from pamoja.core import PamojaError
from pamoja.loopback import LoopbackBroker


async def main() -> None:
    # One broker and two links off it, all in this process. Nothing binds a port and
    # nothing has to be running for the traffic below to flow.
    broker = LoopbackBroker()
    publisher = broker.link()
    subscriber = broker.link()
    await publisher.connect()
    await subscriber.connect()

    # A `+` stands for exactly one level, so the deeper topic is not delivered here and
    # the first message this subscriber sees is the second publish.
    await subscriber.subscribe("sensors/+/temperature")
    await publisher.send("sensors/8/temperature/raw", b"2150")
    await publisher.send("sensors/8/temperature", b"21.5")

    message = await subscriber.recv()
    assert message.topic == "sensors/8/temperature"
    assert message.payload == b"21.5"

    # A `#` covers the levels that remain, so a second link takes the whole subtree,
    # including the reading the single-level filter passed over.
    watcher = broker.link()
    await watcher.connect()
    await watcher.subscribe("sensors/#")
    await publisher.send("sensors/8/temperature/raw", b"2150")

    deep = await watcher.recv()
    assert deep.topic == "sensors/8/temperature/raw"
    assert deep.payload == b"2150"

    # A link that has been disconnected reports the failure instead of dropping the
    # reading, which is the case a test wants to reach without unplugging anything.
    await publisher.disconnect()
    try:
        await publisher.send("sensors/8/temperature", b"21.6")
    except PamojaError:
        pass
    else:
        raise AssertionError("a disconnected link should refuse to publish")


asyncio.run(main())
# ANCHOR_END: example
