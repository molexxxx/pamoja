"""The helpers guide example; see docs/guides/kit.md."""

# ANCHOR: example
from pamoja.kit import Calibration, Median, Thermostat

# A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is full,
# so the span is 16 mA and mid-scale is 12 mA, not 10.
level = Calibration.two_point(4.0, 0.0, 20.0, 100.0)
print(f"12 mA is {level.apply(12.0)}% full, 4 mA is {level.apply(4.0)}%")

# The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
# scale rather than an empty tank.
broken = level.apply(0.0)
print(f"a dead loop reads {broken}%, which is not a level at all")

# A median window drops that sample outright, where an average would blend a quarter of
# the range into every reading after it.
filtered = Median()
percent = 0.0
for milliamps in (12.0, 12.0, 0.0, 12.0, 12.0):
    percent = level.apply(filtered.update(milliamps))
print(f"through the dropout, the level held at {percent}%")

# A refill pump runs when the level falls below the deadband, which is the direction
# heating names; nothing about it is specific to temperature. The deadband stops a level
# sitting on the threshold from chattering the contactor.
pump = Thermostat.heating(50.0, 10.0)
for reading in (percent, 38.0, 45.0, 62.0):
    running = "on" if pump.update(reading) else "off"
    print(f"at {reading}% the pump is {running}")
# ANCHOR_END: example

assert level.apply(12.0) == 50.0
assert level.apply(4.0) == 0.0
assert broken == -25.0
assert percent == 50.0

again = Thermostat.heating(50.0, 10.0)
assert again.update(50.0) is False
assert again.update(38.0) is True
assert again.update(45.0) is True
assert again.update(62.0) is False
