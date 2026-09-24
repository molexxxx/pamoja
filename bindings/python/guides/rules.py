"""The rules guide example; see docs/guides/rules.md."""

# ANCHOR: example
import asyncio

from pamoja.loopback import LoopbackBroker
from pamoja.profile import RuleActionKind, RuleEvaluator

# A rule is a file: the topic it watches, the line a reading crosses, the release band that
# stops it firing over and over, and what to do on the way down and on the way back. Two
# rules watch one bed here: one waters it when it dries past 30 and stops once it is wetter
# than 35, and one raises an alarm when it is soaked past 60.
file = """{ "rules": [
  { "name": "water-when-dry",
    "when": { "topic": "garden/bed-1/moisture", "below": 30.0, "hysteresis": 5.0 },
    "then": [ { "drive": "bed-valve", "on": true },
              { "publish": "garden/bed-1/valve", "payload": "open" } ],
    "otherwise": [ { "drive": "bed-valve", "on": false },
                   { "publish": "garden/bed-1/valve", "payload": "closed" } ] },
  { "name": "flood-alarm",
    "when": { "topic": "garden/bed-1/moisture", "above": 60.0, "hysteresis": 5.0 },
    "then": [ { "publish": "garden/alarm", "payload": "waterlogged" } ] }
] }"""

# The evaluator judges each reading and says what the rules call for; the program moves the
# messages and holds the valve, which is the engine's work in Rust.
evaluator = RuleEvaluator.from_json(file)
print(f"watches   {', '.join(evaluator.topics)}, and drives {', '.join(evaluator.actuators)}")


def described(action) -> str:
    """Says what one action does, in the words the output uses."""
    if action.kind == RuleActionKind.DRIVE:
        return f"drive {action.actuator} {'on' if action.on else 'off'}"
    return f"publish {action.payload} to {action.topic}"


async def main() -> tuple[list[str], list[bool]]:
    # Three parties on one broker: the node that reads the bed, the program that holds the
    # valve, and a watcher on the topics the rules publish to.
    broker = LoopbackBroker()
    probe = broker.link()
    link = broker.link()
    watcher = broker.link()
    await probe.connect()
    await link.connect()
    await watcher.connect()
    await watcher.subscribe("garden/bed-1/valve")
    await watcher.subscribe("garden/alarm")
    for topic in evaluator.topics:
        await link.subscribe(topic)

    valve: list[bool] = []

    # The bed dries out, is watered, and floods. A rule fires only as its condition sets or
    # clears, and the readings in between change nothing. At 65 two rules fire on one
    # reading, in the order the file lists them.
    for reading in [42, 31, 28, 33, 65, 50]:
        await probe.send("garden/bed-1/moisture", str(reading))
        message = await link.recv()
        fired = evaluator.evaluate(message.topic, message.number)
        at = f"{reading:<10}"
        if not fired:
            print(f"{at}nothing fired")
        for one in fired:
            for action in one.actions:
                if action.kind == RuleActionKind.DRIVE:
                    valve.append(action.on)
                else:
                    await link.send(action.topic, action.payload)
            if not one.actions:
                print(f"{at}{one.rule} {one.edge}, with nothing to do")
            else:
                print(f"{at}{one.rule} {one.edge}: {', '.join(described(a) for a in one.actions)}")

    # The watcher heard every message the rules published, in the order they went out.
    heard = [(await watcher.recv()).text for _ in range(3)]
    print(f"heard     {', '.join(heard)}")
    print(f"valve     switched {len(valve)} times, and it is {'on' if valve[-1] else 'off'}")
    return heard, valve


heard, valve = asyncio.run(main())
# ANCHOR_END: example

assert heard == ["open", "closed", "waterlogged"]
assert valve == [True, False]
assert evaluator.is_set("water-when-dry") is False

# ANCHOR: wrong
from pamoja.core import PamojaError

# A reading that is not a number, such as the NaN a failed probe reports, is refused on a
# watched topic rather than leaving every rule as it was with nothing to say why.
judge = RuleEvaluator.from_json(file)
try:
    judge.evaluate("garden/bed-1/moisture", float("nan"))
    print("a reading of NaN was judged, which should never happen")
except PamojaError as error:
    print(f"refused   {error}")

# A topic no rule watches is not judged at all, so even a NaN there says nothing.
if not judge.evaluate("garden/bed-2/moisture", float("nan")):
    print("ignored   no rule watches garden/bed-2/moisture, so even a NaN there is not judged")

# A file no engine could run is refused as it loads, with the rule and the reason.
for edited in [
    file.replace("garden/bed-1/moisture", "garden/+/moisture"),
    file.replace('"flood-alarm"', '"water-when-dry"'),
]:
    try:
        RuleEvaluator.from_json(edited)
        print("a file no engine could run was accepted, which should never happen")
    except PamojaError as error:
        print(f"refused   {error}")


def fires(text: str) -> int:
    """Counts how often the rules fire as four readings hover at the line."""
    rules = RuleEvaluator.from_json(text)
    return sum(len(rules.evaluate("garden/bed-1/moisture", r)) for r in [29.9, 30.1, 29.8, 30.2])


# With no release band, readings that hover at the line set and clear the rule on every
# sample, and each edge switches the valve. The band of 5 holds it through them.
bare = fires(file.replace('"hysteresis": 5.0', '"hysteresis": 0.0'))
banded = fires(file)
print(
    f"chatter   4 readings hovering at 30 fire the rule {bare} times with no release band, "
    f"{banded} with a band of 5"
)
# ANCHOR_END: wrong
