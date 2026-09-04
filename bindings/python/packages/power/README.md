# pamoja-power

Duty cycling and an energy-aware governor that stretches work as the battery drains. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-power`](https://crates.io/crates/pamoja-power) | [docs.rs](https://docs.rs/pamoja-power), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html) |
| TypeScript | [`@pamoja/power`](https://www.npmjs.com/package/@pamoja/power) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html) |
| Python | [`pamoja-power`](https://pypi.org/project/pamoja-power/) | [`pamoja.power`](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html) |
| C# | [`Pamoja.Power`](https://www.nuget.org/packages/Pamoja.Power) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.DutyCycle.html) |

## Documentation

- [The Power guide](https://pamoja.molex.cloud/docs/guides/power.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
