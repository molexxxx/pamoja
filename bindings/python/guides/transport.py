"""The engine surface guide example; see docs/guides/transport.md."""

# ANCHOR: example
import asyncio

from pamoja.core import Transport
from pamoja.ladder import Delivery, Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store

TOPIC = "sensors/1/temperature"


async def main() -> None:
    # Whatever a link is underneath, MQTT, CoAP, or the in-process broker here, it reaches
    # the rest of the framework through one contract. Anything that takes a link works with
    # any of them, so a node is written once and pointed at whichever link it has.
    broker = LoopbackBroker()
    gateway = broker.link()
    await gateway.connect()
    await gateway.subscribe(TOPIC)

    # The fault injector is itself a link wrapping a link, so it composes anywhere one
    # does. This one fails its next send and passes the rest through.
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.faulty(broker.rung(), 1))
    await ladder.connect()

    # The injected failure lands, so the reading is buffered rather than lost.
    first = await ladder.send(TOPIC, b"20.1")
    print(f"first reading: {first}, {await ladder.buffered()} queued")

    # The next reading joins the back of the queue instead of overtaking it, even though
    # the link would take it now. Order on the wire is the order the readings were taken.
    second = await ladder.send(TOPIC, b"20.4")
    queued = await ladder.buffered()
    print(f"second reading: {second}, {queued} queued")

    # Flushing forwards the backlog oldest first, and the subscriber sees it in order.
    forwarded = await ladder.flush()
    earlier = (await gateway.recv()).payload.decode()
    later = (await gateway.recv()).payload.decode()
    print(f"flush forwarded {forwarded}, gateway saw {earlier} then {later}")

    return first, second, queued, forwarded, await ladder.buffered(), earlier, later


first, second, queued, forwarded, left, earlier, later = asyncio.run(main())
# ANCHOR_END: example

assert first == Delivery.BUFFERED
assert second == Delivery.BUFFERED
assert queued == 2
assert forwarded == 2
assert left == 0
assert earlier == "20.1"
assert later == "20.4"
