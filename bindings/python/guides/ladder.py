"""The transport ladder guide example; see docs/guides/ladder.md."""

# ANCHOR: example
import asyncio

from pamoja.core import Transport
from pamoja.ladder import Delivery, Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store

TOPIC = "sensors/1/temperature"


async def main() -> None:
    # Two links off the same node: a near mesh hop and a metered backhaul. Each has its
    # own broker, so which rung carried a reading is visible from its subscriber.
    mesh = LoopbackBroker()
    backhaul = LoopbackBroker()
    gateway = backhaul.link()
    await gateway.connect()
    await gateway.subscribe(TOPIC)

    # Rungs are tried in the order they are added, cheapest first. The mesh hop loses
    # every packet here; the backhaul carries one send, then drops the next two.
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.degraded(mesh.rung(), drop_every=1))
    await ladder.rung(Transport.degraded(backhaul.rung(), up=1, down=2))
    await ladder.connect()

    # The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
    # broker only that rung publishes to.
    first = await ladder.send(TOPIC, b"21.5")
    arrived = await gateway.recv()
    print(f"first reading: {first}, gateway got {arrived.payload.decode()}")

    # Now nothing will take a send, so the next reading is buffered rather than lost.
    second = await ladder.send(TOPIC, b"21.6")
    waiting = await ladder.buffered()
    print(f"second reading: {second}, {waiting} waiting in the queue")

    # A flush while the links are still down forwards nothing and leaves the backlog
    # intact, because a record is removed only once a rung has accepted it.
    while_down = await ladder.flush()
    print(f"flush while down forwarded {while_down}, queue still {await ladder.buffered()}")

    # The backhaul is reachable again, so the buffered reading goes out exactly once.
    when_up = await ladder.flush()
    late = await gateway.recv()
    print(f"flush when up forwarded {when_up}, gateway got {late.payload.decode()}")

    return first, second, waiting, while_down, when_up, await ladder.buffered(), late


first, second, waiting, while_down, when_up, left, late = asyncio.run(main())
# ANCHOR_END: example

assert first == Delivery.SENT
assert second == Delivery.BUFFERED
assert waiting == 1
assert while_down == 0
assert when_up == 1
assert left == 0
assert late.payload == b"21.6"
