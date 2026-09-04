# pamoja-kit

Plain-language helper math: smoothing, calibration, PID and thermostat control, trend and surge prediction, rolling windows, kinematics, and geo. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-kit`](https://crates.io/crates/pamoja-kit) | [docs.rs](https://docs.rs/pamoja-kit), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html) |
| TypeScript | [`@pamoja/kit`](https://www.npmjs.com/package/@pamoja/kit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html) |
| Python | [`pamoja-kit`](https://pypi.org/project/pamoja-kit/) | [`pamoja.kit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html) |
| C# | [`Pamoja.Kit`](https://www.nuget.org/packages/Pamoja.Kit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.Kit.html) |

## Documentation

- [The Helpers guide](https://pamoja.molex.cloud/docs/guides/kit.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
