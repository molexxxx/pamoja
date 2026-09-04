"""The power-scheduling guide example; see docs/guides/power.md."""

# ANCHOR: example
from pamoja.power import DutyCycle, PowerMode, power_plan

# A solar node samples every minute while the charge is healthy, stretches to ten
# minutes to conserve, and to an hour once the battery is nearly flat. Durations cross
# the binding as microseconds.
plan = power_plan(60_000_000, 600_000_000, 3_600_000_000)

# The default thresholds enter saver mode below 50% charge and critical below 20%.
assert plan.mode(0.80) == PowerMode.ACTIVE
assert plan.interval_us(0.80) == 60_000_000
assert plan.mode(0.35) == PowerMode.SAVER
assert plan.interval_us(0.35) == 600_000_000
assert plan.mode(0.12) == PowerMode.CRITICAL
assert plan.interval_us(0.12) == 3_600_000_000

# A panel that is delivering buys back one mode, so the same flat battery keeps
# reporting on the ten-minute saver cadence while the sun is on it.
assert plan.mode_while_charging(0.12, True) == PowerMode.SAVER

# The work is the same two seconds whichever mode the node is in; stretching the cycle
# is what saves the energy. The duty fraction is the proxy for average draw, so the
# hourly cadence costs a sixtieth of what the one-minute cadence does.
awake_us = 2_000_000
healthy = DutyCycle(awake_us, plan.interval_us(0.80) - awake_us)
flat = DutyCycle(awake_us, plan.interval_us(0.12) - awake_us)
assert abs(healthy.fraction - 2 / 60) < 1e-6
assert abs(flat.fraction - 2 / 3600) < 1e-6

# Stating the budget as a fraction instead gives the awake time directly.
quarter = DutyCycle.from_fraction(1_000_000, 0.25)
assert quarter.active_us == 250_000
assert quarter.sleep_us == 750_000
# ANCHOR_END: example
