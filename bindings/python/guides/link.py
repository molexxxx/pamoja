"""The own-link guide example; see docs/guides/link.md."""

# ANCHOR: example
import asyncio

from pamoja.core import Message, Transport
from pamoja.ladder import Ladder
from pamoja.sync import Store


class QueueLink:
    """A link over two queues, standing in for a radio or cloud SDK.

    Nothing about it names a broker: it needs only the operations the contract asks
    for, and ``recv`` is what makes it a link that delivers rather than an uplink.
    """

    def __init__(self) -> None:
        self.sent: list[Message] = []
        self.filters: list[str] = []
        self.inbox: asyncio.Queue[Message] = asyncio.Queue()

    async def connect(self) -> None:
        pass

    async def send(self, topic: str, payload: bytes) -> None:
        self.sent.append(Message(topic, payload))

    async def subscribe(self, topic: str) -> None:
        self.filters.append(topic)

    async def recv(self) -> Message:
        return await self.inbox.get()

    def deliver(self, message: Message) -> None:
        """The vendor side: a message arriving from the radio."""
        self.inbox.put_nowait(message)


async def main() -> None:
    # The link is a rung like any shipped transport, and the ladder is the link a
    # node is written against.
    link = QueueLink()
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.from_handlers(link))
    await ladder.connect()

    # A reading out through the ladder lands in the link, topic and bytes intact.
    await ladder.send("sensors/1", b"21.5")
    carried = link.sent[0]
    print(f"link carried: {carried.topic} {carried.payload.decode()}")

    # A subscription placed on the ladder reaches the link.
    await ladder.subscribe("commands/#")
    filter = link.filters[0]
    print(f"link subscribed to: {filter}")

    # What the link delivers comes back through the ladder.
    link.deliver(Message("commands/1", b"open"))
    command = await ladder.recv()
    print(f"command over the ladder: {command.topic} {command.payload.decode()}")

    return carried, filter, command


carried, filter, command = asyncio.run(main())
# ANCHOR_END: example

assert (carried.topic, carried.payload) == ("sensors/1", b"21.5")
assert filter == "commands/#"
assert (command.topic, command.payload) == ("commands/1", b"open")
