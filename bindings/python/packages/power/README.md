# pamoja-power

Duty cycling and an energy-aware governor that stretches work as the battery drains and holds its mode against a wandering charge. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/power.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html)

## Install

```sh
pip install pamoja-power
```

```python
from pamoja import power
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/power.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/power.py):

```python
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

# A fuel gauge wanders a point or two between readings, so a charge sitting at a threshold
# would change the cadence on every cycle. `next_mode` takes the mode the node is in: it
# drops as soon as the charge falls below a threshold, and climbs back only once the charge
# is the plan's hysteresis margin clear of it.
wandering = (0.49, 0.51, 0.50, 0.53, 0.48, 0.52)


def walk(governor):
    mode = PowerMode.ACTIVE
    modes = []
    for charge in wandering:
        mode = governor.next_mode(mode, charge)
        modes.append(mode)
    return ", ".join(modes)


flapping = walk(plan.with_hysteresis(0))
print(f"a charge wandering around 50% with no margin: {flapping}")
margin = plan.hysteresis * 100
settled = walk(plan)
print(f"and with the {margin:.0f} point margin: {settled}")
back = plan.next_mode(PowerMode.SAVER, 0.56)
print(f"at 56% the charge has cleared the margin: {back}")

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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-power`](https://crates.io/crates/pamoja-power) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html), [docs.rs](https://docs.rs/pamoja-power), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-power) |
| TypeScript | [`@pamoja/power`](https://www.npmjs.com/package/@pamoja/power) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-power) |
| Python | [`pamoja-power`](https://pypi.org/project/pamoja-power/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-power) |
| C# | [`Pamoja.Power`](https://www.nuget.org/packages/Pamoja.Power) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-power) |

## Documentation

- [`pamoja.power` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html), every class and function in this module.
- [The Power guide](https://pamoja.molex.cloud/docs/guides/power.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
