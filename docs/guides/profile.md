# Device profiles

Most deployed nodes are one of a handful of shapes. Something is held near a
setpoint. Something is watched as it falls toward empty. Something is watched for
a change too fast to be real. A profile is that shape written down: what the node
publishes on, which policy it applies to each reading, and how often it samples as
its battery drains.

Writing it down as data rather than as code is what makes a fleet manageable. A
profile loads from and saves to JSON, so retuning a deadband across a hundred
nodes is a file that ships, not a firmware build. The same manifest also carries
what a dashboard needs to draw the node, so the device and the screen agree
without a second source.

The controller is the decision half. Hand it a reading and it says what the output
should do and whether the reading crossed a threshold worth raising. Serializing
writes the defaulted fields out in full, so a manifest that has been round-tripped
leaves nothing for the next reader to infer.

## What the example does

It loads a heater profile from a JSON manifest that leaves the two power
thresholds out, feeds one cold reading to the controller the manifest describes,
and writes the profile back out.

It proves:

- A manifest parses into the name, topic, control policy, and power schedule the
  node runs on.
- Fields the manifest omits come back as the documented defaults rather than as
  nothing.
- The controller the manifest describes switches the output and raises the alert
  the policy calls for, carrying the reading that caused it.
- Serializing writes the defaults out in full, and the result reloads to the same
  profile.

## Rust

<!-- snippet: examples/tests/guides/profile.rs#example -->
From [`examples/tests/guides/profile.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/profile.rs):

```rust
use pamoja_profile::{Alert, ControlSpec, Profile};

// A profile is plain data, so a fleet ships one as a file rather than as code. The two
// power thresholds are optional and fall back to the documented defaults.
let manifest = r#"{
    "name": "brooder-heater",
    "topic": "poultry/brooder/temperature",
    "control": {
        "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
        "cooling": false, "safe_band": 4.0
    },
    "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
}"#;

let profile = Profile::from_json(manifest).expect("a well-formed manifest");
assert_eq!(profile.name, "brooder-heater");
assert_eq!(profile.topic, "poultry/brooder/temperature");
assert_eq!(
    profile.control,
    ControlSpec::Setpoint {
        setpoint: 32.0,
        hysteresis: 0.5,
        cooling: false,
        safe_band: 4.0
    }
);
assert_eq!(profile.power.active_secs, 120);
assert_eq!(profile.power.saver_below, 0.5);

// The manifest is the whole control loop. At 27.5 C the reading is below the deadband,
// so the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
let reaction = profile.controller().evaluate(27.5);
assert_eq!(reaction.actuator, Some(true));
assert_eq!(reaction.alert, Some(Alert::OutOfRange { reading: 27.5 }));

// Serializing writes the defaulted fields out in full, so a profile edited on a device
// and shared back carries no value the next reader has to infer.
let shared = profile.to_json().expect("a serializable profile");
assert!(shared.contains("\"saver_below\""));
assert_eq!(Profile::from_json(&shared).expect("valid JSON"), profile);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/profile.ts#example -->
From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
import assert from 'node:assert/strict'

import { AlertKind, ControlKind, Profile } from '@pamoja/profile'

// A profile is plain data, so a fleet ships one as a file rather than as code. The two
// power thresholds are optional and fall back to the documented defaults.
const manifest = `{
    "name": "brooder-heater",
    "topic": "poultry/brooder/temperature",
    "control": {
        "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
        "cooling": false, "safe_band": 4.0
    },
    "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
}`

const profile = Profile.fromJson(manifest)
assert.equal(profile.name, 'brooder-heater')
assert.equal(profile.topic, 'poultry/brooder/temperature')
assert.equal(profile.control.kind, ControlKind.Setpoint)
assert.equal(profile.control.setpoint, 32.0)
assert.equal(profile.control.cooling, false)
assert.equal(profile.power.activeSecs, 120)
assert.equal(profile.power.saverBelow, 0.5)

// The manifest is the whole control loop. At 27.5 C the reading is below the deadband, so
// the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
const reaction = profile.controller().evaluate(27.5)
assert.equal(reaction.actuator, true)
assert.equal(reaction.alert?.kind, AlertKind.OutOfRange)
assert.equal(reaction.alert?.reading, 27.5)

// Serializing writes the defaulted fields out in full, so a profile edited on a device and
// shared back carries no value the next reader has to infer.
const shared = profile.toJson()
assert.ok(shared.includes('"saver_below"'))
assert.equal(Profile.fromJson(shared).control.setpoint, 32.0)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/profile.py#example -->
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
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs):

```csharp
// A profile is plain data, so a fleet ships one as a file rather than as code. The
// two power thresholds are optional and fall back to the documented defaults.
const string manifest = """
    {
        "name": "brooder-heater",
        "topic": "poultry/brooder/temperature",
        "control": {
            "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
            "cooling": false, "safe_band": 4.0
        },
        "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
    }
    """;

using var profile = Profile.FromJson(manifest);
Expect(profile.Name == "brooder-heater", "the manifest names the profile");
Expect(profile.Topic == "poultry/brooder/temperature", "and the topic it publishes on");
Expect(profile.Control.Kind == ControlKind.Setpoint, "the control policy is a setpoint");
Expect(profile.Control.Setpoint == 32.0f, "held at 32 C");
Expect(profile.Control.Cooling == false, "by heating rather than cooling");
Expect(profile.Power.ActiveSecs == 120, "and it samples every two minutes at charge");
Expect(profile.Power.SaverBelow == 0.5f, "with the default saver threshold filled in");

// The manifest is the whole control loop. At 27.5 C the reading is below the
// deadband, so the lamp switches on, and it is more than 4 C from target, so the
// chicks are cold.
using var controller = profile.Controller();
var reaction = controller.Evaluate(27.5f);
Expect(reaction.Actuator == true, "the lamp is switched on");
Expect(reaction.Alert?.Kind == AlertKind.OutOfRange, "and the drift is reported");
Expect(reaction.Alert?.Reading == 27.5f, "carrying the reading that raised it");

// Serializing writes the defaulted fields out in full, so a profile edited on a
// device and shared back carries no value the next reader has to infer.
var shared = profile.ToJson();
Expect(shared.Contains("\"saver_below\"", StringComparison.Ordinal), "defaults are written out");
using var reloaded = Profile.FromJson(shared);
Expect(reloaded.Control.Setpoint == 32.0f, "and it reloads to the same profile");
```
<!-- end -->

## Reference

<!-- table: reference profile -->
- Rust: [`pamoja-profile`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html)
- TypeScript: [`@pamoja/profile`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html)
- Python: [`pamoja.profile`](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html)
- C#: [`Pamoja.Profile`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html)
<!-- end -->
