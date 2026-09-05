"""The device-profile guide example; see docs/guides/profile.md."""

# ANCHOR: example
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
# ANCHOR_END: example

assert profile.control.kind == ControlKind.SETPOINT
assert profile.control.setpoint == 32.0
assert profile.control.cooling is False
assert profile.power.active_secs == 120
assert profile.power.saver_below == 0.5
assert cold.actuator is True
assert cold.alert.kind == AlertKind.OUT_OF_RANGE
assert settled.alert is None
assert "saver_below" in shared
assert Profile.from_json(shared).name == profile.name
