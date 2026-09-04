"""The event bus guide example; see docs/guides/bus.md."""

# ANCHOR: example
import asyncio

from pamoja.bus import EventBus


async def main() -> None:
    # A sampler announces a reading and whatever cares about readings picks it up,
    # with neither side holding a reference to the other.
    hub = EventBus(8)
    sampler = await hub.subscribe()
    logger = await hub.subscribe()

    await hub.publish(b"battery.low")
    assert await sampler.next_event() == b"battery.low"
    assert await logger.next_event() == b"battery.low"

    # An endpoint taken later starts from the next event, so it never sees what went
    # out before it existed.
    late = await hub.subscribe()
    await hub.publish(b"link.up")
    assert await late.next_event() == b"link.up"
    assert await sampler.next_event() == b"link.up"

    # The buffer is per endpoint and bounded, so an endpoint further behind than the
    # capacity drops what it missed and resumes with the most recent events.
    slow = EventBus(2)
    reader = await slow.subscribe()
    for count in range(5):
        await slow.publish(bytes([count]))
    assert await reader.next_event() == b"\x03"
    assert await reader.next_event() == b"\x04"


asyncio.run(main())
# ANCHOR_END: example
