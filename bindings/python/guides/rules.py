"""The rules guide example; see docs/guides/rules.md."""

# ANCHOR: example
import asyncio
import json

from pamoja.kit import Edge, Trigger
from pamoja.loopback import LoopbackBroker

# A rule is a file: the topic it watches, the line a reading crosses, the release band
# that stops it firing over and over, and what to do on the way down and on the way
# back. The same file runs in every language; here the program reads it as data and
# drives the loop itself, with the kit's trigger deciding the condition.
rules = json.loads("""{ "rules": [ {
  "name": "water-when-dry",
  "when": { "topic": "garden/bed-1/moisture", "compare": "below",
            "threshold": 30.0, "hysteresis": 5.0 },
  "then": [ { "do": "drive", "actuator": "bed-valve", "on": true },
            { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" } ],
  "otherwise": [ { "do": "drive", "actuator": "bed-valve", "on": false },
                 { "do": "publish", "topic": "garden/bed-1/valve", "payload": "closed" } ]
} ] }""")
rule = rules["rules"][0]
when = rule["when"]
clears = when["threshold"] + when["hysteresis"]
print(f"the rule watches {when['topic']} {when['compare']} {when['threshold']:g}, clearing above {clears:g}")


async def main() -> tuple[list[str], bool, int]:
    # Three parties on one broker: the node that reads the bed, the engine that holds
    # the valve, and a watcher on the topic the rule publishes to.
    broker = LoopbackBroker()
    probe = broker.link()
    engine = broker.link()
    watcher = broker.link()
    await probe.connect()
    await engine.connect()
    await watcher.connect()
    await watcher.subscribe("garden/bed-1/valve")
    await engine.subscribe(when["topic"])

    # The condition is the trigger; the actions are the program's own.
    trigger = (
        Trigger.below(when["threshold"], when["hysteresis"])
        if when["compare"] == "below"
        else Trigger.above(when["threshold"], when["hysteresis"])
    )
    valve = {"open": False, "switches": 0}

    async def run(actions: list[dict]) -> None:
        for action in actions:
            if action["do"] == "drive":
                valve["open"] = action["on"]
                valve["switches"] += 1
            else:
                await engine.send(action["topic"], action["payload"])

    # The bed dries out and is watered back: the rule fires once on the way down and
    # once on the way back, and holds its state for the readings in between.
    for reading in [42, 31, 28, 33, 36]:
        await probe.send(when["topic"], str(reading))
        message = await engine.recv()
        edge = trigger.update(message.number)
        if edge == Edge.SET:
            await run(rule["then"])
        elif edge == Edge.CLEARED:
            await run(rule["otherwise"])
        print(f"{reading}: {edge or 'no edge'}, valve {'on' if valve['open'] else 'off'}")

    # The watcher on the other topic heard each edge as the rule published it.
    heard = [(await watcher.recv()).text for _ in range(2)]
    print(f"the watcher heard {', '.join(heard)}")
    print(f"the valve switched {valve['switches']} times")
    return heard, valve["open"], valve["switches"]


heard, valve_open, switches = asyncio.run(main())
# ANCHOR_END: example

assert heard == ["open", "closed"]
assert valve_open is False
assert switches == 2
