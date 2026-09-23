"""The transport ladder guide example; see docs/guides/ladder.md."""

# ANCHOR: example
import asyncio

from pamoja.ladder import Delivery, Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store

REPORT = "vessel/7/report"
ORDERS = "vessel/7/orders"
QUIET_MS = 50


async def main() -> None:
    # Three networks a vessel can reach: the harbor's wifi, the coast's cellular
    # network, and a satellite. Each is a broker with an office ashore listening on it,
    # so which one carried a report is read off that office rather than assumed.
    harbor = LoopbackBroker()
    coast = LoopbackBroker()
    sky = LoopbackBroker()
    harbor_office = harbor.link()
    coast_office = coast.link()
    sky_office = sky.link()
    for office in (harbor_office, coast_office, sky_office):
        await office.connect()
        await office.subscribe(REPORT)

    # Rungs go on cheapest first. The satellite only sends, so it goes on as an uplink,
    # which the ladder never subscribes or listens on.
    ladder = Ladder(Store.memory())
    await ladder.rung(harbor.rung())
    await ladder.rung(coast.rung())
    await ladder.uplink(sky.rung())
    await ladder.connect()
    await ladder.subscribe(ORDERS)

    # In the harbor, the cheapest link takes the report.
    first = await ladder.send(REPORT, "report 1")
    print(f"harbor    carried {(await harbor_office.recv()).text}")

    # Past the breakwater the wifi is out of reach and the report falls through to the
    # coast, and further out to the satellite.
    harbor.reachable = False
    await ladder.send(REPORT, "report 2")
    print(f"coast     carried {(await coast_office.recv()).text}, with the harbor out of reach")
    coast.reachable = False
    await ladder.send(REPORT, "report 3")
    print(f"sky       carried {(await sky_office.recv()).text}, with the coast out of reach too")

    # In a storm nothing is in reach, and the report waits in the store rather than
    # being lost.
    sky.reachable = False
    stormy = await ladder.send(REPORT, "report 4")
    waiting = await ladder.buffered()
    print(f"vessel    buffered report 4 with every link out of reach, {waiting} waiting")

    # A flush with every link still out of reach forwards nothing, because a record
    # leaves the store only once a link has taken it.
    idle = await ladder.flush()
    still = await ladder.buffered()
    print(f"vessel    flushed {idle} while every link was out of reach, {still} still waiting")

    # Back in reach of the coast, a flush sends the backlog, oldest first.
    coast.reachable = True
    forwarded = await ladder.flush()
    late = await coast_office.recv()
    left = await ladder.buffered()
    print(f"coast     carried {late.text} on a flush of {forwarded}, {left} waiting")

    # Orders from shore come back over whichever listening link is in reach.
    await coast_office.send(ORDERS, "return to port")
    order = await ladder.recv()
    print(f"vessel    took {order.text} over the coast network")

    # A ladder does one thing at a time, so a vessel that listens and reports waits
    # for orders with a limit and reports between waits.
    try:
        await asyncio.wait_for(ladder.recv(), QUIET_MS / 1000)
    except asyncio.TimeoutError:
        print(f"vessel    heard nothing more from shore within {QUIET_MS} ms")
    await ladder.send(REPORT, "report 5")
    last = await coast_office.recv()
    print(f"coast     carried {last.text} between waits")

    return first, stormy, (waiting, still, left), order.text, last.text


first, stormy, counts, order, last = asyncio.run(main())
# ANCHOR_END: example

assert first == Delivery.SENT
assert stormy == Delivery.BUFFERED
assert counts == (1, 1, 0)
assert order == "return to port"
assert last == "report 5"
