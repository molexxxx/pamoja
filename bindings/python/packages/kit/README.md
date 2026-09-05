# pamoja-kit

Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/kit.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-kit
```

```python
from pamoja import kit
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/kit.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/kit.py):

```python
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-kit`](https://crates.io/crates/pamoja-kit) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html), [docs.rs](https://docs.rs/pamoja-kit) |
| TypeScript | [`@pamoja/kit`](https://www.npmjs.com/package/@pamoja/kit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html) |
| Python | [`pamoja-kit`](https://pypi.org/project/pamoja-kit/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html) |
| C# | [`Pamoja.Kit`](https://www.nuget.org/packages/Pamoja.Kit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html) |

## Documentation

- [`pamoja.kit` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html), every class and function in this module.
- [The Helpers guide](https://pamoja.molex.cloud/docs/guides/kit.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
