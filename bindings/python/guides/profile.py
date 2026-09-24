"""The device-profile guide example; see docs/guides/profile.md."""

import asyncio

# ANCHOR: example
from pathlib import Path

from pamoja.loopback import LoopbackBroker
from pamoja.profile import Node, Profile

# A profile is a file. This one ships in the catalog under profiles/: it holds a brooder at
# 32 C by switching a heat lamp, says what it reads, and says how a dashboard draws it.
text = Path("profiles/brooder-heater.json").read_text(encoding="utf-8")
profile = Profile.from_json(text)
print(
    f"profile   {profile.name} reads {profile.reads.quantity} in {profile.reads.unit} "
    f"and reports on {profile.topic}"
)


async def example() -> bool:
    # A node is the profile and the parts that make it run: a sensor, an output, and a link.
    # A morning of readings stands in for the probe, and a dashboard listens on the same
    # broker.
    broker = LoopbackBroker()
    link = broker.link()
    dashboard = broker.link()
    await link.connect()
    await dashboard.connect()
    await dashboard.subscribe(profile.topic)
    morning = [27.5, 31.8, 32.6, 32.1, 31.4]
    lamp = []
    node = Node(profile, read=lambda: morning.pop(0), drive=lamp.append, link=link)

    # Each tick reads, decides, switches the lamp, and publishes the reading. The lamp comes
    # on at 31.5 C or below and goes off at 32.5 C or above, and in between it stays as it
    # was; a reading more than 4 C from 32 raises an alert as well.
    was = False
    for _ in range(5):
        tick = await node.tick()
        on = tick.reaction.actuator is True
        if on:
            change = "lamp stays on" if was else "lamp on"
        else:
            change = "lamp off" if was else "lamp stays off"
        alert = f", alert {tick.reaction.alert.kind}" if tick.reaction.alert else ""
        print(f"{f'{tick.reading:g} C':<10}{change}{alert}")
        was = on

    # The dashboard heard every reading the node published.
    heard = [f"{(await dashboard.recv()).number:g}" for _ in range(5)]
    print(f"heard     {', '.join(heard)} on {profile.topic}")

    # Between ticks the node waits as long as its battery allows: often on a healthy charge,
    # sparingly on a low one. run() does this until stopped, waiting each interval.
    for charge in [0.8, 0.3, 0.1]:
        mode, seconds = node.schedule(charge)
        print(f"battery   at {charge * 100:.0f}% it runs {mode} and waits {seconds:g} s")

    # The same file says how a dashboard draws the node.
    element = profile.presentation.elements[0]
    low, high = element.band
    print(
        f"draws     {element.key} in {element.unit} on a {element.viz}, "
        f"safe from {low:g} to {high:g}"
    )
    return lamp[-1]


lamp_on = asyncio.run(example())
# ANCHOR_END: example

assert lamp_on, "the morning ends with the lamp on"

# ANCHOR: kinds
from pamoja.profile import AlertKind

# A level warns before a tank or a well runs dry. The shipped well profile counts 0.5 m as
# dry and warns once the last fall puts dry six samples away or nearer.
well = Profile.well_level().controller()
for depth in [5.0, 4.4, 3.8]:
    alert = well.evaluate(depth).alert
    if alert and alert.kind == AlertKind.RUNNING_OUT:
        print(f"well      {depth:g} m: dry in {alert.samples} samples at this rate, RunningOut")
    else:
        print(f"well      {depth:g} m: no warning yet")

# A surge warns when a reading moves too far in one sample. The shipped flood sensor warns
# when a river rises more than 0.3 m between two readings.
river = Profile.flood_sensor().controller()
for gauge in [1.2, 1.35, 1.9]:
    alert = river.evaluate(gauge).alert
    if alert and alert.kind == AlertKind.CHANGING_FAST:
        print(f"river     {gauge:g} m: up {alert.rate:.2f} m in one sample, ChangingFast")
    else:
        print(f"river     {gauge:g} m: no warning")
# ANCHOR_END: kinds

# ANCHOR: custom
from pamoja.profile import CustomAlert, Decision, PolicyRegistry


class BrooderGuard:
    """A policy of the program's own: the lamp on below the setpoint, and a condition of its
    own when the chicks are chilled."""

    def __init__(self, params: dict) -> None:
        self.setpoint = params.get("setpoint", 32.0)
        self.chilled_below = self.setpoint - params.get("safe_band", 4.0)

    def evaluate(self, reading: float) -> Decision:
        chilled = reading < self.chilled_below
        alert = CustomAlert("Chilled", reading) if chilled else None
        return Decision(actuator=reading < self.setpoint, alert=alert)


async def custom() -> None:
    # A manifest may name a control kind the library never shipped, with its parameters
    # beside it. The program registers the code that decides it under that name, and the
    # node runs whichever kind the file names.
    guarded = Profile.from_json(text.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
    registry = PolicyRegistry().register("brooder_guard", BrooderGuard)
    print(
        f"custom    {guarded.control.custom_kind} is decided by the program's own code, "
        "registered under its name"
    )
    link = LoopbackBroker().link()
    await link.connect()
    lamp = []
    node = Node(guarded, read=lambda: 27.5, drive=lamp.append, link=link, policy=registry)
    tick = await node.tick()
    alert = tick.reaction.alert.code if tick.reaction.alert else "none"
    print(f"{f'{tick.reading:g} C':<10}lamp {'on' if lamp[-1] else 'off'}, alert {alert}")


asyncio.run(custom())
# ANCHOR_END: custom

# ANCHOR: wrong
from pamoja.core import PamojaError

# A probe that fails reports a reading that is not a number. The controller raises it
# rather than going quiet, and the lamp holds its state; what off means for the chicks is
# the node's call.
controller = profile.controller()
controller.evaluate(27.5)
failed = controller.evaluate(float("nan"))
if failed.alert:
    holds = "on" if failed.actuator is True else "off"
    print(f"probe     a reading of NaN raises {failed.alert.kind}, and the lamp holds {holds}")

# A controller built again for each reading forgets the lamp was on, so inside the deadband
# it switches the lamp off.
first = profile.controller().evaluate(27.5).actuator
then = profile.controller().evaluate(31.8).actuator
if first is True and then is False:
    print("fresh     built again for each reading, the controller turns the lamp off at 31.8 C")

# A manifest no node could run is refused as it loads, with the reason. So is a misspelled
# field, with the one it was probably meant to be and where it sits, rather than leaving the
# default in its place without a word.
for edited in [
    text.replace('"hysteresis": 0.5', '"hysteresis": 0.0'),
    text.replace('"saver_secs": 600', '"saver_secs": 60'),
    text.replace('"saver_below"', '"saver_bellow"'),
]:
    try:
        Profile.from_json(edited)
        print("a manifest no node could run was accepted, which should never happen")
    except PamojaError as error:
        print(f"refused   {error}")

# A kind the library does not ship loads with its parameters, but no built-in controller
# decides it, so a node without a registry that knows it is refused rather than running one
# that never switches the lamp.
unknown = Profile.from_json(text.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
try:
    unknown.controller()
    print("a custom kind ran without its policy, which should never happen")
except PamojaError as error:
    print(f"refused   {error}")
# ANCHOR_END: wrong
