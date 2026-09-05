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

It loads a brooder-heater profile from a JSON manifest, runs two temperatures
through the controller that manifest describes, and writes the profile back out
as text.

The manifest sets the three sampling intervals but neither battery threshold, so
the 50% saver figure printed comes from the library's default rather than from
the file. The two readings sit either side of the 32 C target: 27.5 C is below
the deadband and further off target than the 4 C safe band allows, while 32.2 C
is inside both.

It proves:

- A manifest parses into the name, topic, setpoint policy and sampling schedule
  the node runs on, with `cooling` set false marking the output a heater.
- `saver_below` never appears in the manifest and still reads `0.5`, the
  documented default, rather than nothing.
- A reading below the deadband switches the lamp on and raises `OutOfRange`
  carrying the `27.5` that caused it, so what tripped the alert travels with it.
- A reading inside the safe band raises nothing, so an alert tracks the band
  rather than firing on every sample.
- Serializing writes the defaulted threshold out by name, and reloading that
  text gives back an equal profile.

## Rust

<!-- snippet: examples/tests/guides/profile.rs#example -->
From [`examples/tests/guides/profile.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/profile.rs):

```rust
use pamoja_profile::{Alert, Profile};

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
println!("{} reports on {}", profile.name, profile.topic);
println!(
    "wakes every {}s while the battery is healthy",
    profile.power.active_secs
);
println!(
    "saver mode below {:.0}% charge",
    profile.power.saver_below * 100.0
);

// The manifest is the whole control loop. At 27.5 C the reading is below the deadband,
// so the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
let cold = profile.controller().evaluate(27.5);
println!(
    "at 27.5 C: lamp {:?}, alert {:?}",
    cold.actuator, cold.alert
);

// Back inside the deadband the lamp is left as it was, and nothing is raised.
let settled = profile.controller().evaluate(32.2);
println!(
    "at 32.2 C: lamp {:?}, alert {:?}",
    settled.actuator, settled.alert
);

// Serializing writes the defaulted fields out in full, so a profile edited on a device
// and shared back carries no value the next reader has to infer.
let shared = profile.to_json().expect("a serializable profile");
println!(
    "shared form names its defaults: {}",
    shared.contains("saver_below")
);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/profile.ts#example -->
From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
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
console.log(`${profile.name} reports on ${profile.topic}`)
console.log(`wakes every ${profile.power.activeSecs}s while the battery is healthy`)
console.log(`saver mode below ${(profile.power.saverBelow * 100).toFixed(0)}% charge`)

// The manifest is the whole control loop. At 27.5 C the reading is below the deadband, so
// the lamp switches on, and it is more than 4 C from target, so the chicks are cold.
const cold = profile.controller().evaluate(27.5)
console.log(`at 27.5 C: lamp ${cold.actuator}, alert ${cold.alert?.kind}`)

// Back inside the deadband the lamp is left as it was, and nothing is raised.
const settled = profile.controller().evaluate(32.2)
console.log(`at 32.2 C: lamp ${settled.actuator}, alert ${settled.alert}`)

// Serializing writes the defaulted fields out in full, so a profile edited on a device and
// shared back carries no value the next reader has to infer.
const shared = profile.toJson()
console.log(`shared form names its defaults: ${shared.includes('saver_below')}`)
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
Console.WriteLine($"{profile.Name} reports on {profile.Topic}");
Console.WriteLine(
    $"wakes every {profile.Power.ActiveSecs}s while the battery is healthy");
Console.WriteLine($"saver mode below {profile.Power.SaverBelow * 100:F0}% charge");

// The manifest is the whole control loop. At 27.5 C the reading is below the
// deadband, so the lamp switches on, and it is more than 4 C from target, so the
// chicks are cold.
Reaction cold = profile.Controller().Evaluate(27.5f);
Console.WriteLine($"at 27.5 C: lamp {cold.Actuator}, alert {cold.Alert?.Kind}");

// Back inside the deadband the lamp is left as it was, and nothing is raised.
Reaction settled = profile.Controller().Evaluate(32.2f);
Console.WriteLine($"at 32.2 C: lamp {settled.Actuator}, alert {settled.Alert?.Kind}");

// Serializing writes the defaulted fields out in full, so a profile edited on a
// device and shared back carries no value the next reader has to infer.
string shared = profile.ToJson();
Console.WriteLine($"shared form names its defaults: {shared.Contains("saver_below")}");
```
<!-- end -->

## Reference

<!-- table: reference profile -->
- Rust: [`pamoja-profile`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html)
- TypeScript: [`@pamoja/profile`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html)
- Python: [`pamoja.profile`](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html)
- C#: [`Pamoja.Profile`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html)
<!-- end -->
