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

    await hub.publish("battery.low")
    to_control = await control.next_text()
    to_logger = await logger.next_text()
    print(f"control saw {to_control}, the logger saw {to_logger}")

    # A subscriber taken later starts from the next event, so it never sees what went out
    # before it existed.
    late = await hub.subscribe()
    await hub.publish("link.up")
    first_seen = await late.next_text()
    print(f"the late subscriber's first event is {first_seen}")

    # The buffer is per subscriber and bounded, so one further behind than the capacity
    # drops what it missed and resumes with the most recent events. A slow reader costs
    # itself, not the publisher.
    slow = EventBus(2)
    reader = await slow.subscribe()
    for count in range(5):
        await slow.publish(str(count))
    resumed = await reader.next_text()
    print(f"after five events into a buffer of two, the reader resumes at {resumed}")

    return to_control, to_logger, first_seen, resumed


to_control, to_logger, first_seen, resumed = asyncio.run(main())
# ANCHOR_END: example

assert to_control == "battery.low"
assert to_logger == "battery.low"
assert first_seen == "link.up"
assert resumed == "3"
