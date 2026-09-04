"""The helper-math guide example; see docs/guides/kit.md."""

# ANCHOR: example
from pamoja.kit import Calibration, Median, Thermostat

# A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is
# full, so the span is 16 mA and mid-scale is 12 mA, not 10.
level = Calibration.two_point(4.0, 0.0, 20.0, 100.0)
assert level.apply(12.0) == 50.0
assert level.apply(4.0) == 0.0

# The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
# scale rather than an empty tank. A median window drops that sample outright, where an
# average would blend a quarter of the range into every reading after it.
assert level.apply(0.0) == -25.0
filtered = Median()
percent = 0.0
for milliamps in (12.0, 12.0, 0.0, 12.0, 12.0):
    percent = level.apply(filtered.update(milliamps))
    assert percent == 50.0

# A refill pump runs when the level falls below the deadband, which is the direction
# heating names; nothing about it is specific to temperature. The deadband stops a level
# sitting on the threshold from chattering the contactor.
pump = Thermostat.heating(50.0, 10.0)
assert pump.update(percent) is False
assert pump.update(38.0) is True
assert pump.update(45.0) is True
assert pump.update(62.0) is False
# ANCHOR_END: example
