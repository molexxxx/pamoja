# pamoja-profile

Named, ready-to-run device profiles from plain data or a JSON manifest. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/profile.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html)

## Install

```sh
pip install pamoja-profile
```

```python
from pamoja import profile
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/profile.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/profile.py):

```python
from pamoja.profile import ElementSpec, Presentation, Profile, Viz

# A profile is plain data, so a fleet ships one as a file rather than as code. This
# manifest names no battery thresholds, so the documented defaults apply.
manifest = """{
    "name": "brooder-heater",
    "topic": "poultry/brooder/temperature",
    "control": {
        "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
        "cooling": false, "safe_band": 4.0
    },
    "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
}"""
profile = Profile.from_json(manifest)
print(f"profile   {profile.name} reports on {profile.topic}")
print(
    "defaults  the file names no battery thresholds, so saver starts below "
    f"{profile.power.saver_below * 100:.0f}% and critical below "
    f"{profile.power.critical_below * 100:.0f}%"
)

# The schedule becomes a power plan, which says what mode a charge puts the node in and
# how long it waits between samples there, in microseconds.
plan = profile.power_plan()
for charge in [0.8, 0.3, 0.1]:
    print(
        f"battery   at {charge * 100:.0f}% it runs {plan.mode(charge)} "
        f"and samples every {plan.interval_us(charge) // 1_000_000} s"
    )

# One controller runs for the life of the node, because it remembers whether the lamp is
# on. The lamp switches on at 31.5 C or below and off at 32.5 C or above, the setpoint
# less and plus the hysteresis, and in between it stays as it was. A reading more than 4 C
# from the setpoint raises an alert as well.
controller = profile.controller()
lamp = False
for reading in [27.5, 31.8, 32.6, 32.1, 31.4]:
    reaction = controller.evaluate(reading)
    on = reaction.actuator is True
    if on:
        change = "lamp stays on" if lamp else "lamp on"
    else:
        change = "lamp off" if lamp else "lamp stays off"
    alert = f", alert {reaction.alert.kind}" if reaction.alert else ""
    print(f"{f'{reading:g} C':<10}{change}{alert}")
    lamp = on

# Written back out, the manifest names the thresholds the file left to their defaults, so
# the next reader has nothing to infer, and it loads as the same profile.
shared = profile.to_json()
if "saver_below" in shared and Profile.from_json(shared).to_json() == shared:
    print("shared    written back out, it names saver_below and loads as the same profile")

# The manifest also carries how a dashboard draws the node: one element here, the brooder's
# temperature on a thermometer with the band the chicks are safe in.
drawn = profile.with_presentation(
    Presentation(
        [
            ElementSpec(
                "brooder_temperature", "celsius", "Brooder temperature", Viz.THERMOMETER,
                band=(28, 36),
            ),
        ]
    )
)
element = drawn.presentation.elements[0]
low, high = element.band
print(
    f"draws     {element.key} in {element.unit} on a {element.viz}, "
    f"safe from {low:g} to {high:g}"
)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-profile`](https://crates.io/crates/pamoja-profile) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [docs.rs](https://docs.rs/pamoja-profile), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-profile) |
| TypeScript | [`@pamoja/profile`](https://www.npmjs.com/package/@pamoja/profile) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-profile) |
| Python | [`pamoja-profile`](https://pypi.org/project/pamoja-profile/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-profile) |
| C# | [`Pamoja.Profile`](https://www.nuget.org/packages/Pamoja.Profile) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-profile) |

## Documentation

- [`pamoja.profile` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), every class and function in this module.
- [The Device profiles guide](https://pamoja.molex.cloud/docs/guides/profile.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
