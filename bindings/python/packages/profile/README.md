# pamoja-profile

Named, ready-to-run device profiles from plain data or a JSON manifest. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/profile.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
from pamoja.profile import AlertKind, ControlKind, Profile

# A profile is plain data, so a fleet ships one as a file rather than as code. The two
# power thresholds are optional and fall back to the documented defaults.
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
print(f"{profile.name} reports on {profile.topic}")
print(f"wakes every {profile.power.active_secs}s while the battery is healthy")
print(f"saver mode below {profile.power.saver_below * 100:.0f}% charge")

# The manifest is the whole control loop. At 27.5 C the reading is below the deadband, so
# the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
cold = profile.controller().evaluate(27.5)
print(f"at 27.5 C: lamp {cold.actuator}, alert {cold.alert.kind if cold.alert else None}")

# Back inside the deadband the lamp is left as it was, and nothing is raised.
settled = profile.controller().evaluate(32.2)
print(f"at 32.2 C: lamp {settled.actuator}, alert {settled.alert}")

# Serializing writes the defaulted fields out in full, so a profile edited on a device and
# shared back carries no value the next reader has to infer.
shared = profile.to_json()
print(f"shared form names its defaults: {'saver_below' in shared}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-profile`](https://crates.io/crates/pamoja-profile) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [docs.rs](https://docs.rs/pamoja-profile) |
| TypeScript | [`@pamoja/profile`](https://www.npmjs.com/package/@pamoja/profile) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html) |
| Python | [`pamoja-profile`](https://pypi.org/project/pamoja-profile/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html) |
| C# | [`Pamoja.Profile`](https://www.nuget.org/packages/Pamoja.Profile) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html) |

## Documentation

- [`pamoja.profile` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), every class and function in this module.
- [The Device profiles guide](https://pamoja.molex.cloud/docs/guides/profile.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
