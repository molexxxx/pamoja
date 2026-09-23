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

# A panel that is delivering buys back one mode. The interval for a charge knows nothing of
# the panel, so the cadence comes from the mode the panel bought.
charging = plan.mode_while_charging(0.12, True)
every = plan.interval_for_us(charging) // 1_000_000
print(f"at 12% charge while charging: {charging}, sampling every {every}s")

# A charge worked out from a fuel gauge that did not answer is not a number. The plan takes
# it as critical, so a node that cannot tell what it has left does the least until it can.
unknown = float("nan")
every = plan.interval_us(unknown) // 1_000_000
print(f"with no reading from the gauge: {plan.mode(unknown)}, sampling every {every}s")

# The thresholds say how long the battery must carry the node without sun. Winter nights
# are long, so a winter plan starts saving sooner and goes critical sooner.
winter = plan.with_thresholds(0.70, 0.30)
saver, critical = winter.saver_below * 100, winter.critical_below * 100
print(f"the winter plan saves below {saver:.0f}% and goes critical below {critical:.0f}%")
cold, mild = winter.mode(0.60), plan.mode(0.60)
print(f"at 60% charge: {cold} in winter, {mild} by default")

# The work is the same two seconds whichever mode the node is in; stretching the cycle is
# what saves the energy. The duty fraction is the proxy for average draw, so the hourly
# cadence costs a sixtieth of what the one-minute cadence does.
awake_us = 2_000_000
healthy = DutyCycle(awake_us, plan.interval_us(0.80) - awake_us)
flat = DutyCycle(awake_us, plan.interval_us(0.12) - awake_us)
print(f"awake {healthy.fraction * 100:.2f}% of the time when healthy")
print(f"awake {flat.fraction * 100:.3f}% of the time when flat")

# A node that lives on its panel can stay awake for the share of the time the harvest pays
# for. Asleep it draws next to nothing, so that share is the harvest over what it draws
# awake, and the duty cycle turns it into time.
minute_us = 60_000_000
awake_mw = 120
cloudy = DutyCycle.from_fraction(minute_us, 12 / awake_mw)
print(f"a 12 mW harvest pays for {cloudy.active_us // 1000}ms awake in each minute")

# The share is clamped, so a harvest above the draw keeps the node awake throughout, and a
# harvest the meter could not read keeps it asleep until one can be.
sunny = DutyCycle.from_fraction(minute_us, 150 / awake_mw)
unread = DutyCycle.from_fraction(minute_us, float("nan"))
print(f"a 150 mW harvest keeps it awake all {sunny.active_us // 1000}ms")
print(f"an unread harvest keeps it asleep all {unread.sleep_us // 1000}ms")
# ANCHOR_END: example

assert plan.mode(0.80) == PowerMode.ACTIVE
assert plan.interval_us(0.80) == 60_000_000
assert plan.mode(0.35) == PowerMode.SAVER
assert plan.mode(0.12) == PowerMode.CRITICAL
assert plan.interval_us(0.12) == 3_600_000_000
assert charging == PowerMode.SAVER
assert plan.interval_for_us(charging) == 600_000_000
assert plan.mode(unknown) == PowerMode.CRITICAL
assert plan.interval_us(unknown) == 3_600_000_000
assert cold == PowerMode.SAVER
assert mild == PowerMode.ACTIVE
assert abs(healthy.fraction - 2 / 60) < 1e-6
assert abs(flat.fraction - 2 / 3600) < 1e-6
assert cloudy.active_us == 6_000_000
assert cloudy.period_us == minute_us
assert sunny.active_us == minute_us
assert sunny.sleep_us == 0
assert unread.active_us == 0
assert unread.sleep_us == minute_us
