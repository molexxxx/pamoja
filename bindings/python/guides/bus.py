"""The event bus guide example; see docs/guides/bus.md."""

# ANCHOR: example
import asyncio

from pamoja.bus import EventPublisher

QUIET_MS = 50


async def main() -> None:
    # The station's wiring makes one bus and hands each part what it needs: a publisher
    # to announce, an endpoint to listen. No part holds a reference to another, so any
    # of them can be replaced without touching the rest.
    bus = EventPublisher(2)
    power = bus.publisher()
    sampler = bus.publisher()
    heater = bus.subscribe()
    logger = bus.subscribe()

    # One announcement reaches every part that listens, and each reads its own copy.
    reached = power.publish("battery.low")
    print(f"power     handed battery.low to {reached} parts")
    heater_took = await heater.next_text()
    print(f"heater    took {heater_took}")
    logger_took = await logger.next_text()
    print(f"logger    took {logger_took}")

    # Publishing never waits, even while the part's own wait is open, and a part hears
    # what it publishes.
    waiting = heater.next_text()
    heater.publish("heater.off")
    heard = await waiting
    print(f"heater    heard its own {heard}, sent while it waited")

    # A part that joins late sees only what is published after it subscribes. There is
    # no history to replay.
    radio = bus.subscribe()
    power.publish("battery.ok")
    first = await radio.next_text()
    print(f"radio     joined late, so the first event it sees is {first}")

    # Each endpoint buffers two events. The logger, busy writing to flash, falls behind
    # while the sampler publishes five readings: it loses the oldest events, resumes
    # with the newest, and counts what it lost.
    for reading in range(5):
        sampler.publish(f"wind {reading}")
    resumed = await logger.next_text()
    missed = logger.missed
    print(f"logger    missed {missed} and resumes at {resumed}")
    newest = await logger.next_text()
    print(f"logger    then took {newest}")

    # A wait with a limit gives up without taking anything, so a part can do other work
    # between events and lose nothing by it.
    try:
        await asyncio.wait_for(logger.next_text(), QUIET_MS / 1000)
        print("logger    took an event no one published, which should never happen")
    except asyncio.TimeoutError:
        print(f"logger    heard nothing more within {QUIET_MS} ms")

    return reached, heater_took, logger_took, heard, first, missed, resumed, newest


seen = asyncio.run(main())
# ANCHOR_END: example

reached, heater_took, logger_took, heard, first, missed, resumed, newest = seen
assert reached == 2
assert heater_took == "battery.low"
assert logger_took == "battery.low"
assert heard == "heater.off"
assert first == "battery.ok"
assert missed == 5
assert resumed == "wind 3"
assert newest == "wind 4"
