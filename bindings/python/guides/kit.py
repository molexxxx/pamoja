"""The helpers guide example; see docs/guides/kit.md."""

# ANCHOR: example
import math

from pamoja.kit import (
    Anomaly,
    Calibration,
    Coordinate,
    Debounce,
    Depletion,
    Edge,
    Geofence,
    Kalman,
    Median,
    Pid,
    Ramp,
    Smoother,
    Surge,
    Thermostat,
    Trend,
    Trigger,
    Window,
    bearing_between,
    deadband,
    distance_between,
)

# The tower's level transmitter reports on a 4-20 mA loop: 4 mA is empty and 20 mA is
# full, so mid-scale is 12 mA, and a dead loop reads below empty rather than as empty.
level = Calibration.two_point(4.0, 0.0, 20.0, 100.0)
mid, empty, dead = level.apply(12.0), level.apply(4.0), level.apply(0.0)
print(f"level     12 mA reads {mid:.1f}%, 4 mA reads {empty:.1f}%, a dead loop {dead:.1f}%")

# One dropout in five readings: the median of the five ignores it, the mean does not.
median = Median(5)
recent = Window(5)
held = 0.0
for milliamps in (12.0, 12.0, 0.0, 12.0, 12.0):
    held = median.update(milliamps)
    recent.push(milliamps)
held_percent, mean_percent = level.apply(held), level.apply(recent.mean())
print(
    f"level     through a dropout the median holds {held_percent:.1f}%, "
    f"the mean falls to {mean_percent:.1f}%"
)

# Water sloshing in the tower swings the reading. A smoother moves a quarter of the way
# from its last value toward each new reading, so the swing mostly cancels out.
smoother = Smoother(0.25)
swing = Window(6)
for percent in (50.0, 53.0, 48.0, 52.0, 49.0, 51.0):
    smoother.update(percent)
    swing.push(percent)
print(
    f"level     sloshing readings from {swing.min():.1f}% to {swing.max():.1f}% "
    f"smooth to {smoother.value:.1f}%"
)

# A Kalman filter is told how noisy the sensor is and how fast the level can really move.
# When the pump starts and the level climbs from 50% to 60%, the one told the level barely
# moves takes the climb for noise and lags; the one told it moves keeps up.
expects_steady = Kalman(0.01, 2.0, 50.0)
expects_motion = Kalman(0.5, 2.0, 50.0)
for percent in (50.0, 50.0, 50.0, 60.0, 60.0, 60.0, 60.0):
    expects_steady.update(percent)
    expects_motion.update(percent)
slow, fast = expects_steady.estimate, expects_motion.estimate
print(
    f"level     four readings into a rise to 60%, a Kalman filter expecting a steady level "
    f"reads {slow:.1f}%, one expecting motion {fast:.1f}%"
)

# The refill pump starts at 40% and stops at 60%: on/off control with a band either side
# of 50. Starting when the level falls is the direction heating names.
pump = Thermostat.heating(50.0, 10.0)
states = []
for percent in (50.0, 39.0, 45.0, 61.0):
    running = "on" if pump.update(percent) else "off"
    states.append(f"{percent:.0f}% {running}")
print(f"pump      {', '.join(states)}")

# The high-level float switch bounces as the water sloshes at the top. It has to read full
# three times running before the pump controller believes it.
float_switch = Debounce(3, False)
raw_changes, settled_changes, last_raw = 0, 0, False
for raw in (True, False, True, True, True, False, True):
    raw_changes += raw != last_raw
    last_raw = raw
    before = float_switch.state
    settled_changes += float_switch.update(raw) != before
full = "full" if float_switch.state else "not full"
print(
    f"float     {raw_changes} raw changes settled into {settled_changes}: "
    f"the tower reads {full}"
)

# The low-water alarm is sent once when the level drops under 20% and not again until it
# has come back above 25%, however long it hovers near the line.
low_water = Trigger.below(20.0, 5.0)
for percent in (24.0, 19.0, 18.0, 21.0, 19.0, 26.0):
    edge = low_water.update(percent)
    if edge is Edge.SET:
        print(f"alarm     low water at {percent:.0f}%: alarm sent")
    elif edge is Edge.CLEARED:
        print(f"alarm     back to {percent:.0f}%: all clear sent")

