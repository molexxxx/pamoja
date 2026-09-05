"""The store-and-forward guide example; see docs/guides/sync.md."""

# ANCHOR: example
import asyncio

from pamoja.core import PamojaError
from pamoja.sync import Store


async def main() -> None:
    # A node with nowhere to send buffers its readings. This queue is held in memory, so it
    # lasts as long as the process; Store.file(dir) is the same queue on disk, which is what
    # a node uses to survive a reboot with its backlog intact.
    outbox = Store.memory()
    for reading in (b"20.1", b"20.4", b"20.2"):
        await outbox.append(reading)
    print(f"queued    {await outbox.len()} readings with no link")

    # Peek reads the oldest record without taking it, so a send that fails part-way leaves
    # the queue exactly as it was.
    oldest = await outbox.peek()
    print(f"oldest    {oldest.decode()} and still {await outbox.len()} held")

    # The link returns and the queue drains oldest first, in the order the readings were
    # taken rather than the order they happen to come back off a buffer.
    drained = []
    while (record := await outbox.pop()) is not None:
        drained.append(record.decode())
    print(f"drained   {', '.join(drained)}")

    # A bounded queue refuses the append that would overflow it. A full store is
    # backpressure the caller is told about, not a reading dropped behind its back.
    bounded = Store.memory(capacity=2)
    await bounded.append(b"20.1")
    await bounded.append(b"20.4")
    try:
        await bounded.append(b"20.2")
        print("a full queue took a third reading, which should never happen")
    except PamojaError as error:
        print(f"full      refused the third reading: {error}")

    return oldest, drained, await outbox.len(), await bounded.len()


oldest, drained, left, held = asyncio.run(main())
# ANCHOR_END: example

assert oldest == b"20.1"
assert drained == ["20.1", "20.4", "20.2"]
assert left == 0
assert held == 2
