"""The store-and-forward guide example; see docs/guides/sync.md."""

# ANCHOR: example
import asyncio

from pamoja.core import PamojaError
from pamoja.sync import Store


async def main() -> None:
    # A node with nowhere to send buffers its readings. This queue is held in memory, so
    # it lasts as long as the process; Store.file(dir) is the same queue on disk.
    outbox = Store.memory()
    for reading in (b"20.1", b"20.4", b"20.2"):
        await outbox.append(reading)
    assert await outbox.len() == 3

    # Peek reads the oldest record without taking it, so a send that fails part-way
    # leaves the queue exactly as it was.
    assert await outbox.peek() == b"20.1"
    assert await outbox.len() == 3

    # The link returns and the queue drains oldest first, in the order the readings were
    # taken rather than the order they happen to come off a heap.
    drained = []
    while (record := await outbox.pop()) is not None:
        drained.append(record)
    assert drained == [b"20.1", b"20.4", b"20.2"]
    assert await outbox.len() == 0

    # A bounded store refuses the append that would overflow it. A full queue is
    # backpressure the caller is told about, not a reading dropped behind its back.
    bounded = Store.memory(2)
    await bounded.append(b"20.1")
    await bounded.append(b"20.4")
    try:
        await bounded.append(b"20.2")
    except PamojaError:
        pass
    else:
        raise AssertionError("a full store should refuse rather than drop")
    assert await bounded.len() == 2


asyncio.run(main())
# ANCHOR_END: example
