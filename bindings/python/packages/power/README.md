# pamoja-power

Duty cycling and an energy-aware governor that stretches work as the battery drains. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/power.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
