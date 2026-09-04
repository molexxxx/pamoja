"""The transport ladder guide example; see docs/guides/ladder.md."""

# ANCHOR: example
import asyncio

from pamoja.core import Transport
from pamoja.ladder import Delivery, Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store


async def main() -> None:
    # Two links off the same node: a near mesh hop and a metered backhaul. Each is a
    # separate broker, so which one carried a reading is visible from its subscriber.
    mesh = LoopbackBroker()
    backhaul = LoopbackBroker()
    gateway = backhaul.link()
    await gateway.connect()
    await gateway.subscribe("sensors/1/temperature")

    # Rungs are tried in the order they are added, cheapest first. The mesh hop loses
    # every packet here; the backhaul carries one send, then drops the next two.
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.degraded(mesh.rung(), drop_every=1))
    await ladder.rung(Transport.degraded(backhaul.rung(), up=1, down=2))
    await ladder.connect()

    # The mesh hop refuses, so the reading goes out over the backhaul and arrives on
    # the broker only that rung publishes to.
    assert await ladder.send("sensors/1/temperature", b"21.5") == Delivery.SENT
    assert (await gateway.recv()).payload == b"21.5"

    # Now nothing will take a send, so the next reading is buffered rather than lost.
    assert await ladder.send("sensors/1/temperature", b"21.6") == Delivery.BUFFERED
    assert await ladder.buffered() == 1

    # A flush while the links are still down forwards nothing and leaves the backlog
    # intact, because a record is removed only once a rung has accepted it.
    assert await ladder.flush() == 0
    assert await ladder.buffered() == 1

    # The backhaul is reachable again, so the buffered reading goes out exactly once.
    assert await ladder.flush() == 1
    assert await ladder.buffered() == 0
    assert (await gateway.recv()).payload == b"21.6"


asyncio.run(main())
# ANCHOR_END: example
