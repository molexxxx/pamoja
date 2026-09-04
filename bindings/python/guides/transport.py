"""The engine surface guide example; see docs/guides/transport.md."""

# ANCHOR: example
import asyncio

from pamoja.core import Transport
from pamoja.ladder import Delivery, Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store


async def main() -> None:
    # Whatever a link is underneath, MQTT, CoAP, or the in-process broker below, it
    # reaches the rest of the framework as one Transport. Anything that takes a link
    # takes that, so a node is written once and pointed at whichever link it has.
    broker = LoopbackBroker()
    gateway = broker.link()
    await gateway.connect()
    await gateway.subscribe("sensors/1/temperature")

    # The fault injector is a Transport wrapping a Transport, so it composes anywhere a
    # link does. This one fails its next send and passes everything after through.
    flaky = Transport.faulty(broker.rung(), 1)
    assert flaky.is_available

    # Composing consumes the transport, because whatever it was composed into owns it
    # from here. The handle is emptied rather than left aliasing what now belongs to
    # something else, so it cannot be sent on twice.
    ladder = Ladder(Store.memory())
    await ladder.rung(flaky)
    assert not flaky.is_available
    await ladder.connect()

    # The injected failure lands, so the reading is buffered rather than lost.
    assert await ladder.send("sensors/1/temperature", b"20.1") == Delivery.BUFFERED
    assert await ladder.buffered() == 1

    # The next reading joins the back of the queue instead of overtaking it, even though
    # the link would take it now. Order on the wire is the order the readings were taken.
    assert await ladder.send("sensors/1/temperature", b"20.4") == Delivery.BUFFERED
    assert await ladder.buffered() == 2

    # Flushing forwards the backlog oldest first, and the subscriber sees it in order.
    assert await ladder.flush() == 2
    assert await ladder.buffered() == 0
    assert (await gateway.recv()).payload == b"20.1"
    assert (await gateway.recv()).payload == b"20.4"


asyncio.run(main())
# ANCHOR_END: example
