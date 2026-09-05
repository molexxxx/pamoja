"""The in-process broker guide example; see docs/guides/loopback.md."""

# ANCHOR: example
import asyncio

from pamoja.core import PamojaError
from pamoja.loopback import LoopbackBroker


async def main() -> None:
    # One broker and two links off it, all in this process. Nothing binds a port and
    # nothing has to be running for the traffic below to flow, which is what makes this
    # the link to develop a node against before it has a real one.
    broker = LoopbackBroker()
    publisher = broker.link()
    subscriber = broker.link()
    await publisher.connect()
    await subscriber.connect()

    # A `+` stands for exactly one level, so this takes the mixer's temperature but not the
    # raw reading a level below it.
    await subscriber.subscribe("line/+/temp")
    await publisher.send("line/mixer/temp/raw", b"2150")
    await publisher.send("line/mixer/temp", b"21.5")

    message = await subscriber.recv()
    print(f"line/+/temp took {message.payload.decode()} from {message.topic}")

    # A `#` covers every level that remains, so a second link takes the whole subtree,
    # including the reading the single-level filter passed over.
    watcher = broker.link()
    await watcher.connect()
    await watcher.subscribe("line/#")
    await publisher.send("line/mixer/temp/raw", b"2150")

    deep = await watcher.recv()
    print(f"line/#     took {deep.payload.decode()} from {deep.topic}")

    # A link that has been disconnected reports the failure instead of dropping the
    # reading, which is the case a test wants to reach without unplugging anything.
    await publisher.disconnect()
    try:
        await publisher.send("line/mixer/temp", b"21.6")
        print("a disconnected link took a reading, which should never happen")
    except PamojaError as error:
        print(f"disconnected refused the reading: {error}")

    return message, deep


message, deep = asyncio.run(main())
# ANCHOR_END: example

assert message.topic == "line/mixer/temp"
assert message.payload == b"21.5"
assert deep.topic == "line/mixer/temp/raw"
assert deep.payload == b"2150"
