"""A part pamoja has never heard of: a soil probe and a valve of the maker's own, run
against a rule and published with nothing plugged in."""

# ANCHOR: parts
import asyncio

from pamoja.core import Transport
from pamoja.kit import Calibration, Thermostat
from pamoja.ladder import Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sim import Replay
from pamoja.sync import Store


class SoilProbe:
    """A capacitive soil probe on an analog-to-digital converter.

    Whatever reads the chip is ``adc``: anything with a ``read`` that hands back counts,
    so a replay stands in for it here and the converter's own driver does on the node.
    The probe turns counts into percent, and nothing downstream needs to know there was
    a chip at all.
    """

    def __init__(self, adc, calibration: Calibration) -> None:
        self.adc = adc
        self.calibration = calibration

    async def read(self) -> float:
        return self.calibration.apply(await self.adc.read())


class Valve:
    """A solenoid valve on a relay.

    It keeps what it was last told and counts the changes, which is what a test needs
    and what a real one does before it drives the pin.
    """

    def __init__(self) -> None:
        self.open = False
        self.switched = 0

    async def apply(self, open: bool) -> None:
        if open != self.open:
            self.open = open
            self.switched += 1


# ANCHOR_END: parts

# ANCHOR: example
TOPIC = "garden/bed-1/moisture"


async def main():
    # Bone dry read 3200 counts on this probe and a soaked pot read 1400, measured once
    # and kept. The replay hands back the counts a bed reads as it dries and is watered.
    counts = Replay([2900.0, 2700.0, 2300.0, 2450.0, 2750.0, 2500.0])
    probe = SoilProbe(counts, Calibration.two_point(3200.0, 0.0, 1400.0, 100.0))
    valve = Valve()

    # Water below 30% and stop above 45%. A valve that adds water is what `heating`
    # names, so the band sits at 37.5 with 7.5 either side.
    rule = Thermostat.heating(37.5, 7.5)

    # The link, with its first two sends failing the way a radio does at dusk. The ladder
    # keeps what it could not send and replays it in order once a send goes through.
    broker = LoopbackBroker()
    gateway = broker.link()
    await gateway.connect()
    await gateway.subscribe(TOPIC)
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.faulty(broker.rung(), 2))
    await ladder.connect()

    # The loop: read, decide, act, publish. Nothing in it knows the probe is homemade.
    for _ in range(6):
        moisture = await probe.read()
        await valve.apply(rule.update(moisture))
        report = f"{moisture:.1f}"
        delivery = await ladder.send(TOPIC, report.encode())
        state = "open" if valve.open else "closed"
        print(f"bed at {report}%, valve {state}, {delivery}")
        if await ladder.buffered() > 0:
            caught_up = await ladder.flush()
            if caught_up > 0:
                print(f"link back, {caught_up} readings caught up")
    print(f"the valve switched {valve.switched} times")

    # On the gateway, in the order they were read, outage included.
    got = [(await gateway.recv()).payload.decode() for _ in range(6)]
    print(f"gateway got {', '.join(got)}")
    return got, valve, await ladder.buffered()


got, valve, left = asyncio.run(main())
# ANCHOR_END: example

assert got == ["16.7", "27.8", "50.0", "41.7", "25.0", "38.9"]
assert valve.switched == 3
assert valve.open
assert left == 0