# A booster pump holds the mains at 3.0 bar. The PID asks for a speed, the ramp lets the
# pump change by at most 25% a second so the pipes never take a water hammer, and within
# 0.05 bar of 3.0 the reading counts as on target.
pressure_hold = Pid(40.0, 8.0, 0.0, min=0.0, max=100.0)
soft_start = Ramp(0.0, 25.0)
for bar in (1.0, 1.8, 2.5, 2.9, 3.02):
    steady = deadband(bar, 3.0, 0.05)
    asked = pressure_hold.update(3.0, steady, 1.0)
    given = soft_start.update(asked)
    print(
        f"booster   at {bar:.2f} bar the PID asks for {asked:.0f}%, "
        f"the pump is given {given:.0f}%"
    )

# A power cut stops the borehole pump. From the hourly level, the countdown says how long
# until the tower reaches its 20% reserve.
reserve = Depletion(20.0)
hours_left = None
for percent in (80.0, 76.0, 72.0):
    hours_left = reserve.update(percent)
print(f"outage    at the rate it is falling, the tower reaches 20% in {hours_left} hours")

# With the outlet shut overnight the level should hold. A steady fall is a leak.
overnight = Trend(6)
for percent in (78.0, 77.6, 77.1, 76.7, 76.2, 75.8):
    overnight.push(percent)
slope = overnight.slope
print(f"leak      with the outlet shut the level falls {-slope:.2f}% an hour")

# A burst main shows as pressure falling faster than any demand could pull it.
burst = Surge.falling(0.5)
for bar in (3.0, 2.9, 1.7):
    fall = burst.update(bar)
    if fall is not None:
        print(f"burst     the pressure fell {fall:.1f} bar in one reading")

# The flow meter's readings set their own baseline. A hydrant opened stands out, and so
# does a reading the meter could not make.
flow = Anomaly(3.0, 8)
normal = Window(8)
flagged = 0
for cubic_meters in (12.1, 11.8, 12.4, 12.0, 11.9, 12.2, 12.0, 12.3):
    flagged += flow.check(cubic_meters)
    normal.push(cubic_meters)
print(
    f"meter     {len(normal)} readings from {normal.min():.1f} to {normal.max():.1f} m3/h, "
    f"{flagged} flagged"
)


def verdict(stands_out: bool) -> str:
    return "stands out" if stands_out else "passes"


hydrant = flow.check(30.5)
failed = flow.check(math.nan)
print(f"meter     a reading of 30.5 m3/h {verdict(hydrant)}; a failed reading {verdict(failed)}")

# The tanker truck delivers inside a 20 km district around its depot.
depot = Coordinate(-1.5177, 37.2634)
village = Coordinate(-1.4480, 37.3390)
km = distance_between(depot, village) / 1000.0
bearing = bearing_between(depot, village)
print(f"truck     the village is {km:.1f} km from the depot, bearing {bearing:.0f} degrees")
district = Geofence(depot, 20_000.0)
road_out = Coordinate(-1.3000, 37.4500)
further = Coordinate(-1.2500, 37.5000)
route = (depot, village, road_out, further, village)
crossings = [district.update(fix).value.lower() for fix in route]
print(f"truck     {', '.join(crossings)}")
# ANCHOR_END: example

assert (mid, empty, dead) == (50.0, 0.0, -25.0)
assert held_percent == 50.0
assert abs(mean_percent - 35.0) < 1e-3
assert states == ["50% off", "39% on", "45% on", "61% off"]
assert (raw_changes, settled_changes) == (5, 1)
assert float_switch.state is True
assert hours_left == 13
assert abs(slope + 0.4457) < 1e-3
assert slow < 56.0 and fast > 58.0
assert hydrant and failed
assert flagged == 0
assert crossings == ["inside", "inside", "exited", "outside", "entered"]
