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
assert profile.name == "brooder-heater"
assert profile.topic == "poultry/brooder/temperature"
assert profile.control.kind == ControlKind.SETPOINT
assert profile.control.setpoint == 32.0
assert profile.control.cooling is False
assert profile.power.active_secs == 120
assert profile.power.saver_below == 0.5

# The manifest is the whole control loop. At 27.5 C the reading is below the deadband,
# so the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
reaction = profile.controller().evaluate(27.5)
assert reaction.actuator is True
assert reaction.alert.kind == AlertKind.OUT_OF_RANGE
assert reaction.alert.reading == 27.5

# Serializing writes the defaulted fields out in full, so a profile edited on a device
# and shared back carries no value the next reader has to infer.
shared = profile.to_json()
assert '"saver_below"' in shared
assert Profile.from_json(shared).control.setpoint == 32.0
# ANCHOR_END: example
