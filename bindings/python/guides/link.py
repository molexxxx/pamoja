"""The own-link guide example; see docs/guides/link.md."""

# ANCHOR: example
import asyncio
from typing import Union

from pamoja.core import Message, PamojaError, Transport
from pamoja.ladder import Ladder
from pamoja.sync import Store


class Modem:
    """A cellular modem reached through its vendor's SDK, which this class stands in for.

    Nothing in it names a broker or a protocol: it needs only the operations the
    contract asks for, and ``recv`` is what makes it a link that delivers.
    """

    def __init__(self) -> None:
        self.sent: list[Message] = []
        self.filters: list[str] = []
        self.no_signal = False
        self.inbox: asyncio.Queue[Union[Message, Exception]] = asyncio.Queue()

    def connect(self) -> None:
        pass

    def send(self, topic: str, payload: bytes) -> None:
        if self.no_signal:
            raise ConnectionError("no signal")
        self.sent.append(Message(topic, payload))

    def subscribe(self, topic: str) -> None:
        self.filters.append(topic)

    async def recv(self) -> Message:
        arrived = await self.inbox.get()
        if isinstance(arrived, Exception):
            raise arrived
        return arrived

    def hand_over(self, arrived: Union[Message, Exception]) -> None:
        """The vendor's side: a message from the network, or the loss of the session."""
        self.inbox.put_nowait(arrived)


class Satellite:
    """A satellite messenger sends and never receives.

    With no ``recv`` it is an uplink, which a ladder never listens on or subscribes.
    """

    def __init__(self) -> None:
        self.sent: list[Message] = []

    def connect(self) -> None:
        pass

    def send(self, topic: str, payload: bytes) -> None:
        self.sent.append(Message(topic, payload))

    def subscribe(self, topic: str) -> None:
        raise ConnectionError("a satellite messenger only sends")


async def main() -> None:
    # The modem is the cheaper link and goes first. The messenger has no `recv`, so it
    # goes on as an uplink.
    modem = Modem()
    satellite = Satellite()
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.from_handlers(modem))
    await ladder.rung(Transport.from_handlers(satellite))
    await ladder.connect()

    # A reading goes out over the first link that takes it.
    await ladder.send("waves/height", "1.8")
    carried = modem.sent[0]
    print(f"modem     carried {carried.topic} {carried.text}")

    # A subscription reaches every link that listens, and only those: the messenger,
    # whose subscribe would raise, is never asked.
    await ladder.subscribe("commands/#")
    placed = modem.filters[0]
    print(f"modem     listens on {placed}, and the satellite was never asked")

    # What the modem hands over comes back through the ladder.
    modem.hand_over(Message("commands/interval", "600"))
    command = await ladder.recv()
    print(f"buoy      took {command.topic} {command.text}")

    # A link that refuses a send passes the reading down to the next link.
    modem.no_signal = True
    await ladder.send("waves/height", "2.4")
    relayed = satellite.sent[0]
    print(f"satellite carried {relayed.topic} {relayed.text} while the modem had no signal")

    # A link that fails while listening says why, once, and has ended after that. With
    # no other link listening, the ladder then has nothing to wait on.
    modem.hand_over(ConnectionResetError("the modem lost its session"))
    try:
        await ladder.recv()
    except PamojaError as error:
        lost = str(error)
    print(f"buoy      lost the modem: {lost}")
    try:
        await ladder.recv()
    except PamojaError as error:
        idle = str(error)
    print(f"buoy      has no link left to listen on: {idle}")

    # Connecting again brings the modem back, and the ladder places its filter on it
    # again, so a link's subscribe runs once for every connect.
    await ladder.connect()
    print(f"modem     reconnected, and the ladder placed {modem.filters[1]} on it again")
    modem.hand_over(Message("commands/interval", "900"))
    later = await ladder.recv()
    print(f"buoy      took {later.topic} {later.text}")

    return carried, modem.filters, command, relayed, lost, idle, later


seen = asyncio.run(main())
# ANCHOR_END: example

carried, filters, command, relayed, lost, idle, later = seen
assert (carried.topic, carried.payload) == ("waves/height", b"1.8")
assert filters == ["commands/#", "commands/#"]
assert (command.topic, command.payload) == ("commands/interval", b"600")
assert (relayed.topic, relayed.payload) == ("waves/height", b"2.4")
assert lost == "transport error: the modem lost its session"
assert idle == "resource is closed"
assert (later.topic, later.payload) == ("commands/interval", b"900")
