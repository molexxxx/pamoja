"""The event bus guide example; see docs/guides/bus.md."""

# ANCHOR: example
import asyncio

from pamoja.bus import EventBus


async def main() -> None:
    # A sampler announces something and whatever cares picks it up, with neither side
    # holding a reference to the other. This is how the parts of one node are wired.
    hub = EventBus(8)
    control = await hub.subscribe()
    logger = await hub.subscribe()

    await hub.publish(b"battery.low")
    to_control = await control.next_event()
    to_logger = await logger.next_event()
    print(f"control saw {to_control.decode()}, the logger saw {to_logger.decode()}")

    # A subscriber taken later starts from the next event, so it never sees what went out
    # before it existed.
    late = await hub.subscribe()
    await hub.publish(b"link.up")
    first_seen = await late.next_event()
    print(f"the late subscriber's first event is {first_seen.decode()}")

    # The buffer is per subscriber and bounded, so one further behind than the capacity
    # drops what it missed and resumes with the most recent events. A slow reader costs
    # itself, not the publisher.
    slow = EventBus(2)
    reader = await slow.subscribe()
    for count in range(5):
        await slow.publish(bytes([count]))
    resumed = await reader.next_event()
    print(f"after five events into a buffer of two, the reader resumes at {resumed[0]}")

    return to_control, to_logger, first_seen, resumed


to_control, to_logger, first_seen, resumed = asyncio.run(main())
# ANCHOR_END: example

assert to_control == b"battery.low"
assert to_logger == b"battery.low"
assert first_seen == b"link.up"
assert resumed == bytes([3])
