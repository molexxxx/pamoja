"""The device-profile guide example; see docs/guides/profile.md."""

# ANCHOR: example
from pamoja.profile import ElementSpec, Presentation, Profile, Viz

# A profile is plain data, so a fleet ships one as a file rather than as code. This
# manifest names no battery thresholds, so the documented defaults apply.
manifest = """{
    "name": "brooder-heater",
    "topic": "poultry/brooder/temperature",
    "control": {
        "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
        "cooling": false, "safe_band": 4.0
    },
    "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
}"""
profile = Profile.from_json(manifest)
print(f"profile   {profile.name} reports on {profile.topic}")
print(
    "defaults  the file names no battery thresholds, so saver starts below "
    f"{profile.power.saver_below * 100:.0f}% and critical below "
    f"{profile.power.critical_below * 100:.0f}%"
)

# The schedule becomes a power plan, which says what mode a charge puts the node in and
# how long it waits between samples there, in microseconds.
plan = profile.power_plan()
for charge in [0.8, 0.3, 0.1]:
    print(
        f"battery   at {charge * 100:.0f}% it runs {plan.mode(charge)} "
        f"and samples every {plan.interval_us(charge) // 1_000_000} s"
    )

# One controller runs for the life of the node, because it remembers whether the lamp is
# on. The lamp switches on at 31.5 C or below and off at 32.5 C or above, the setpoint
# less and plus the hysteresis, and in between it stays as it was. A reading more than 4 C
# from the setpoint raises an alert as well.
controller = profile.controller()
lamp = False
for reading in [27.5, 31.8, 32.6, 32.1, 31.4]:
    reaction = controller.evaluate(reading)
    on = reaction.actuator is True
    if on:
        change = "lamp stays on" if lamp else "lamp on"
    else:
        change = "lamp off" if lamp else "lamp stays off"
    alert = f", alert {reaction.alert.kind}" if reaction.alert else ""
    print(f"{f'{reading:g} C':<10}{change}{alert}")
    lamp = on

# Written back out, the manifest names the thresholds the file left to their defaults, so
# the next reader has nothing to infer, and it loads as the same profile.
shared = profile.to_json()
if "saver_below" in shared and Profile.from_json(shared).to_json() == shared:
    print("shared    written back out, it names saver_below and loads as the same profile")

# The manifest also carries how a dashboard draws the node: one element here, the brooder's
# temperature on a thermometer with the band the chicks are safe in.
drawn = profile.with_presentation(
    Presentation(
        [
            ElementSpec(
                "brooder_temperature", "celsius", "Brooder temperature", Viz.THERMOMETER,
                band=(28, 36),
            ),
        ]
    )
)
element = drawn.presentation.elements[0]
low, high = element.band
print(
    f"draws     {element.key} in {element.unit} on a {element.viz}, "
    f"safe from {low:g} to {high:g}"
)
# ANCHOR_END: example

assert lamp
assert profile.power.saver_below == 0.5

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

# ANCHOR: wrong
from pamoja.core import PamojaError

# A probe that fails reports a reading that is not a number. The controller raises it
# rather than going quiet, and the lamp holds its state; what off means for the chicks is
# the node's call.
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
# field, with the one it was probably meant to be, rather than leaving the default in its
# place without a word.
for edited in [
    manifest.replace('"hysteresis": 0.5', '"hysteresis": 0.0'),
    manifest.replace('"saver_secs": 600', '"saver_secs": 60'),
    manifest.replace('"critical_secs": 1800 }', '"critical_secs": 1800, "saver_bellow": 0.3 }'),
]:
    try:
        Profile.from_json(edited)
        print("a manifest no node could run was accepted, which should never happen")
    except PamojaError as error:
        print(f"refused   {error}")

# A kind the library does not ship loads with its parameters, but no built-in controller
# decides it, so asking for one is refused rather than handing back a node that would never
# switch the lamp.
custom = Profile.from_json(manifest.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
try:
    custom.controller()
    print("a custom kind ran without its policy, which should never happen")
except PamojaError as error:
    print(f"refused   {error}")
# ANCHOR_END: wrong
