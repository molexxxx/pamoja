"""The store-and-forward guide example; see docs/guides/sync.md."""

# ANCHOR: example
import asyncio
import tempfile

from pamoja.core import PamojaError, Transport
from pamoja.loopback import LoopbackBroker
from pamoja.sync import Store

TOPIC = "apiary/hive-3/weight"


async def main(folder: str) -> None:
    # The scale logs its weight to a queue on its SD card, bounded so a long outage
    # cannot fill the card. The directory is the queue, so the scale can lose power at
    # any moment and lose nothing it logged.
    outbox = Store.file(folder, 3)
    for weight in ("41.2", "41.5", "40.9"):
        await outbox.append(weight)
    logged = await outbox.len()
    print(f"hive      logged {logged} weights with no link, the most its store holds")

    # A full store refuses the next weight rather than dropping one it already holds.
    try:
        await outbox.append("41.1")
    except PamojaError as error:
        print(f"hive      was refused a 4th: {error}")

    # The scale reboots. Its queue is the directory, so it comes back whole and in
    # order.
    outbox = Store.file(folder, 3)
    held = await outbox.len()
    oldest = await outbox.peek_text()
    print(f"hive      restarted and still holds {held}, oldest first: {oldest}")

    # The cellular uplink carries one weight, then drops. A weight leaves the queue only
    # once a link has taken it, so what the uplink never took stays, in order.
    cellular = LoopbackBroker()
    uplink = Transport.degraded(cellular.rung(), up=1, down=10)
    await uplink.connect()
    forwarded = 0
    try:
        await outbox.drain_to(uplink, TOPIC)
    except PamojaError as error:
        forwarded = held - await outbox.len()
        print(f"uplink    forwarded {forwarded}, then failed: {error}")
    left = await outbox.len()
    next_weight = await outbox.peek_text()
    print(f"hive      still holds {left}, oldest first: {next_weight}")

    # The beekeeper's gateway comes within reach, and the scale drains the rest onto
    # it.
    visit = LoopbackBroker()
    gateway = visit.link()
    await gateway.connect()
    await gateway.subscribe(TOPIC)
    to_gateway = visit.rung()
    await to_gateway.connect()
    await outbox.drain_to(to_gateway, TOPIC)
    took = [(await gateway.recv()).text for _ in range(left)]
    print(f"gateway   took {', '.join(took)} when the beekeeper came by")
    empty = await outbox.len()
    print(f"hive      holds {empty} once the backlog is through")

    return (logged, held, forwarded, left, empty), oldest, next_weight, took


with tempfile.TemporaryDirectory(prefix="pamoja-hive-") as folder:
    counts, oldest, next_weight, took = asyncio.run(main(folder))
# ANCHOR_END: example

assert counts == (3, 3, 1, 2, 0)
assert oldest == "41.2"
assert next_weight == "41.5"
assert took == ["41.5", "40.9"]
