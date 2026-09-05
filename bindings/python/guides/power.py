"""The power-budget guide example; see docs/guides/power.md."""

# ANCHOR: example
from pamoja.power import DutyCycle, PowerMode, power_plan

# A solar node samples every minute while the charge is healthy, stretches to ten minutes
# to conserve, and to an hour once the battery is nearly flat. Durations cross the binding
# as microseconds.
plan = power_plan(60_000_000, 600_000_000, 3_600_000_000)

# The default thresholds enter saver mode below 50% charge and critical below 20%.
for charge in (0.80, 0.35, 0.12):
    every = plan.interval_us(charge) // 1_000_000
    print(f"at {charge * 100:.0f}% charge: {plan.mode(charge)}, sampling every {every}s")

# A panel that is delivering buys back one mode, so the same flat battery keeps reporting
# on the ten-minute saver cadence while the sun is on it.
charging = plan.mode_while_charging(0.12, True)
print(f"the same flat battery, while charging: {charging}")

# The work is the same two seconds whichever mode the node is in; stretching the cycle is
# what saves the energy. The duty fraction is the proxy for average draw, so the hourly
# cadence costs a sixtieth of what the one-minute cadence does.
awake_us = 2_000_000
healthy = DutyCycle(awake_us, plan.interval_us(0.80) - awake_us)
flat = DutyCycle(awake_us, plan.interval_us(0.12) - awake_us)
print(f"awake {healthy.fraction * 100:.2f}% of the time when healthy")
print(f"awake {flat.fraction * 100:.3f}% of the time when flat")

# Stating the budget as a fraction instead gives the awake time directly.
quarter = DutyCycle.from_fraction(1_000_000, 0.25)
print(f"a quarter-duty second is {quarter.active_us / 1000:.0f}ms awake")
# ANCHOR_END: example

assert plan.mode(0.80) == PowerMode.ACTIVE
assert plan.interval_us(0.80) == 60_000_000
assert plan.mode(0.35) == PowerMode.SAVER
assert plan.mode(0.12) == PowerMode.CRITICAL
assert plan.interval_us(0.12) == 3_600_000_000
assert charging == PowerMode.SAVER
assert abs(healthy.fraction - 2 / 60) < 1e-6
assert abs(flat.fraction - 2 / 3600) < 1e-6
assert quarter.active_us == 250_000
assert quarter.sleep_us == 750_000
